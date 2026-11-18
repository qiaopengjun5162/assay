//! 解释层：模型写人话，数字必须过护栏。
//!
//! 位置约定：模型只看事实字典，拿不到原始数据；输出里的每个数字都要能在事实里找到，
//! 不合格重写一次，仍不合格标记 `rejected`，页面上不展示。

use crate::aggregate::{lag_days, load_catalog};
use crate::config::{LlmConfig, Protocol};
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
    /// 实际用的模型名，写进 `explanations.json` 与本行日志。
    pub model: String,
}

/// 单次调用自检：只发一个问题，验证端点 / 模型 / key / 代理这四件事是否都通。
///
/// 存在的理由：`explain --top 100` 要跑几分钟，配置错了要等几分钟才知道；
/// 而且失败信息往往只有一句 HTTP 401，看不出到底是地址错了还是渠道不认这个模型。
pub struct ProbeReport {
    pub endpoint: String,
    pub model: String,
    pub protocol: &'static str,
    pub source: String,
    pub masked_key: String,
    pub proxy: Option<String>,
    pub reply: String,
    pub latency_secs: f64,
}

pub fn probe() -> Result<ProbeReport> {
    let cfg = LlmConfig::resolve().map_err(Error::Other)?;
    let t0 = Instant::now();
    let reply = call_once(&cfg, "只回复两个字：可以", 16)?;
    Ok(ProbeReport {
        endpoint: cfg.endpoint(),
        model: cfg.model.clone(),
        protocol: cfg.protocol().as_str(),
        source: describe_source(&cfg),
        masked_key: cfg.masked_key(),
        proxy: cfg.proxy.clone(),
        reply,
        latency_secs: t0.elapsed().as_secs_f64(),
    })
}

fn describe_source(cfg: &LlmConfig) -> String {
    match &cfg.source {
        crate::config::Source::Env => "环境变量".to_string(),
        crate::config::Source::File(p) => format!("配置文件 {p}"),
        crate::config::Source::Default => "兜底值".to_string(),
    }
}

fn client(cfg: &LlmConfig) -> Result<ureq::Agent> {
    let mut b = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(90));
    if let Some(p) = &cfg.proxy {
        b = b.proxy(
            ureq::Proxy::new(p).map_err(|e| Error::Http(format!("代理地址 {p} 无效: {e}")))?,
        );
    }
    Ok(b.build())
}

pub fn explain(opts: &ExplainOptions<'_>) -> Result<ExplainOutcome> {
    let cfg = LlmConfig::resolve().map_err(Error::Other)?;
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
                let rec = run_one(d, &facts, &cfg);
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
        "model": cfg.model.clone(),
        "endpoint": cfg.endpoint(),
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
        model: cfg.model,
    })
}

