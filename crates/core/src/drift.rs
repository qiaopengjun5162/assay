//! 周度漂移对比：把两次体检的 `summary.json` 摆在一起，判断指标是否漂了。
//!
//! 本模块在纯函数层：不读文件、不联网，`cli` 负责 IO，单测可以全速跑。
//!
//! 口径三条（改之前先想清楚）：
//!
//! 1. **比例类**指标按「占 `audited` 的百分比点数（pp）」比，不比绝对条数。
//!    数据集总量每周都在变，绝对条数涨了不代表质量变了。
//! 2. **绝对类**指标（中位滞后天数、空值率中位数）按原单位比。
//! 3. `audited` 本身只记录不告警——平台增删数据集是常态。
//!
//! 关键约束：跑批不健康时**不判漂移**（见 [`Verdict::Suspect`]）。否则一次抓取
//! 抖动就会把脏数字写进基线，此后所有对比全部失真。这比漏报一次漂移贵得多。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `summary.json` 的读侧视图。
///
/// 字段全为 `Option`：基线可能来自旧版本代码，缺字段必须当「不知道」，
/// 不能当 0——否则新增字段的第一周会凭空报一次漂移。
/// 未列出的字段（`byDept` / `updateCycle` / `mismatchKinds`）反序列化时被忽略：
/// 它们逐周波动大，放进基线只会让 PR diff 变成噪音。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// 采集日期（`YYYY-MM-DD`）。只有基线文件带，CI 新鲜产出的 summary 没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<String>,
    /// 采集时的提交号，用于追溯「这条基线是哪一版代码跑出来的」。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audited: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_preview: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_mismatch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_update: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_cycle_overdue: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all_empty_column_datasets: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_column_list: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_rate_over30: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_rate_median: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_rate_p90: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lag_median_days: Option<i64>,
}

impl Summary {
    /// 从 `summary.json` 的 JSON 值读指标。
    ///
    /// 同时接受两种形状：直接是 summary 本体，或包在 `{"summary": {...}}` 里。
    /// 后者是为了让「一次跑批的全量快照」也能直接拿来当基线用。
    ///
    /// # Errors
    /// JSON 结构不是对象、或字段类型对不上时返回 [`Error::Json`]。
    pub fn from_value(mut v: Value) -> crate::Result<Self> {
        if let Some(inner) = v.get("summary") {
            v = inner.clone();
        }
        Ok(serde_json::from_value(v)?)
    }

    /// 写成基线文件的文本（pretty JSON，字段缺省即省略）。
    ///
    /// # Errors
    /// 序列化失败时返回 [`Error::Json`]（正常不会发生）。
    pub fn to_baseline_text(&self) -> crate::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// 漂移阈值。**全部是经验值**，需要按实测校准，别当定理用。
///
/// 校准方法见 `docs/audit/README.md`：连续跑 4 周，取真实波动的 P95 再留一倍余量。
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    /// 抓取成功率（`withPreview / audited`）下跌超过这么多 pp → 判跑批异常。
    pub preview_drop_pp: f64,
    /// 比例类指标变化超过这么多 pp → 判漂移。
    pub ratio_pp: f64,
    /// 中位滞后天数变化超过这么多天 → 判漂移。
    pub lag_days: i64,
    /// 空值率中位数变化超过这个绝对值（0~1）→ 判漂移。
    pub empty_rate: f64,
    /// 目录条目数减少超过这个比例 → 判跑批异常（多半是分页/接口坏了）。
    pub audited_drop: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            preview_drop_pp: 5.0,
            ratio_pp: 2.0,
            lag_days: 14,
            empty_rate: 0.05,
            audited_drop: 0.2,
        }
    }
}

/// 单个指标的显示单位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// 百分数（值已乘 100）或百分比点数。
    Percent,
    /// 天。
    Days,
    /// 条。
    Count,
}

