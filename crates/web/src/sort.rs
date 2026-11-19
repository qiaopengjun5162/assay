//! 榜单行排序。
//!
//! 刻意从组件里抽出来：这是前端唯一一处「比较浮点数」的地方，而浮点比较
//! 恰好是能让**整页白屏**的地方（见下面 `desc` 的注释）。抽成纯函数才能在
//! native 下用单测盯住它，而不是等评委在演示现场点出一个白屏。

use crate::model::Row;
use std::cmp::Ordering;

/// 按 `<select>` 的 `value` 排序。空串以外的未知值一律按分数排——
/// 宁可排错顺序，也不让页面空着。
pub fn sort_rows(rows: &mut [Row], mode: &str) {
    match mode {
        "empty" => rows.sort_by(|a, b| desc(a.empty_rate, b.empty_rate)),
        "lag" => rows.sort_by_key(|r| std::cmp::Reverse(r.lag_days.unwrap_or(-1))),
        _ => rows.sort_by(|a, b| desc(a.score, b.score)),
    }
}

/// 降序比较。
///
/// 用 `total_cmp` 而不是 `partial_cmp().unwrap()`：后者遇到 NaN 会 panic，
/// 在 WASM 里就是整页白屏（不是某个区块降级）。数据里理论上进不了 NaN
/// ——`serde_json` 不接受 NaN 字面量——但这一层的代价是零，没有理由留一条
/// 白屏路径。
///
/// 但 `total_cmp` 单独用还不够：它把 +NaN 排在 +inf **之后**，降序就会让 NaN
/// 抢到榜首。NaN 的含义是「不知道」，不是「最差」——和 `lag_days` 缺失、
/// 部门空值率缺失一样，不能因为不知道就判它第一。所以这里显式把 NaN 压到最后。
fn desc(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => b.total_cmp(&a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, score: f64, empty_rate: f64, lag_days: Option<i64>) -> Row {
        Row {
            id: id.into(),
            score,
            empty_rate,
            lag_days,
            ..Default::default()
        }
    }

    fn ids(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.id.as_str()).collect()
    }

    #[test]
    fn score_desc_is_the_default_mode() {
        let mut v = vec![row("a", 3.0, 0.0, None), row("b", 9.0, 0.0, None)];
        sort_rows(&mut v, "score");
        assert_eq!(ids(&v), ["b", "a"], "下拉框的 value 就是 \"score\"");
        // 未知值不许把页面排成空：仍按分数给出一个顺序
        sort_rows(&mut v, "unknown-mode");
        assert_eq!(ids(&v), ["b", "a"]);
    }

    #[test]
    fn empty_rate_desc() {
        let mut v = vec![row("a", 1.0, 0.10, None), row("b", 9.0, 0.90, None)];
        sort_rows(&mut v, "empty");
        assert_eq!(ids(&v), ["b", "a"], "空值率降序，与分数无关");
    }

    #[test]
    fn lag_desc_puts_unknown_last() {
        let mut v = vec![
            row("none", 1.0, 0.0, None),
            row("old", 1.0, 0.0, Some(800)),
            row("new", 1.0, 0.0, Some(3)),
        ];
        sort_rows(&mut v, "lag");
        assert_eq!(
            ids(&v),
            ["old", "new", "none"],
            "没有更新时间的排最后，不能当成最新"
        );
    }

    /// 回归测试：这行以前是 `partial_cmp().unwrap()`，NaN 会 panic，
    /// 在浏览器里等于整页白屏。改成 `total_cmp` 之后必须不 panic 且顺序稳定。
    #[test]
    fn nan_sorts_last_instead_of_panicking() {
        let mut v = vec![
            row("nan", f64::NAN, f64::NAN, None),
            row("ok", 5.0, 0.5, None),
        ];
        sort_rows(&mut v, "");
        assert_eq!(ids(&v), ["ok", "nan"], "NaN 排最后，而不是炸掉整个页面");

        sort_rows(&mut v, "empty");
        assert_eq!(ids(&v), ["ok", "nan"]);
    }

    /// 同分时的顺序必须稳定：首屏第一张卡就是榜首，同分若随构建顺序漂移，
    /// 讲稿里「这张卡」和实际页面对不上。
    #[test]
    fn ties_keep_input_order() {
        let mut v = vec![
            row("first", 7.0, 0.0, None),
            row("second", 7.0, 0.0, None),
            row("third", 7.0, 0.0, None),
        ];
        sort_rows(&mut v, "");
        assert_eq!(ids(&v), ["first", "second", "third"]);
    }
}
