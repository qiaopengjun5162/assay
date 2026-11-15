//! 解释层：模型写人话，数字必须过护栏。
//!
//! 位置约定：模型只看事实字典，拿不到原始数据；输出里的每个数字都要能在事实里找到，
//! 不合格重写一次，仍不合格标记 `rejected`，页面上不展示。

use crate::aggregate::{lag_days, load_catalog};
use crate::config::MODEL;
use crate::guard::{check, dataset_facts, extract};
use crate::model::{Dataset, Explanation};
use crate::util::now_iso8601;
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

const SYSTEM: &str = "你是数据质量报告的解释员，写给要用这份数据的人看。规则：\
1) 只能依据给定事实，不得引入外部知识，不推测原因；\
2) 不得出现事实之外的数字；全文最多引用 3 个数字，能不用就不用；\
3) 不要罗列事实，不要复述字段清单；\
4) 固定三句：第一句说最要紧的问题，第二句说这对使用者意味着什么，第三句给一个可执行的建议；\
5) 不用 markdown，不用『我』，不超过 120 字。";

pub struct ExplainOptions<'a> {
    pub items_path: &'a str,
    pub catalog_path: &'a str,
    pub out_dir: &'a str,
    pub top: usize,
    pub threads: usize,
}

pub struct ExplainOutcome {
    pub passed: usize,
    pub total: usize,
    pub elapsed_secs: f64,
}

pub fn explain(opts: &ExplainOptions<'_>) -> Result<ExplainOutcome> {
    let key = crate::config::llm_key()
        .ok_or_else(|| Error::Other("找不到 LLM key：设置 DATAPROOF_LLM_KEY 或本地配置".into()))?;
    let mut items = crate::audit::read_items(opts.items_path)?;
    items.retain(|d| d.has_preview == 1);
    items.sort_by(|a, b| {
        crate::scoring::score(b)
            .partial_cmp(&crate::scoring::score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items.truncate(opts.top);

    let catalog = load_catalog(opts.catalog_path)?;
    let lags: HashMap<String, i64> = catalog
        .iter()
        .filter_map(|r| {
            r.update_time
                .as_deref()
                .and_then(lag_days)
                .map(|d| (r.id.clone(), d))
        })
        .collect();

    let cursor = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Explanation>>> = Mutex::new(vec![None; items.len()]);
    let t0 = Instant::now();

    std::thread::scope(|s| {
        for _ in 0..opts.threads.max(1) {
            s.spawn(|| loop {
                let i = cursor.fetch_add(1, Ordering::SeqCst);
                if i >= items.len() {
                    break;
                }
                let d = &items[i];
                let dept = d
                    .dept
                    .as_deref()
                    .unwrap_or("")
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .to_string();
                let facts = dataset_facts(d, lags.get(&d.id).copied(), &dept);
                let rec = run_one(d, &facts, &key);
                results.lock().unwrap()[i] = Some(rec);
            });
        }
    });

    let list: Vec<Explanation> = results
        .into_inner()
        .unwrap()
        .into_iter()
        .flatten()
        .collect();
    let passed = list.iter().filter(|e| e.status == "pass").count();
    let index = json!({
        "generatedAt": now_iso8601(),
        "model": MODEL,
        "guard": "输出中的每个数字必须出现在事实里；最多 3 个数字；不合格重写一次，仍不合格标记 rejected",
        "total": list.len(),
        "passed": passed,
        "elapsedSec": round1(t0.elapsed().as_secs_f64()),
        "items": list,
    });
    fs::create_dir_all(opts.out_dir)?;
    fs::write(
        Path::new(opts.out_dir).join("explanations.json"),
        serde_json::to_string_pretty(&index)?,
    )?;
    Ok(ExplainOutcome {
        passed,
        total: index["items"].as_array().map(|a| a.len()).unwrap_or(0),
        elapsed_secs: t0.elapsed().as_secs_f64(),
    })
}

fn run_one(d: &Dataset, facts: &[(&'static str, String)], key: &str) -> Explanation {
    let mut last_text = String::new();
    let mut last_illegal: Vec<String> = Vec::new();
    for attempt in 1..=2 {
        let hint = if attempt == 1 {
            "请写 3 句解释。".to_string()
        } else {
            "上一版不合格（数字越界、引用过多或被截断），请重写：只写三句完整的中文，不超过 120 字，数字最多 3 个，结尾必须是句号。".to_string()
        };
        let text = match call_model(facts, &hint, key) {
            Ok(t) => t,
            Err(e) => {
                return Explanation {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    status: "error".into(),
                    attempts: attempt,
                    illegal: vec![e.to_string()],
                    ..Default::default()
                }
            }
        };
        let complete = text.trim_end().ends_with(['。', '！', '？', '”']);
        if !complete || text.trim().chars().count() < 15 {
            eprintln!(
                "  CUT   {} 结尾不完整或过短（第 {attempt} 次）",
                short(&d.name)
            );
            last_text = text;
            continue;
        }
        let (ok, bad) = check(&text, facts, 3);
        if ok {
            println!("  PASS {}  {} 字", short(&d.name), text.chars().count());
            let numbers: Vec<String> = extract(&text).into_iter().collect();
            return Explanation {
                id: d.id.clone(),
                name: d.name.clone(),
                text,
                status: "pass".into(),
                attempts: attempt,
                numbers,
                illegal: Vec::new(),
            };
        }
        eprintln!(
            "  FAIL {}  越界数字 {bad:?}（第 {attempt} 次）",
            short(&d.name)
        );
        last_illegal = bad;
        last_text = text;
    }
    Explanation {
        id: d.id.clone(),
        name: d.name.clone(),
        text: last_text,
        status: "rejected".into(),
        attempts: 2,
        numbers: Vec::new(),
        illegal: last_illegal,
    }
}

fn call_model(facts: &[(&'static str, String)], hint: &str, key: &str) -> Result<String> {
    let facts_json = crate::guard::facts_text(facts);
    let body = json!({
        "model": MODEL,
        "max_tokens": 900,
        "system": SYSTEM,
        "messages": [{"role": "user", "content": [{
            "type": "text",
            "text": format!("事实：\n{facts_json}\n\n{hint}")
        }]}],
    });
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(90))
        .build();
    let resp: Value = agent
        .post("https://api.deepseek.com/anthropic/v1/messages")
        .set("Content-Type", "application/json")
        .set("x-api-key", key)
        .send_json(body)
        .map_err(|e| Error::Http(format!("LLM 调用失败: {e}")))?
        .into_json()
        .map_err(|e| Error::Http(format!("LLM 返回解析失败: {e}")))?;
    let text = resp["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    Ok(text.trim().to_string())
}

fn short(s: &str) -> String {
    s.chars().take(24).collect()
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