/// 单条指标的判定。
///
/// `Info` 不是「通过」，而是「这个指标只记录、不告警」——`audited` 用这一档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Stable,
    Drift,
    Info,
}

/// 一行对比结果。
///
/// `baseline` / `current` 用 [`Unit`] 指定的单位；`delta` 是 `current - baseline`。
#[derive(Debug, Clone)]
pub struct MetricDelta {
    pub name: &'static str,
    pub baseline: f64,
    pub current: f64,
    pub delta: f64,
    pub unit: Unit,
    pub level: Level,
}

/// 抓取成功率对比。这一项单独拿出来，因为它是「数据可不可信」的前置判断。
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub baseline_pct: f64,
    pub current_pct: f64,
    pub drop_pp: f64,
}

/// 整体判定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// 各指标都在阈值内。
    Healthy,
    /// 有指标越过阈值，值得开 PR 让人看。
    Drifted,
    /// 跑批本身不可信（抓取成功率骤降 / 目录条目骤减），本轮不应更新基线。
    Suspect { reason: String },
}

impl Verdict {
    /// CLI 退出码：0 正常、1 漂移、2 跑批异常。
    ///
    /// 分开给码是为了让 workflow 能区分「该开 PR」和「该判失败」——
    /// 两者都非零，但处置完全相反。
    pub fn exit_code(&self) -> i32 {
        match self {
            Verdict::Healthy => 0,
            Verdict::Drifted => 1,
            Verdict::Suspect { .. } => 2,
        }
    }
}

/// 对比报告。
#[derive(Debug, Clone)]
pub struct DriftReport {
    pub verdict: Verdict,
    pub health: Option<Health>,
    pub deltas: Vec<MetricDelta>,
    pub baseline_captured_at: Option<String>,
    pub current_captured_at: Option<String>,
}

impl DriftReport {
    /// 渲染成 Markdown。PR 正文与 `docs/audit/latest.md` 用同一份文本，
    /// 避免「PR 里说的」和「存档里的」对不上。
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        s.push_str("# 周度体检漂移报告\n\n");

        match &self.verdict {
            Verdict::Healthy => s.push_str("**判定：正常。** 各指标均在阈值内。\n\n"),
            Verdict::Drifted => s.push_str(
                "**判定：漂移。** 下表标 `漂移` 的指标越过了阈值，请确认是否接受为新常态。\n\n",
            ),
            Verdict::Suspect { reason } => s.push_str(&format!(
                "**判定：跑批不可信，未更新基线。** {reason}\n\n\
                 > 本轮数据不进入对照系。先确认是平台侧故障还是本项目抓取逻辑变化，\
                 修好后用 `workflow_dispatch` 重跑。\n\n"
            )),
        }

        if let Some(at) = &self.baseline_captured_at {
            s.push_str(&format!("- 基线采集日：{at}\n"));
        }
        if let Some(at) = &self.current_captured_at {
            s.push_str(&format!("- 本次采集日：{at}\n"));
        }
        if let Some(h) = &self.health {
            // 0 变化时 `-0.0` 会打印成 "-0.00pp"，这里显式归零
            let change_pp = if h.drop_pp.abs() < 0.005 {
                0.0
            } else {
                -h.drop_pp
            };
            s.push_str(&format!(
                "- 抓取成功率：{:.2}% → {:.2}%（{change_pp:+.2}pp）\n",
                h.baseline_pct, h.current_pct
            ));
        }
        s.push('\n');

        s.push_str("| 指标 | 基线 | 本次 | 变化 | 判定 |\n|---|---:|---:|---:|---|\n");
        for d in &self.deltas {
            s.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                d.name,
                fmt_value(d.baseline, d.unit),
                fmt_value(d.current, d.unit),
                fmt_delta(d.delta, d.unit),
                match d.level {
                    Level::Stable => "正常",
                    Level::Drift => "**漂移**",
                    Level::Info => "仅记录",
                }
            ));
        }

        s.push_str(
            "\n> 口径：比例类指标按占 `audited` 的百分比点数（pp）比，不比绝对条数；\
             `体检条目数` 只记录不告警，因为平台增删数据集是常态。\n\
             > 阈值是经验值，需按实测校准，见 `docs/audit/README.md`。\n",
        );
        s
    }
}

