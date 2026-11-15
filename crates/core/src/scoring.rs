//! 字段一致性分类与问题分。
//!
//! 口径与旧版 `scripts/scoring.py` 完全一致——重构期间用真实产物做对比测试，
//! 任何一处权重的改动都会让测试红掉。

use crate::model::Dataset;
use serde::{Deserialize, Serialize};

/// 停更标记：平台用 -1 / 999 / 不更新 三种写法表达同一件事。
pub const STOPPED_CYCLES: [&str; 3] = ["-1", "999", "不更新"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaKind {
    /// 列数与字段名都对得上
    Consistent,
    /// 声明列数与样例列数不同——真的少了或多了
    ColumnCount,
    /// 列数相同，但名字对不上且不是前缀关系
    FieldSet,
    /// 列数相同，声明名 = 前缀 + 样例名（部门前缀的命名习惯）
    NamingOnly,
    /// 平台没给样例数据
    NoSample,
}

impl SchemaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SchemaKind::Consistent => "一致",
            SchemaKind::ColumnCount => "列数不一致",
            SchemaKind::FieldSet => "字段集合不一致",
            SchemaKind::NamingOnly => "仅命名不一致",
            SchemaKind::NoSample => "无样例",
        }
    }

    /// 结构性问题（用于统计 schemaMismatch）。
    pub fn is_mismatch(self) -> bool {
        matches!(self, SchemaKind::ColumnCount | SchemaKind::FieldSet)
    }
}

/// 分类：先看有没有样例，再看列数，最后看名字是不是只是带前缀。
pub fn classify_schema(
    declared_cols: usize,
    sample_cols: usize,
    missing: &[String],
    extra: &[String],
) -> SchemaKind {
    if sample_cols == 0 {
        return SchemaKind::NoSample;
    }
    if declared_cols == sample_cols && missing.is_empty() && extra.is_empty() {
        return SchemaKind::Consistent;
    }
    if declared_cols != sample_cols {
        return SchemaKind::ColumnCount;
    }
    if !missing.is_empty() && missing.len() == extra.len() {
        let matched = missing
            .iter()
            .filter(|m| {
                extra
                    .iter()
                    .any(|e| m.ends_with(e.as_str()) || e.ends_with(m.as_str()))
            })
            .count();
        if matched == missing.len() {
            return SchemaKind::NamingOnly;
        }
    }
    SchemaKind::FieldSet
}

pub fn classify(d: &Dataset) -> SchemaKind {
    classify_schema(
        d.declared_cols,
        d.sample_cols,
        &d.missing_fields,
        &d.extra_fields,
    )
}

/// 问题分：只用于排序，不是科学指标。权重公开写在代码里。
pub fn score(d: &Dataset) -> f64 {
    let mut s = 0.0_f64;
    match classify(d) {
        SchemaKind::ColumnCount | SchemaKind::FieldSet => s += 3.0,
        SchemaKind::NamingOnly => s += 1.0,
        SchemaKind::Consistent | SchemaKind::NoSample => {}
    }
    if d.all_empty_cols > 0 {
        s += 2.0;
    }
    s += (d.empty_rate * 6.0).min(3.0);
    if STOPPED_CYCLES.contains(&d.cycle_str().as_str()) {
        s += 1.0;
    }
    if d.desc_len < 10 {
        s += 1.0;
    }
    if d.has_preview == 0 {
        s += 1.0;
    }
    round2(s)
}

/// 保留两位小数。Python 的 round 是「四舍六入五成双」，这里用同样的策略，
/// 保证同一份输入在两种实现下得到同一个分数。
pub fn round2(x: f64) -> f64 {
    crate::util::round_to(x, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ds(declared: usize, sample: usize, missing: &[&str], extra: &[&str]) -> Dataset {
        Dataset {
            declared_cols: declared,
            sample_cols: sample,
            missing_fields: missing.iter().map(|s| s.to_string()).collect(),
            extra_fields: extra.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn consistent_when_everything_matches() {
        assert_eq!(classify(&ds(4, 4, &[], &[])), SchemaKind::Consistent);
    }

    #[test]
    fn column_count_mismatch_wins_over_names() {
        assert_eq!(
            classify(&ds(9, 8, &["更新时间"], &[])),
            SchemaKind::ColumnCount
        );
    }

    #[test]
    fn prefix_naming_is_not_a_data_problem() {
        let k = classify(&ds(
            2,
            2,
            &["鹿西乡民宿房间数", "鹿西乡民宿床位数"],
            &["房间数", "床位数"],
        ));
        assert_eq!(k, SchemaKind::NamingOnly);
    }

    #[test]
    fn field_set_mismatch_is_structural() {
        assert_eq!(
            classify(&ds(1, 1, &["甲公司名称"], &["联系人电话"])),
            SchemaKind::FieldSet
        );
    }

    #[test]
    fn no_sample_is_its_own_bucket() {
        assert_eq!(classify(&ds(13, 0, &[], &[])), SchemaKind::NoSample);
    }

    #[test]
    fn score_is_capped_and_additive() {
        let mut d = ds(9, 8, &[], &[]);
        d.empty_rate = 0.9;
        d.all_empty_cols = 2;
        d.desc_len = 3;
        d.has_preview = 1;
        d.update_cycle = Some(serde_json::json!("-1"));
        // 3(列数) + 2(全空) + 3(空值上限) + 1(停更) + 1(说明太短)
        assert_eq!(score(&d), 10.0);
    }

    #[test]
    fn round2_matches_python_half_even() {
        assert_eq!(round2(2.5), 2.5);
        assert_eq!(round2(0.125), 0.12);
        assert_eq!(round2(0.135), 0.14);
    }
}
