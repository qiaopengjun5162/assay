//! 数据模型：目录记录、体检条目、证据与解读。
//!
//! 序列化口径与旧版 Python 产物保持一致（camelCase），
//! 这样重构期间可以直接拿已有的 `out-full/*.json(l)` 做对比测试。

use serde::{Deserialize, Serialize};

/// 目录接口返回的一条记录（只保留用得到的字段，其余忽略）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub source_name: String,
    #[serde(default)]
    pub private_dept: Option<String>,
    #[serde(default)]
    pub open_type: Option<String>,
    #[serde(default)]
    pub update_cycle: Option<String>,
    #[serde(default)]
    pub data_size: Option<String>,
    #[serde(default)]
    pub area_type: Option<String>,
    /// 平台给的最后更新时间，形如 `2025-06-09 08:27:49`
    #[serde(default)]
    pub update_time: Option<String>,
    #[serde(default)]
    pub region_id: Option<String>,
}

/// 一个数据集的体检结果（items.jsonl 的一行）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dataset {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i: Option<usize>,
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dept: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_cycle: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_cycle_detail: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_type: Option<String>,
    #[serde(default)]
    pub declared_cols: usize,
    #[serde(default)]
    pub desc_len: usize,
    #[serde(default)]
    pub has_preview: i64,
    #[serde(default)]
    pub sample_cols: usize,
    #[serde(default)]
    pub sample_rows: usize,
    #[serde(default)]
    pub empty_rate: f64,
    #[serde(default)]
    pub all_empty_cols: usize,
    #[serde(default)]
    pub missing_fields: Vec<String>,
    #[serde(default)]
    pub extra_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail_error: Option<String>,

    // 聚合阶段补上的派生字段
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lag_days: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overdue: Option<bool>,
}

impl Dataset {
    /// 平台声明的更新周期（字符串化，便于和 STOPPED 集合比对）。
    pub fn cycle_str(&self) -> String {
        match &self.update_cycle {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            _ => String::new(),
        }
    }
}

/// 证明条目（proofs.json 的一项）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub dept: Option<String>,
    pub rows: usize,
    pub cols: usize,
    #[serde(default)]
    pub empty_rate: f64,
    #[serde(default)]
    pub all_empty_cols: usize,
    #[serde(default)]
    pub missing_fields: Vec<String>,
    pub sha256: String,
    #[serde(default)]
    pub source: Option<String>,
}

/// 证据包条目（evidence.json 的一项）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceEntry {
    pub id: String,
    pub name: String,
    pub fetched_at: String,
    pub detail_bytes: usize,
    pub preview_bytes: usize,
    pub sha256: String,
    pub files: Vec<String>,
}

/// 解释层条目。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub text: String,
    pub status: String,
    #[serde(default)]
    pub attempts: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub numbers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub illegal: Vec<String>,
}

/// 部门维度的聚合结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeptStat {
    pub n: usize,
    pub preview: usize,
    pub empty_sum: f64,
    pub empty_n: usize,
    pub all_empty: usize,
    pub mismatch: usize,
    pub no_desc: usize,
    pub stopped: usize,
    #[serde(skip_serializing)]
    pub empty_avg: Option<f64>,
}