/// 对比两份 summary。
///
/// 判定顺序：先查跑批健康度（不可信则直接 [`Verdict::Suspect`]），
/// 再看各指标是否越过阈值。顺序不能反——不健康的跑批，任何指标都不作数。
///
/// 缺失字段（`None`）一律跳过，不参与对比，也不报错。
pub fn compare(baseline: &Summary, current: &Summary, th: &Thresholds) -> DriftReport {
    let health = match (
        ratio(baseline.with_preview, baseline.audited),
        ratio(current.with_preview, current.audited),
    ) {
        (Some(b), Some(c)) => Some(Health {
            baseline_pct: round2(b * 100.0),
            current_pct: round2(c * 100.0),
            drop_pp: round2((b - c) * 100.0),
        }),
        _ => None,
    };

    let mut suspect: Option<String> = None;

    if let Some(h) = &health {
        if h.drop_pp > th.preview_drop_pp {
            suspect = Some(format!(
                "抓取成功率 {:.2}% → {:.2}%（跌 {:.2}pp，阈值 {:.2}pp）",
                h.baseline_pct, h.current_pct, h.drop_pp, th.preview_drop_pp
            ));
        }
    }

    // 目录条目骤减：更可能是分页参数被忽略、接口改了返回结构，而不是数据真的少了。
    if let (Some(b), Some(c)) = (baseline.audited, current.audited) {
        if b > 0 && (c as f64) < b as f64 * (1.0 - th.audited_drop) {
            suspect.get_or_insert_with(|| {
                format!(
                    "目录条目 {b} → {c}（减少 {:.1}%，阈值 {:.0}%）",
                    (1.0 - c as f64 / b as f64) * 100.0,
                    th.audited_drop * 100.0
                )
            });
        }
    }

    let mut deltas: Vec<MetricDelta> = Vec::new();

    deltas.extend(ratio_delta(
        "字段一致性问题占比",
        baseline.schema_mismatch,
        current.schema_mismatch,
        baseline.audited,
        current.audited,
        th.ratio_pp,
    ));
    deltas.extend(ratio_delta(
        "停更占比",
        baseline.stopped_update,
        current.stopped_update,
        baseline.audited,
        current.audited,
        th.ratio_pp,
    ));
    deltas.extend(ratio_delta(
        "声称周期已逾期占比",
        baseline.claimed_cycle_overdue,
        current.claimed_cycle_overdue,
        baseline.audited,
        current.audited,
        th.ratio_pp,
    ));
    deltas.extend(ratio_delta(
        "整列为空的数据集占比",
        baseline.all_empty_column_datasets,
        current.all_empty_column_datasets,
        baseline.audited,
        current.audited,
        th.ratio_pp,
    ));
    deltas.extend(ratio_delta(
        "无列清单占比",
        baseline.no_column_list,
        current.no_column_list,
        baseline.audited,
        current.audited,
        th.ratio_pp,
    ));

    deltas.extend(value_delta(
        "空值率中位数",
        baseline.empty_rate_median.map(|v| v * 100.0),
        current.empty_rate_median.map(|v| v * 100.0),
        th.empty_rate * 100.0,
        Unit::Percent,
    ));
    deltas.extend(value_delta(
        "中位滞后天数",
        baseline.lag_median_days.map(|v| v as f64),
        current.lag_median_days.map(|v| v as f64),
        th.lag_days as f64,
        Unit::Days,
    ));

    // 只记录不告警：总量变化本身没有好坏，解释它需要人看新增/消失的是哪些数据集。
    if let (Some(b), Some(c)) = (baseline.audited, current.audited) {
        deltas.push(MetricDelta {
            name: "体检条目数",
            baseline: b as f64,
            current: c as f64,
            delta: c as f64 - b as f64,
            unit: Unit::Count,
            level: Level::Info,
        });
    }

    let verdict = match suspect {
        Some(reason) => Verdict::Suspect { reason },
        None if deltas.iter().any(|d| d.level == Level::Drift) => Verdict::Drifted,
        None => Verdict::Healthy,
    };

    DriftReport {
        verdict,
        health,
        deltas,
        baseline_captured_at: baseline.captured_at.clone(),
        current_captured_at: current.captured_at.clone(),
    }
}

