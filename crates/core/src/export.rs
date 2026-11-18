//! 导出给前端读的静态 JSON。
//!
//! 字段口径与旧版 `scripts/export_web.py` 一致；前端只读这个文件，不需要后端。

use crate::aggregate::{cycle_threshold, lag_days, load_catalog};
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
    /// `Some(n)` 时额外产出 `index.json` + `shards/NN.json`（**全量，不受 limit 影响**）。
    ///
    /// 起因：单文件 `data.json` 是全量加载的，一周涨 723KB，52 周就是 37MB。
    /// 分层后索引（列表用）常驻，详情按分片懒加载。`data.json` 保持原样，
    /// 两个产物并存，前端可以分步迁移。
    pub shards: Option<usize>,
}

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
    if let Some(m) = summary.as_object_mut() {
        m.insert("proofCount".into(), json!(proof_by_id.len()));
        m.insert("evidenceCount".into(), json!(evidence_by_id.len()));
        m.insert("explainCount".into(), json!(explain_by_id.len()));
    }
    let out = json!({
        "generatedAt": "2026-11-14",
        "source": "浙江省公共数据开放平台（data.zjzwfw.gov.cn）",
        "api": "/dopServer/dataCatalog/{dataQueryPage,catalogDetail,catalogDataPreview}",
        "summary": summary,
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
        "source": full["source"],
        "api": full["api"],
        "summary": full["summary"],
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
    let lag = update_time.as_deref().and_then(lag_days);
    let overdue = cycle_threshold(&d.cycle_str())
        .map(|limit| lag.map(|l| l > limit).unwrap_or(false))
        .unwrap_or(false);
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
            shards: Some(4),
        })
        .unwrap();

        assert_eq!(o.total, n);
        assert_eq!(o.rows, 5, "data.json 仍受 limit 约束");
        assert_eq!(o.shards_written, 4);

        let index: Value =
            serde_json::from_slice(&fs::read(dir.join("index.json")).unwrap()).unwrap();
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
}
