//! 导出给前端读的静态 JSON。
//!
//! 字段口径与旧版 `scripts/export_web.py` 一致；前端只读这个文件，不需要后端。

use crate::aggregate::load_catalog;
use crate::model::{Dataset, EvidenceEntry, Explanation, ProofEntry};
use crate::util::safe_id;
use crate::{Error, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct ExportOptions<'a> {
    pub items_path: &'a str,
    pub summary_path: &'a str,
    pub proofs_path: Option<&'a str>,
    pub catalog_path: Option<&'a str>,
    pub evidence_path: Option<&'a str>,
    pub explanations_path: Option<&'a str>,
    /// 报告目录：命中 `<dir>/<safe_id>.md` 的行会在前端得到下载链接
    pub reports_dir: Option<&'a str>,
    pub out_path: &'a str,
    pub limit: usize,
    /// 快照日期（写进 `generatedAt`）。`None` 时按 `run.json` → 证据时间自动推，
    /// 都拿不到就**报错**，不猜。
    pub snapshot_date: Option<&'a str>,
    /// 跑批历史日志（`history/summary.json`）。给了就嵌进 `data.json` 的 `history`，
    /// 前端据此画曲线。只**读**不写：追加由 `assay history` 负责，
    /// 让「导出」保持纯读，副作用只在一个地方发生。
    pub history_path: Option<&'a str>,
    /// `Some(n)` 时额外产出 `index.json` + `shards/NN.json`（**全量，不受 limit 影响**）。
    ///
    /// 起因：单文件 `data.json` 是全量加载的，一周涨 723KB，52 周就是 37MB。
    /// 分层后索引（列表用）常驻，详情按分片懒加载。`data.json` 保持原样，
    /// 两个产物并存，前端可以分步迁移。
    pub shards: Option<usize>,
}

#[derive(Debug)]
pub struct ExportOutcome {
    pub rows: usize,
    pub total: usize,
    pub proofs: usize,
    pub evidence: usize,
    pub explanations: usize,
    /// 实际写出的分片数（未开 split 时为 0）
    pub shards_written: usize,
}

/// 数据集 → 分片号。纯函数、确定性：同一批数据任何时候切分结果一致。
///
/// 用 SHA-256 前 4 字节取模，而不是 `id.len()` 之类的便宜哈希——平台 id 是
/// base64（含 `+ / =`），前缀高度相似，便宜哈希会让分片严重倾斜。
///
/// 分片号同时写进索引的每一行，前端**不需要**复刻这个函数：
/// 「只有一处实现」这个不变量在跨语言边界上同样成立。
pub fn shard_of(id: &str, shards: usize) -> usize {
    if shards <= 1 {
        return 0;
    }
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(id.as_bytes());
    let d = h.finalize();
    let n = u32::from_be_bytes([d[0], d[1], d[2], d[3]]) as usize;
    n % shards
}

/// 索引行的字段白名单：列表页要用到的全部字段。
///
/// 刻意用手写白名单而不是「全字段减几个」——新增字段时默认**不进**索引，
/// 逼着人想一次「这个字段列表页真的需要吗」，避免索引悄悄膨胀回单文件的体积。
const INDEX_FIELDS: [&str; 12] = [
    "id",
    "name",
    "dept",
    "updateCycle",
    "lagDays",
    "stopped",
    "declaredCols",
    "sampleCols",
    "emptyRate",
    "allEmptyCols",
    "schemaKind",
    "score",
];