fn ratio(num: Option<u64>, den: Option<u64>) -> Option<f64> {
    match (num, den) {
        (Some(n), Some(d)) if d > 0 => Some(n as f64 / d as f64),
        _ => None,
    }
}

fn ratio_delta(
    name: &'static str,
    b_num: Option<u64>,
    c_num: Option<u64>,
    b_den: Option<u64>,
    c_den: Option<u64>,
    limit_pp: f64,
) -> Option<MetricDelta> {
    let b = ratio(b_num, b_den)? * 100.0;
    let c = ratio(c_num, c_den)? * 100.0;
    let delta = c - b;
    Some(MetricDelta {
        name,
        baseline: round2(b),
        current: round2(c),
        delta: round2(delta),
        unit: Unit::Percent,
        level: if delta.abs() >= limit_pp {
            Level::Drift
        } else {
            Level::Stable
        },
    })
}

fn value_delta(
    name: &'static str,
    b: Option<f64>,
    c: Option<f64>,
    limit: f64,
    unit: Unit,
) -> Option<MetricDelta> {
    let (b, c) = (b?, c?);
    let delta = c - b;
    Some(MetricDelta {
        name,
        baseline: round2(b),
        current: round2(c),
        delta: round2(delta),
        unit,
        level: if delta.abs() >= limit {
            Level::Drift
        } else {
            Level::Stable
        },
    })
}

fn fmt_value(v: f64, unit: Unit) -> String {
    match unit {
        Unit::Percent => format!("{v:.2}%"),
        Unit::Days => format!("{v:.0} 天"),
        Unit::Count => format!("{:.0}", v),
    }
}

fn fmt_delta(v: f64, unit: Unit) -> String {
    match unit {
        Unit::Percent => format!("{v:+.2}pp"),
        Unit::Days => format!("{v:+.0} 天"),
        Unit::Count => format!("{v:+.0}"),
    }
}

