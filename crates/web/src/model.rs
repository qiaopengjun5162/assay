//! 前端数据模型：与 `web/data.json` 一一对应。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    #[serde(default)]
    pub summary: Summary,
    #[serde(default)]
    pub datasets: Vec<Row>,
    /// 检查项清单：由 `crates/core/src/checks.rs` 直接发布，前端不自己写文案。
    #[serde(default)]
    pub checks: Vec<CheckRow>,
    /// 跑批历史（`history/summary.json` 的镜像）。空数组 = 还没开始攒。
    #[serde(default)]
    pub history: Vec<HistoryPoint>,
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub generated_at: String,
}

/// 一条检查项的公开定义。`hits` 是全量命中数（分母是 `Data::total`，不是榜单行数）。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckRow {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub dim: String,
    #[serde(default)]
    pub weight: f64,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub hits: u64,
    #[serde(default)]
    pub of: u64,
}

/// 一次跑批的指标点。与 `crates/core/src/history.rs` 的 `Point` 同形。
///
/// 只含原始计数：问题分随权重变，曲线里混进分数等于把两把尺子画在同一张图上。
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub audited: u64,
    #[serde(default)]
    pub with_preview: u64,
    #[serde(default)]
    pub lag_over90: u64,
    #[serde(default)]
    pub mismatch: u64,
    #[serde(default)]
    pub all_empty: u64,
    #[serde(default)]
    pub stopped: u64,
    #[serde(default)]
    pub empty_over30: u64,
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
    /// 部门维度统计，key 是部门全名
    #[serde(default)]
    pub by_dept: std::collections::BTreeMap<String, DeptRow>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeptRow {
    #[serde(default)]
    pub n: u64,
    /// 平均空值率。`None` 表示这个部门一条样例都没拿到——是「不知道」，不是 0。
    ///
    /// 老版本 `aggregate` 把这个 key 写成 snake_case，仓库里入库的
    /// `web/data.json` 快照还是旧格式；两种都认，否则部门榜的空值率会静默变 0。
    #[serde(default, alias = "empty_avg")]
    pub empty_avg: Option<f64>,
    #[serde(default)]
    pub stopped: u64,
    #[serde(default)]
    pub mismatch: u64,
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
    /// 站点里有这份数据集的 Markdown 报告
    #[serde(default)]
    pub report: bool,
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

    /// 非空比例，给「完整性」维度用。
    ///
    /// 不能把 `empty_pct()` 挂到完整性下面：空值率 100% 的数据集会显示成
    /// 「完整性 100.0%」，把最差的说成最好的。这个 bug 在页面上安静地显示，
    /// 只有对着同一条的报告（写的是「样例空值率 100.00%」）才看得出矛盾。
    pub fn fill_pct(&self) -> f64 {
        (1.0 - self.empty_rate) * 100.0
    }

    /// 报告文件名：与后端 `util::safe_id` 同一映射（非 `[A-Za-z0-9_-]` 一律换成 `_`，截 64）。
    pub fn report_path(&self) -> String {
        let name: String = self
            .id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .take(64)
            .collect();
        format!("reports/{name}.md")
    }
}