fn run_one(d: &Dataset, facts: &[(&'static str, String)], cfg: &LlmConfig) -> Explanation {
    let mut last_text = String::new();
    let mut last_illegal: Vec<String> = Vec::new();
    for attempt in 1..=2 {
        let hint = if attempt == 1 {
            "请写 3 句解释。".to_string()
        } else {
            "上一版不合格（数字越界、引用过多或被截断），请重写：只写三句完整的中文，不超过 120 字，数字最多 3 个，结尾必须是句号。".to_string()
        };
        let text = match call_model(cfg, facts, &hint) {
            Ok(t) => t,
            // 传输层失败（超时、空响应、429）也重试一次。
            // 「重写一次」原本只覆盖护栏不合格，但调模型最常见的失败其实是网络和空返回，
            // 一次抖动就让这条永久标成 error，等于把可用数据丢掉。
            Err(e) if attempt < 2 => {
                eprintln!("  RETRY {}  {e}（第 {attempt} 次）", short(&d.name));
                last_illegal = vec![e.to_string()];
                continue;
            }
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

/// 组请求体。两家只有三处差异：system 放哪、messages 的结构、max_tokens 的位置。
fn body_for(cfg: &LlmConfig, prompt: &str, max_tokens: u32) -> Value {
    match cfg.protocol() {
        Protocol::Anthropic => json!({
            "model": cfg.model,
            "max_tokens": max_tokens,
            "system": SYSTEM,
            "messages": [{"role": "user", "content": [{"type": "text", "text": prompt}]}],
        }),
        Protocol::OpenAi => json!({
            "model": cfg.model,
            "max_tokens": max_tokens,
            "messages": [
                {"role": "system", "content": SYSTEM},
                {"role": "user", "content": prompt},
            ],
        }),
    }
}

/// 从响应里取文本。两种格式的路径不同，都试一遍比按协议分支更耐改。
fn text_of(resp: &Value) -> String {
    // OpenAI: choices[0].message.content
    if let Some(s) = resp["choices"][0]["message"]["content"].as_str() {
        return s.trim().to_string();
    }
    // Anthropic: content: [{type: "text", text: "..."}]
    resp["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn call_model(cfg: &LlmConfig, facts: &[(&'static str, String)], hint: &str) -> Result<String> {
    let facts_json = crate::guard::facts_text(facts);
    let prompt = format!("事实：\n{facts_json}\n\n{hint}");
    let text = call_with(cfg, &prompt, 1500)?;
    if text.is_empty() {
        return Err(Error::Http("LLM 返回空文本".into()));
    }
    Ok(text)
}

/// 一次请求。`probe` 也走这里，保证自检路径和生产路径是同一条。
fn call_once(cfg: &LlmConfig, prompt: &str, max_tokens: u32) -> Result<String> {
    call_with(cfg, prompt, max_tokens)
}

fn call_with(cfg: &LlmConfig, prompt: &str, max_tokens: u32) -> Result<String> {
    let body = body_for(cfg, prompt, max_tokens);
    let mut req = client(cfg)?
        .post(&cfg.endpoint())
        .set("Content-Type", "application/json");
    req = match cfg.protocol() {
        Protocol::Anthropic => req
            .set("x-api-key", &cfg.api_key)
            .set("anthropic-version", "2023-06-01"),
        Protocol::OpenAi => req.set("Authorization", &format!("Bearer {}", cfg.api_key)),
    };
    let resp = req.send_json(body).map_err(|e| match e {
        ureq::Error::Status(code, r) => {
            // 上游的错误体前面往往带着真正的原因（模型不存在、余额不足、key 无效），
            // 只报 HTTP 状态码等于让人去猜。
            let detail = r.into_string().unwrap_or_default();
            let detail = detail.trim();
            if detail.is_empty() {
                Error::Http(format!("LLM 调用失败: HTTP {code}"))
            } else {
                Error::Http(format!(
                    "LLM 调用失败: HTTP {code} · {}",
                    truncate(detail, 300)
                ))
            }
        }
        other => Error::Http(format!("LLM 调用失败: {other}")),
    })?;
    let value: Value = resp
        .into_json()
        .map_err(|e| Error::Http(format!("LLM 返回不是 JSON: {e}")))?;
    Ok(text_of(&value))
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

fn short(s: &str) -> String {
    s.chars().take(24).collect()
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LlmConfig, Protocol, Source};

    fn cfg(base: &str) -> LlmConfig {
        LlmConfig {
            api_key: "sk-test".into(),
            base_url: base.into(),
            model: "deepseek-v4.1-flash".into(),
            proxy: None,
            source: Source::Default,
        }
    }

    /// 换渠道最容易踩的坑：把 OpenAI 兼容口也发成 anthropic 形状，
    /// 上游回 400 但说不清哪里错。这条钉住两种形状。
    #[test]
    fn openai_body_puts_system_in_messages() {
        let b = body_for(&cfg("https://platform.yeta.ai/v1"), "问一句", 16);
        assert_eq!(b["model"], "deepseek-v4.1-flash");
        assert_eq!(b["messages"][0]["role"], "system");
        assert_eq!(b["messages"][1]["role"], "user");
        assert_eq!(b["messages"][1]["content"], "问一句");
        assert!(b.get("system").is_none());
    }

    #[test]
    fn anthropic_body_keeps_system_top_level() {
        let c = cfg("https://api.deepseek.com/anthropic");
        assert_eq!(c.protocol(), Protocol::Anthropic);
        let b = body_for(&c, "问一句", 16);
        assert_eq!(b["system"], SYSTEM);
        assert_eq!(b["messages"][0]["content"][0]["type"], "text");
    }

    #[test]
    fn text_of_reads_both_shapes() {
        let openai = serde_json::json!({
            "choices": [{"message": {"role": "assistant", "content": "  可以  "}}]
        });
        assert_eq!(text_of(&openai), "可以");

        let anthropic = serde_json::json!({
            "content": [{"type": "text", "text": "可"}, {"type": "text", "text": "以"}]
        });
        assert_eq!(text_of(&anthropic), "可以");
    }

    /// 上游错误体要带出来，否则 401 和「模型不存在」在日志里长得一样。
    #[test]
    fn truncate_keeps_head() {
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("abcdef", 3), "abc…");
    }
}
