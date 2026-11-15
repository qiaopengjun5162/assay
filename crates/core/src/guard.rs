//! 数字护栏：模型输出里的每个数字都必须来自既定事实。
//!
//! 只做集合比对，不做语义判断——规则越简单，越容易被单测钉住。
//! 与旧版 `scripts/guard.py` 同一口径：比例同时接受 0.6667 与 66.67%，
//! 千分位归一，尾零去掉。

use crate::model::Dataset;
use std::collections::BTreeSet;

/// 从文本里抽出数字并归一化（去千分位、去尾零）。
pub fn extract(text: &str) -> BTreeSet<String> {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == ',') {
            i += 1;
        }
        // 小数点后必须还有数字才算小数
        if i + 1 < bytes.len() && bytes[i] == '.' && bytes[i + 1].is_ascii_digit() {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        let raw: String = bytes[start..i].iter().collect();
        out.insert(normalize(&raw));
    }
    out
}

fn normalize(raw: &str) -> String {
    let s = raw.replace(',', "");
    if let Some((int, frac)) = s.split_once('.') {
        let frac = frac.trim_end_matches('0');
        if frac.is_empty() {
            int.to_string()
        } else {
            format!("{int}.{frac}")
        }
    } else {
        s
    }
}

/// 把一份事实摊平成可比文本；0~1 之间的小数同时给出百分数写法。
pub fn facts_text(facts: &[(&str, String)]) -> String {
    facts
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 数据集事实：解释层能看到的东西，就是这些。
pub fn dataset_facts(
    d: &Dataset,
    lag_days: Option<i64>,
    dept: &str,
) -> Vec<(&'static str, String)> {
    let mut f: Vec<(&'static str, String)> = Vec::new();
    f.push(("数据集", d.name.clone()));
    if !dept.is_empty() {
        f.push(("部门", dept.to_string()));
    }
    f.push(("声明列数", d.declared_cols.to_string()));
    f.push(("实际样例列数", d.sample_cols.to_string()));
    f.push(("样例行数", d.sample_rows.to_string()));
    f.push(("样例空值率", format!("{}%", round2(d.empty_rate * 100.0))));
    f.push(("整列为空的列数", d.all_empty_cols.to_string()));
    f.push(("更新频率", cycle_label(&d.cycle_str())));
    f.push((
        "字段一致性",
        crate::scoring::classify(d).as_str().to_string(),
    ));
    f.push(("字段说明字数", d.desc_len.to_string()));
    if let Some(lag) = lag_days {
        f.push(("距上次更新天数", lag.to_string()));
    }
    if !d.missing_fields.is_empty() {
        f.push((
            "声明有但样例没有的字段",
            d.missing_fields
                .iter()
                .take(6)
                .cloned()
                .collect::<Vec<_>>()
                .join("、"),
        ));
    }
    if !d.extra_fields.is_empty() {
        f.push((
            "样例有但声明没有的字段",
            d.extra_fields
                .iter()
                .take(6)
                .cloned()
                .collect::<Vec<_>>()
                .join("、"),
        ));
    }
    f
}

pub fn cycle_label(cycle: &str) -> String {
    match cycle {
        "1" => "每日".into(),
        "7" => "每周".into(),
        "30" => "每月".into(),
        "365" => "每年".into(),
        "-1" | "不更新" | "999" => "停更".into(),
        "" => "未知".into(),
        other => other.to_string(),
    }
}

/// 返回 (是否合格, 越界数字)。全文最多引用 3 个数字。
pub fn check(text: &str, facts: &[(&str, String)], max_numbers: usize) -> (bool, Vec<String>) {
    let flat = facts_text(facts);
    let allowed = extract(&flat);
    let used = extract(text);
    let illegal: Vec<String> = used.difference(&allowed).cloned().collect();
    if !illegal.is_empty() {
        return (false, illegal);
    }
    if used.len() > max_numbers {
        return (false, used.iter().skip(max_numbers).cloned().collect());
    }
    (true, Vec::new())
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Vec<(&'static str, String)> {
        vec![
            ("声明列数", "4".into()),
            ("实际样例列数", "3".into()),
            ("样例空值率", "66.67%".into()),
            ("audited", "24322".into()),
        ]
    }

    #[test]
    fn normalizes_commas_and_trailing_zeros() {
        let got = extract("24,322 个数据集，6.70% 空值");
        assert!(got.contains("24322"));
        assert!(got.contains("6.7"));
    }

    #[test]
    fn percent_form_is_allowed() {
        let (ok, bad) = check("样例空值率 66.67%，属于偏高。", &facts(), 3);
        assert!(ok, "{bad:?}");
    }

    #[test]
    fn thousands_form_is_allowed() {
        let (ok, _) = check("共体检 24,322 个数据集。", &facts(), 3);
        assert!(ok);
    }

    #[test]
    fn fabricated_number_is_rejected() {
        let (ok, bad) = check("这份数据有 382 条记录，字段 4 个。", &facts(), 3);
        assert!(!ok);
        assert_eq!(bad, vec!["382".to_string()]);
    }

    #[test]
    fn too_many_numbers_is_rejected() {
        let (ok, bad) = check("4 个字段、3 列、66.67% 空值、24322 个数据集。", &facts(), 3);
        assert!(!ok, "{bad:?}");
    }

    #[test]
    fn text_without_numbers_passes() {
        let (ok, _) = check("这份数据长期未更新，字段说明也缺失。", &facts(), 3);
        assert!(ok);
    }
}
