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
}

pub struct ExportOutcome {
    pub rows: usize,
    pub total: usize,
    pub proofs: usize,
    pub evidence: usize,
    pub explanations: usize,
}

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
    Ok(ExportOutcome {
        rows: rows.len().min(opts.limit),
        total,
        proofs: proof_by_id.len(),
        evidence: evidence_by_id.len(),
        explanations: explain_by_id.len(),
    })
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