fn round2(x: f64) -> f64 {
    let r = (x * 100.0).round() / 100.0;
    // `-0.0` 会让报告里出现 "-0.00pp"，归一成 0.0
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 基准场景：24,322 条，抓取成功率约 81.5%。
    fn base() -> Summary {
        Summary {
            captured_at: Some("2026-11-18".into()),
            audited: Some(24_322),
            with_preview: Some(19_830),
            schema_mismatch: Some(1_284),
            stopped_update: Some(8_843),
            claimed_cycle_overdue: Some(2_940),
            all_empty_column_datasets: Some(3_976),
            no_column_list: Some(129),
            empty_rate_median: Some(0.0),
            empty_rate_p90: Some(0.2),
            lag_median_days: Some(115),
            ..Default::default()
        }
    }

    #[test]
    fn identical_summary_is_healthy() {
        let r = compare(&base(), &base(), &Thresholds::default());
        assert_eq!(r.verdict, Verdict::Healthy);
        assert_eq!(r.verdict.exit_code(), 0);
    }

    #[test]
    fn ratio_jump_is_drift() {
        let mut now = base();
        // 字段一致性问题从 5.28% 涨到 8.5%，越 2pp 阈值
        now.schema_mismatch = Some(2_067);
        let r = compare(&base(), &now, &Thresholds::default());
        assert_eq!(r.verdict, Verdict::Drifted);
        assert_eq!(r.verdict.exit_code(), 1);
        let d = r
            .deltas
            .iter()
            .find(|d| d.name == "字段一致性问题占比")
            .unwrap();
        assert_eq!(d.level, Level::Drift);
        assert!(d.delta > 2.0, "{:?}", d);
    }

    #[test]
    fn lag_days_is_drift() {
        let mut now = base();
        now.lag_median_days = Some(140);
        let r = compare(&base(), &now, &Thresholds::default());
        assert_eq!(r.verdict, Verdict::Drifted);
    }

    #[test]
    fn preview_drop_beats_everything() {
        let mut now = base();
        // 抓取成功率从 81.5% 掉到 50%，同时指标看起来「变好」了
        now.with_preview = Some(12_161);
        now.schema_mismatch = Some(10);
        let r = compare(&base(), &now, &Thresholds::default());
        match r.verdict {
            Verdict::Suspect { .. } => {}
            other => panic!("期望 Suspect，得到 {other:?}"),
        }
        assert_eq!(r.verdict.exit_code(), 2);
    }

    #[test]
    fn catalog_shrink_is_suspect() {
        let mut now = base();
        now.audited = Some(9_000);
        now.with_preview = Some(7_400);
        let r = compare(&base(), &now, &Thresholds::default());
        assert!(
            matches!(r.verdict, Verdict::Suspect { .. }),
            "{:?}",
            r.verdict
        );
    }

    #[test]
    fn missing_fields_do_not_invent_drift() {
        let bare = Summary::default();
        let r = compare(&bare, &base(), &Thresholds::default());
        assert_eq!(r.verdict, Verdict::Healthy);
        assert!(r.health.is_none());
        // 缺字段的指标整行跳过，不留 0 值假象
        assert!(r.deltas.iter().all(|d| d.name != "中位滞后天数"));
    }

    #[test]
    fn audited_change_is_info_only() {
        let mut now = base();
        now.audited = Some(25_000);
        now.with_preview = Some(20_400);
        let r = compare(&base(), &now, &Thresholds::default());
        let d = r.deltas.iter().find(|d| d.name == "体检条目数").unwrap();
        assert_eq!(d.level, Level::Info);
        assert_eq!(r.verdict, Verdict::Healthy, "总量变化不该告警");
    }

    #[test]
    fn parses_summary_wrapped_and_raw() {
        let raw: Value = serde_json::json!({
            "audited": 100, "withPreview": 80,
            "byDept": {"某厅": {"n": 1}},
            "updateCycle": {"365": 3},
        });
        let s = Summary::from_value(raw).unwrap();
        assert_eq!(s.audited, Some(100));
        assert_eq!(s.with_preview, Some(80));

        let wrapped: Value = serde_json::json!({
            "summary": {"audited": 7}, "something": 1
        });
        assert_eq!(Summary::from_value(wrapped).unwrap().audited, Some(7));
    }

    #[test]
    fn baseline_text_omits_unknown_fields() {
        let text = base().to_baseline_text().unwrap();
        assert!(text.contains("\"audited\": 24322"));
        assert!(!text.contains("null"), "未知字段应省略而不是写 null");
        assert!(!text.contains("byDept"));
    }

    #[test]
    fn markdown_carries_verdict_and_rows() {
        let mut now = base();
        now.schema_mismatch = Some(2_067);
        let md = compare(&base(), &now, &Thresholds::default()).to_markdown();
        assert!(md.contains("判定：漂移"));
        assert!(md.contains("字段一致性问题占比"));
        assert!(md.contains("**漂移**"));
    }

    #[test]
    fn suspect_markdown_says_baseline_untouched() {
        let mut now = base();
        now.with_preview = Some(1_000);
        let md = compare(&base(), &now, &Thresholds::default()).to_markdown();
        assert!(md.contains("未更新基线"));
    }
}
