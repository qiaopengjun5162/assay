//! 前端数据模型：与 `web/data.json` 一一对应。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    #[serde(default)]
    pub summary: Summary,
    #[serde(default)]
    pub datasets: Vec<Row>,
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub generated_at: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    #[serde(default)]
    pub audited: u64,
    #[serde(default)]
    pub with_preview: u64,
    #[serde(default)]
    pub empty_rate_mean: Option<f64>,
    #[serde(default)]
    pub empty_rate_over30: u64,
    #[serde(default)]
    pub all_empty_column_datasets: u64,
    #[serde(default)]
    pub schema_mismatch: u64,
    #[serde(default)]
    pub stopped_update: u64,
    #[serde(default)]
    pub lag_over90_days: u64,
    #[serde(default)]
    pub lag_median_days: Option<i64>,
    #[serde(default)]
    pub proof_count: usize,
    #[serde(default)]
    pub evidence_count: usize,
    #[serde(default)]
    pub explain_count: usize,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub dept: String,
    #[serde(default)]
    pub open_type: Option<String>,
    #[serde(default)]
    pub update_cycle: serde_json::Value,
    #[serde(default)]
    pub update_time: Option<String>,
    #[serde(default)]
    pub lag_days: Option<i64>,
    #[serde(default)]
    pub overdue: bool,
    #[serde(default)]
    pub declared_cols: usize,
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
    #[serde(default)]
    pub schema_kind: String,
    #[serde(default)]
    pub desc_len: usize,
    #[serde(default)]
    pub has_preview: i64,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub proof: Option<Proof>,
    #[serde(default)]
    pub evidence: Option<Evidence>,
    #[serde(default)]
    pub explain: Option<Explain>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Proof {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub rows: usize,
    #[serde(default)]
    pub cols: usize,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub fetched_at: String,
    #[serde(default)]
    pub detail_bytes: usize,
    #[serde(default)]
    pub preview_bytes: usize,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Explain {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub status: String,
}

impl Row {
    pub fn cycle_label(&self) -> String {
        let s = match &self.update_cycle {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        };
        match s.as_str() {
            "1" => "每日".into(),
            "7" => "每周".into(),
            "30" => "每月".into(),
            "365" => "每年".into(),
            "-1" | "999" | "不更新" => "停更".into(),
            "" => "未知".into(),
            other => other.to_string(),
        }
    }

    pub fn stopped(&self) -> bool {
        matches!(self.cycle_label().as_str(), "停更")
    }

    pub fn empty_pct(&self) -> f64 {
        self.empty_rate * 100.0
    }
}