pub fn export(opts: &ExportOptions<'_>) -> Result<ExportOutcome> {
    let items = crate::audit::read_items(opts.items_path)?;
    let summary: Value = read_json(opts.summary_path)?
        .ok_or_else(|| Error::Other(format!("缺少 {}", opts.summary_path)))?;

    let mut proof_by_id: HashMap<String, ProofEntry> = HashMap::new();
    let mut evidence_by_id: HashMap<String, EvidenceEntry> = HashMap::new();
    let mut explain_by_id: HashMap<String, Explanation> = HashMap::new();

    if let Some(p) = opts.proofs_path.and_then(|p| read_json(p).ok().flatten()) {
        if let Ok(list) = serde_json::from_value::<Vec<ProofEntry>>(p) {
            for e in list {
                if !e.sha256.is_empty() {
                    proof_by_id.insert(e.id.clone(), e);
                }
            }
        }
    }
    if let Some(v) = opts.evidence_path.and_then(|p| read_json(p).ok().flatten()) {
        if let Ok(list) = serde_json::from_value::<Vec<EvidenceEntry>>(v["items"].clone()) {
            for e in list {
                evidence_by_id.insert(e.id.clone(), e);
            }
        }
    }
    if let Some(v) = opts
        .explanations_path
        .and_then(|p| read_json(p).ok().flatten())
    {
        if let Ok(list) = serde_json::from_value::<Vec<Explanation>>(v["items"].clone()) {
            for e in list {
                if !e.text.is_empty() {
                    explain_by_id.insert(e.id.clone(), e);
                }
            }
        }
    }

    let catalog: HashMap<String, String> = match opts.catalog_path {
        Some(p) => load_catalog(p)?
            .into_iter()
            .filter_map(|r| r.update_time.map(|t| (r.id, t)))
            .collect(),
        None => HashMap::new(),
    };

    let mut rows: Vec<Value> = items
        .iter()
        .map(|d| {
            row(
                d,
                &catalog,
                &proof_by_id,
                &evidence_by_id,
                &explain_by_id,
                opts.reports_dir,
            )
        })
        .collect();
    rows.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["score"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total = rows.len();
    let mut summary = summary;
    // 快照日期必须来自这次跑批，不能写死。
    //
    // 这里原来是 `const SNAPSHOT_DATE = "2026-11-14"`，注释写着「重跑时跟着改」——
    // 而事实是没人会记得改：页头写死 11-14，500 份证据每份都写着 `fetchedAt: 11-18`，
    // 同一页两个日期。写死的日期不会报错，只会安静地变旧。
    //
    // 三档来源，按可信度排队：显式参数 → `run.json`（audit 开跑时写的清单）
    // → 证据里的抓取日期。都拿不到就报错：export 没有能力知道数据是哪天的，
    // 编一个出来只会把错误藏到页面上。
    //
    // 来源本身也写进产物（`generatedAtFrom`）：同样的日期，来自清单和来自
    // 「记得传参数」是两件事，读产物的人有权知道是哪种。
    let (snapshot_date, snapshot_from) = match opts.snapshot_date {
        Some(s) => (Some(s.to_string()), "explicit"),
        None => {
            match captured_at_of(opts.items_path).and_then(|t| date_part(&t).map(str::to_string)) {
                Some(d) => (Some(d.to_string()), "run.json"),
                None => (snapshot_date_of_evidence(&evidence_by_id), "evidence"),
            }
        }
    };
    let snapshot_date = snapshot_date.ok_or_else(|| {
        Error::Other(format!(
            "无法确定快照日期：{} 旁边没有 run.json，也没有证据文件。\
             跑一次 audit 会写 run.json，或用 --snapshot-date 显式指定。",
            opts.items_path
        ))
    })?;
    if let Some(m) = summary.as_object_mut() {
        m.insert("proofCount".into(), json!(proof_by_id.len()));
        m.insert("evidenceCount".into(), json!(evidence_by_id.len()));
        m.insert("explainCount".into(), json!(explain_by_id.len()));
    }
    // 检查项清单：把「尺子」本身也发布出去。
    //
    // 之前这些阈值只存在于 Rust 源码里，读者想问「你们凭什么说它有问题、门槛多少」
    // 只能去翻代码——而翻代码的人不会相信结论，信结论的人不会去翻代码。
    // 命中数是按这一批全量算的（不受 `--limit` 影响），所以「N/M」两数都对得上。
    let tally =
        crate::checks::tally(items.iter().map(|d| {
            crate::checks::Facts::with_catalog(d, catalog.get(&d.id).map(|s| s.as_str()))
        }));
    let checks_json: Vec<crate::checks::CheckRow> = tally
        .iter()
        .map(|(c, hits)| crate::checks::CheckRow::new(c, *hits, total))
        .collect();
    // 曲线数据：只读日志。日志坏了就让导出失败——静默当成空数组会在页面上
    // 表现成「历史被清空」，而人往往先看到图、后去查原因。
    let history_json: Vec<crate::history::Point> = match opts.history_path {
        Some(p) => crate::history::read_log(p)?,
        None => Vec::new(),
    };
    let out = json!({
        "generatedAt": snapshot_date,
        "generatedAtFrom": snapshot_from,
        "source": "浙江省公共数据开放平台（data.zjzwfw.gov.cn）",
        "api": "/dopServer/dataCatalog/{dataQueryPage,catalogDetail,catalogDataPreview}",
        "summary": summary,
        "checks": checks_json,
        "history": history_json,
        "datasets": rows.iter().take(opts.limit).collect::<Vec<_>>(),
        "total": total,
    });
    if let Some(dir) = Path::new(opts.out_path).parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(opts.out_path, serde_json::to_string(&out)?)?;

    let shards_written = match opts.shards {
        Some(n) if n > 0 => write_split(&rows, &out, opts.out_path, n)?,
        _ => 0,
    };

    Ok(ExportOutcome {
        rows: rows.len().min(opts.limit),
        total,
        proofs: proof_by_id.len(),
        evidence: evidence_by_id.len(),
        explanations: explain_by_id.len(),
        shards_written,
    })
}

/// 读这次跑批的开跑时间（`audit` 写在 `items.jsonl` 旁边的 `run.json`）。
///
/// 为什么不做成必填参数：清单本来就躺在产物目录里，多一个「别忘了传」的参数
/// 就多一个忘记传的地方。文件不在返回 `None`，由调用方退到下一档来源。
fn captured_at_of(items_path: &str) -> Option<String> {
    let dir = Path::new(items_path).parent()?;
    let path = dir.join("run.json");
    let v = read_json(path.to_str()?).ok().flatten()?;
    v["capturedAt"].as_str().map(str::to_string)
}

/// 取 ISO-8601 的日期部分；形状不对就当没有。
///
/// 宁可让 `export` 报错，也不要写出 `2026-11` 这种半截日期——页面上它会被当成
/// 「抓取于 2026-11」原样显示出来，比缺字段更难发现。
fn date_part(ts: &str) -> Option<&str> {
    let head = ts.get(..10)?;
    let shaped = head.as_bytes().iter().enumerate().all(|(i, b)| match i {
        4 | 7 => *b == b'-',
        _ => b.is_ascii_digit(),
    });
    shaped.then_some(head)
}

/// 证据里出现最多的那个日期，并列取较晚的。
///
/// 证据是唯一自带时间的产物：`items.jsonl` 没有时间戳，`summary.json` 也不能加
/// （加了 `drift` 每周都会报漂移）。所以没有 `run.json` 时只能退到这一档。
///
/// 取众数而不是最大值：跑批跨零点时会有几条落在第二天，取最大值会把整批数据
/// 的日期推后一天。并列取较晚的，是为了让「一半 14 号一半 18 号」这种异常
/// 至少显示成较新的那天，而不是随机挑一个。
pub fn snapshot_date_of_evidence(evidence: &HashMap<String, EvidenceEntry>) -> Option<String> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for e in evidence.values() {
        if let Some(d) = date_part(&e.fetched_at) {
            *counts.entry(d).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by(|(da, ca), (db, cb)| ca.cmp(cb).then(da.cmp(db)))
        .map(|(d, _)| d.to_string())
}

/// 写 `index.json` + `shards/NN.json`，返回实际分片数。
///
/// 索引与 `data.json` 共用 `summary` / `total`，但 `datasets` 是**全量**且只含
/// `INDEX_FIELDS`；详情整行按 `shard_of` 分到各分片里。
///
/// 空分片也照写：前端按 `index.shards.count` 决定要拉哪些，缺文件会比空数组
/// 更难处理（404 vs 空列表）。
fn write_split(rows: &[Value], full: &Value, out_path: &str, shards: usize) -> Result<usize> {
    let dir = Path::new(out_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let shard_dir = dir.join("shards");
    fs::create_dir_all(&shard_dir)?;

    let mut buckets: Vec<Vec<Value>> = vec![Vec::new(); shards];
    let mut index_rows_out: Vec<Value> = Vec::with_capacity(rows.len());

    for v in rows {
        let id = v["id"].as_str().unwrap_or_default();
        let n = shard_of(id, shards);
        index_rows_out.push(light_row(v, Some(n)));
        buckets[n].push(v.clone());
    }

    for (n, bucket) in buckets.iter().enumerate() {
        let file = shard_dir.join(format!("{n:02}.json"));
        let body = json!({ "shard": n, "datasets": bucket });
        fs::write(file, serde_json::to_string(&body)?)?;
    }

    let index = json!({
        "generatedAt": full["generatedAt"],
        "generatedAtFrom": full["generatedAtFrom"],
        "source": full["source"],
        "api": full["api"],
        "summary": full["summary"],
        "checks": full["checks"],
        "history": full["history"],
        "total": full["total"],
        "shards": { "count": shards, "dir": "shards" },
        "datasets": index_rows_out,
    });
    fs::write(dir.join("index.json"), serde_json::to_string(&index)?)?;
    Ok(shards)
}

/// 把整行压成列表页要用的轻量行。
///
/// `hasProof` / `shard` 是派生字段：前者来自 `proof` 是否存在（列表上有个
/// 「已证」标记），后者是分片号。两者都进索引，前端就不必自己推导。
fn light_row(v: &Value, shard: Option<usize>) -> Value {
    let mut obj = serde_json::Map::with_capacity(INDEX_FIELDS.len() + 2);
    for k in INDEX_FIELDS {
        obj.insert(k.to_string(), v[k].clone());
    }
    obj.insert("hasProof".into(), json!(!v["proof"].is_null()));
    if let Some(n) = shard {
        obj.insert("shard".into(), json!(n));
    }
    Value::Object(obj)
}

/// 供 `assay ingest` 复用：算出与 `index.json` 单行同形的行（不带 `shard`）。
///
/// 为什么不另写一份：字段分类（`scoring::classify`）、评分（`scoring::score`）、
/// 停更判定（`STOPPED_CYCLES`）、部门取末段这些规则一旦在「导出静态文件」和
/// 「写数据库」两处各写一遍，迟早分叉——而分叉后表现是前端和接口对同一条数据
/// 给出不同分数，最难查。所以这里直接走 `row()` + `light_row()`。
///
/// 返回顺序与 `export` 一致（问题分降序），调用方不必再排。
pub fn index_rows(
    items_path: &str,
    catalog_path: Option<&str>,
    proofs_path: Option<&str>,
) -> Result<Vec<Value>> {
    let items = crate::audit::read_items(items_path)?;

    let mut proof_by_id: HashMap<String, ProofEntry> = HashMap::new();
    if let Some(p) = proofs_path.and_then(|p| read_json(p).ok().flatten()) {
        if let Ok(list) = serde_json::from_value::<Vec<ProofEntry>>(p) {
            for e in list {
                if !e.sha256.is_empty() {
                    proof_by_id.insert(e.id.clone(), e);
                }
            }
        }
    }

    let catalog: HashMap<String, String> = match catalog_path {
        Some(p) => load_catalog(p)?
            .into_iter()
            .filter_map(|r| r.update_time.map(|t| (r.id, t)))
            .collect(),
        None => HashMap::new(),
    };

    let no_evidence: HashMap<String, EvidenceEntry> = HashMap::new();
    let no_explain: HashMap<String, Explanation> = HashMap::new();
    let mut rows: Vec<Value> = items
        .iter()
        .map(|d| {
            light_row(
                &row(d, &catalog, &proof_by_id, &no_evidence, &no_explain, None),
                None,
            )
        })
        .collect();
    rows.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["score"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(rows)
}

fn row(
    d: &Dataset,
    catalog: &HashMap<String, String>,
    proofs: &HashMap<String, ProofEntry>,
    evidence: &HashMap<String, EvidenceEntry>,
    explanations: &HashMap<String, Explanation>,
    reports_dir: Option<&str>,
) -> Value {
    let update_time = catalog.get(&d.id).cloned();
    let facts = crate::checks::Facts::with_catalog(d, update_time.as_deref());
    let lag = facts.lag_days();
    let overdue = facts.overdue();
    let report = reports_dir
        .map(|dir| {
            Path::new(dir)
                .join(format!("{}.md", safe_id(&d.id)))
                .is_file()
        })
        .unwrap_or(false);
    let cycle = d.cycle_str();
    let stopped = crate::scoring::STOPPED_CYCLES.contains(&cycle.as_str());
    json!({
        "id": d.id,
        "name": d.name,
        "dept": d.dept.as_deref().unwrap_or("").rsplit('/').next().unwrap_or(""),
        "openType": d.open_type,
        "updateCycle": d.update_cycle,
        "updateDetail": d.update_cycle_detail,
        "updateTime": update_time,
        "lagDays": lag,
        "overdue": overdue,
        "stopped": stopped,
        "declaredCols": d.declared_cols,
        "sampleCols": d.sample_cols,
        "sampleRows": d.sample_rows,
        "emptyRate": d.empty_rate,
        "allEmptyCols": d.all_empty_cols,
        "missingFields": d.missing_fields,
        "extraFields": d.extra_fields,
        "schemaKind": crate::scoring::classify(d).as_str(),
        "descLen": d.desc_len,
        "hasPreview": d.has_preview,
        "score": crate::scoring::score(d),
        "proof": proofs.get(&d.id),
        "evidence": evidence.get(&d.id),
        "explain": explanations.get(&d.id),
        "report": report,
    })
}

fn read_json(path: &str) -> Result<Option<Value>> {
    if !Path::new(path).exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Dataset;
    use std::collections::HashSet;

    fn ids(n: usize) -> Vec<String> {
        // 前缀高度相似，模拟平台 base64 id 的形状
        (0..n).map(|i| format!("Y2F0YWxvZ0lk{i:08}==")).collect()
    }

    #[test]
    fn shard_of_is_deterministic_and_in_range() {
        for id in ids(500) {
            let a = shard_of(&id, 24);
            let b = shard_of(&id, 24);
            assert_eq!(a, b, "同一 id 两次结果必须一致");
            assert!(a < 24, "分片号越界：{a}");
        }
        assert_eq!(shard_of("anything", 1), 0, "只有一片时必须是 0");
    }

    #[test]
    fn shard_distribution_is_not_skewed() {
        // 便宜的哈希（长度、首字符）会让 base64 前缀相似的 id 全挤在一片
        let shards = 24;
        let mut counts = vec![0usize; shards];
        let sample = ids(20_000);
        for id in &sample {
            counts[shard_of(id, shards)] += 1;
        }
        let min = *counts.iter().min().unwrap();
        let max = *counts.iter().max().unwrap();
        let avg = sample.len() / shards;
        assert!(min > 0, "有分片是空的：{counts:?}");
        assert!(
            max < avg * 2,
            "分布倾斜：max={max} avg={avg} counts={counts:?}"
        );
    }

    #[test]
    fn split_writes_every_row_exactly_once() {
        let dir = std::env::temp_dir().join("assay-split-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let items_path = dir.join("items.jsonl");
        let mut body = String::new();
        let n = 37;
        for i in 0..n {
            let d = Dataset {
                id: format!("id/{i}"),
                name: format!("数据集 {i}"),
                dept: Some("某部门/子部门".into()),
                declared_cols: 7,
                sample_cols: if i % 3 == 0 { 9 } else { 7 },
                empty_rate: (i as f64) / 100.0,
                has_preview: 1,
                ..Default::default()
            };
            body.push_str(&serde_json::to_string(&d).unwrap());
            body.push('\n');
        }
        fs::write(&items_path, body).unwrap();
        let summary_path = dir.join("summary.json");
        fs::write(&summary_path, r#"{"audited":37}"#).unwrap();
        // 跑批清单：`generatedAt` 从这读，不写死
        fs::write(
            dir.join("run.json"),
            r#"{"capturedAt":"2027-03-05T09:12:00+08:00"}"#,
        )
        .unwrap();

        let out_path = dir.join("data.json");
        let o = export(&ExportOptions {
            items_path: items_path.to_str().unwrap(),
            summary_path: summary_path.to_str().unwrap(),
            proofs_path: None,
            catalog_path: None,
            evidence_path: None,
            explanations_path: None,
            reports_dir: None,
            out_path: out_path.to_str().unwrap(),
            // limit 故意小于总数：索引与分片必须是全量，不受它影响
            limit: 5,
            snapshot_date: None,
            history_path: None,
            shards: Some(4),
        })
        .unwrap();

        assert_eq!(o.total, n);
        assert_eq!(o.rows, 5, "data.json 仍受 limit 约束");
        assert_eq!(o.shards_written, 4);

        let data: Value = serde_json::from_slice(&fs::read(&out_path).unwrap()).unwrap();
        assert_eq!(
            data["generatedAt"], "2027-03-05",
            "generatedAt 必须来自 run.json，不是写死的常量"
        );
        assert_eq!(data["generatedAtFrom"], "run.json", "来源也要落盘");
        let index: Value =
            serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
        assert_eq!(
            index["generatedAt"], data["generatedAt"],
            "index.json 与 data.json 的日期必须一致，否则分层后两页各说各话"
        );
        let rows = index["datasets"].as_array().unwrap();
        assert_eq!(rows.len(), n, "索引必须是全量");
        assert_eq!(index["shards"]["count"], 4);

        // 索引里每行都必须有分片号，且索引本身不含重字段
        for r in rows {
            assert!(r["shard"].is_u64(), "索引行缺 shard：{r}");
            assert!(r["hasProof"].is_boolean());
            for heavy in [
                "proof",
                "evidence",
                "explain",
                "sampleRows",
                "missingFields",
            ] {
                assert!(r.get(heavy).is_none(), "索引不该带重字段 {heavy}");
            }
        }

        // 分片合并起来必须正好等于索引里的 id 集合，不多不少
        let index_ids: HashSet<&str> = rows.iter().map(|r| r["id"].as_str().unwrap()).collect();
        let mut shard_ids: HashSet<String> = HashSet::new();
        for s in 0..4 {
            let p = dir.join("shards").join(format!("{s:02}.json"));
            let v: Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
            assert_eq!(v["shard"], s);
            for r in v["datasets"].as_array().unwrap() {
                let id = r["id"].as_str().unwrap().to_string();
                assert!(shard_ids.insert(id.clone()), "id 重复：{id}");
                // 分片里的行必须与索引记的分片号一致
                let row = rows.iter().find(|x| x["id"] == r["id"]).unwrap();
                assert_eq!(row["shard"].as_u64().unwrap() as usize, s, "分片号不一致");
            }
        }
        assert_eq!(shard_ids.len(), n, "分片总量与索引不符");
        assert!(
            index_ids.iter().all(|i| shard_ids.contains(*i)),
            "分片丢了索引里的某些 id"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn index_rows_matches_index_json_rows() {
        // ingest 写库的字段必须与 index.json 逐字段同形。两者都从 `row()` 派生，
        // 但只要有人改了 INDEX_FIELDS 却忘了另一边，这里立刻红——这正是
        // 「字段分类只有一处实现」在跨产物上的落点。
        let dir = std::env::temp_dir().join("assay-index-rows-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let items_path = dir.join("items.jsonl");
        let mut body = String::new();
        for i in 0..5 {
            let d = Dataset {
                id: format!("id/{i}"),
                name: format!("数据集 {i}"),
                dept: Some("省交通运输厅/子处".into()),
                declared_cols: 7,
                sample_cols: if i % 2 == 0 { 7 } else { 9 },
                empty_rate: (i as f64) / 10.0,
                has_preview: 1,
                update_cycle: Some(serde_json::json!(30)),
                ..Default::default()
            };
            body.push_str(&serde_json::to_string(&d).unwrap());
            body.push('\n');
        }
        fs::write(&items_path, body).unwrap();

        let summary_path = dir.join("summary.json");
        fs::write(&summary_path, r#"{"audited":5}"#).unwrap();
        let out_path = dir.join("data.json");
        export(&ExportOptions {
            items_path: items_path.to_str().unwrap(),
            summary_path: summary_path.to_str().unwrap(),
            proofs_path: None,
            catalog_path: None,
            evidence_path: None,
            explanations_path: None,
            reports_dir: None,
            out_path: out_path.to_str().unwrap(),
            limit: 5,
            snapshot_date: Some("2026-01-02"),
            history_path: None,
            shards: Some(2),
        })
        .unwrap();

        let index: Value =
            serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
        let exported: std::collections::HashMap<String, Value> = index["datasets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["id"].as_str().unwrap().to_string(), r.clone()))
            .collect();

        let rows = index_rows(items_path.to_str().unwrap(), None, None).unwrap();
        assert_eq!(rows.len(), 5);
        for r in &rows {
            let id = r["id"].as_str().unwrap();
            let want = exported
                .get(id)
                .unwrap_or_else(|| panic!("导出侧没有 {id}"));
            assert!(
                r.get("shard").is_none(),
                "ingest 行不该带分片号，那是静态分片专用的"
            );
            for k in INDEX_FIELDS.iter().chain(["hasProof"].iter()) {
                assert_eq!(&r[*k], &want[*k], "字段 {k} 两侧不一致（id={id}）");
            }
        }

        // 顺序也必须一致，否则「榜单前 N」在接口与静态站会不同
        assert_eq!(rows[0]["id"], index["datasets"][0]["id"]);

        let _ = fs::remove_dir_all(&dir);
    }

    fn evidence(id: &str, fetched_at: &str) -> EvidenceEntry {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": format!("数据集 {id}"),
            "fetchedAt": fetched_at,
            "detailBytes": 1,
            "previewBytes": 1,
            "sha256": "deadbeef",
            "files": [],
        }))
        .unwrap()
    }

    #[test]
    fn date_part_only_accepts_full_dates() {
        assert_eq!(date_part("2026-11-18T14:33:46+08:00"), Some("2026-11-18"));
        assert_eq!(date_part("2026-11-18"), Some("2026-11-18"));
        // 半截日期不许通过：页面上会被原样显示成「抓取于 2026-11」
        assert_eq!(date_part("2026-11"), None);
        assert_eq!(date_part("2026/11/18"), None);
        assert_eq!(date_part(""), None);
    }

    #[test]
    fn evidence_date_uses_mode_not_max() {
        // 跑批跨零点：3 条落在 14 号、2 条落在 15 号，整批属于 14 号
        let mut ev = HashMap::new();
        for (i, d) in ["2026-11-14", "2026-11-14", "2026-11-14"]
            .iter()
            .enumerate()
        {
            ev.insert(
                format!("a{i}"),
                evidence(&format!("a{i}"), &format!("{d}T23:5{i}:00+08:00")),
            );
        }
        for (i, d) in ["2026-11-15", "2026-11-15"].iter().enumerate() {
            ev.insert(
                format!("b{i}"),
                evidence(&format!("b{i}"), &format!("{d}T00:0{i}:00+08:00")),
            );
        }
        assert_eq!(
            snapshot_date_of_evidence(&ev).as_deref(),
            Some("2026-11-14")
        );

        // 并列取较晚的：不能随机挑一个
        let mut tie = HashMap::new();
        tie.insert("x".into(), evidence("x", "2026-11-14T10:00:00+08:00"));
        tie.insert("y".into(), evidence("y", "2026-11-18T10:00:00+08:00"));
        assert_eq!(
            snapshot_date_of_evidence(&tie).as_deref(),
            Some("2026-11-18")
        );

        assert_eq!(snapshot_date_of_evidence(&HashMap::new()), None);
    }

    #[test]
    fn export_refuses_to_invent_a_date() {
        // 没有 run.json、也没有证据：无从知道数据是哪天的。
        // 以前这里会静默写出写死的常量，页头于是长期显示一个错的日期。
        let dir = std::env::temp_dir().join("assay-nodate-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let items_path = dir.join("items.jsonl");
        let d = Dataset {
            id: "id/0".into(),
            name: "数据集 0".into(),
            ..Default::default()
        };
        fs::write(
            &items_path,
            format!("{}\n", serde_json::to_string(&d).unwrap()),
        )
        .unwrap();
        let summary_path = dir.join("summary.json");
        fs::write(&summary_path, r#"{"audited":1}"#).unwrap();

        let err = export(&ExportOptions {
            items_path: items_path.to_str().unwrap(),
            summary_path: summary_path.to_str().unwrap(),
            proofs_path: None,
            catalog_path: None,
            evidence_path: None,
            explanations_path: None,
            reports_dir: None,
            out_path: dir.join("data.json").to_str().unwrap(),
            limit: 5,
            snapshot_date: None,
            history_path: None,
            shards: None,
        })
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("无法确定快照日期"), "错误信息要说得清：{msg}");
        assert!(msg.contains("--snapshot-date"), "错误信息要给出出路：{msg}");

        let _ = fs::remove_dir_all(&dir);
    }
}
