//! 聚合：items.jsonl + 目录快照 → summary.json / leaderboard.md / leaderboard_dept.md。
//!
//! 所有口径与旧版 `scripts/aggregate.py` 一致；对比测试直接拿真实产物核对数字。

use crate::model::{CatalogRecord, Dataset, DeptStat};
use crate::scoring::{classify, score, SchemaKind};
use crate::Result;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 报告的基准日。写成常量而不是 `now()`：数字必须能复现。
pub const NOW_YMD: (i64, i64, i64) = (2026, 11, 14);

/// 声称的更新频率 → 允许的最大滞后天数。
pub const BAD_CYCLE: [(&str, i64); 5] =
    [("1", 7), ("7", 10), ("30", 45), ("365", 400), ("每年", 400)];

pub fn cycle_threshold(cycle: &str) -> Option<i64> {
    BAD_CYCLE
        .iter()
        .find(|(c, _)| *c == cycle)
        .map(|(_, days)| *days)
}

/// 两个日期之间的天数（a - b），按公历计算，不引入时间库。
pub fn days_between(a: (i64, i64, i64), b: (i64, i64, i64)) -> i64 {
    days_from_civil(a.0, a.1, a.2) - days_from_civil(b.0, b.1, b.2)
}

/// Howard Hinnant 的公历→纪元日算法。
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// 解析 `2025-06-09 08:27:49`（也接受只有日期）。
pub fn parse_update_time(s: &str) -> Option<(i64, i64, i64)> {
    if s.len() < 10 {
        return None;
    }
    let y: i64 = s.get(0..4)?.parse().ok()?;
    let m: i64 = s.get(5..7)?.parse().ok()?;
    let d: i64 = s.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // 时分秒也要合法：平台上存在 09:70:21 这种脏时间戳，
    // Python 的 strptime 会拒绝，Rust 这边必须同样拒绝，否则统计会多算一条。
    if let Some(t) = s.get(11..19) {
        let hh: u32 = t.get(0..2)?.parse().ok()?;
        let mm: u32 = t.get(3..5)?.parse().ok()?;
        let ss: u32 = t.get(6..8)?.parse().ok()?;
        if hh > 23 || mm > 59 || ss > 59 {
            return None;
        }
    }
    Some((y, m, d))
}

/// 与 Python `(datetime(2026,11,14) - dt).days` 同口径：
/// 基准是当天 00:00，带时分秒的时间戳会少算一天（向下取整）。
pub fn lag_days(update_time: &str) -> Option<i64> {
    let dt = parse_update_time(update_time)?;
    let has_time = update_time
        .get(11..19)
        .map(|t| t != "00:00:00")
        .unwrap_or(false);
    Some(days_between(NOW_YMD, dt) - i64::from(has_time))
}

pub fn load_catalog(path: &str) -> Result<Vec<CatalogRecord>> {
    if !Path::new(path).exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

/// 有序计数：并列时按首次出现顺序，和 Python Counter 的 most_common 一致。
#[derive(Default)]
struct OrderedCount {
    order: Vec<String>,
    counts: HashMap<String, usize>,
}

impl OrderedCount {
    fn add(&mut self, key: String) {
        if !self.counts.contains_key(&key) {
            self.order.push(key.clone());
        }
        *self.counts.entry(key).or_insert(0) += 1;
    }

    fn most_common(&self, n: usize) -> Vec<(String, usize)> {
        let mut v: Vec<(usize, &String)> = self.order.iter().enumerate().collect();
        v.sort_by(|a, b| self.counts[b.1].cmp(&self.counts[a.1]).then(a.0.cmp(&b.0)));
        v.into_iter()
            .take(n)
            .map(|(_, k)| (k.clone(), self.counts[k]))
            .collect()
    }
}

fn median(mut xs: Vec<f64>) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = xs.len();
    Some(if n % 2 == 1 {
        xs[n / 2]
    } else {
        (xs[n / 2 - 1] + xs[n / 2]) / 2.0
    })
}

fn round4(x: f64) -> f64 {
    crate::util::round_to(x, 4)
}

pub fn aggregate(items_path: &str, catalog_path: &str, out_dir: &str) -> Result<Value> {
    let items = crate::audit::read_items(items_path)?;
    let catalog = load_catalog(catalog_path)?;
    let by_id: HashMap<&str, &CatalogRecord> = catalog.iter().map(|r| (r.id.as_str(), r)).collect();

    // 时效性
    let mut lags: Vec<i64> = Vec::new();
    let mut overdue = 0usize;
    for it in &items {
        let Some(ut) = by_id
            .get(it.id.as_str())
            .and_then(|r| r.update_time.clone())
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let Some(d) = lag_days(&ut) else { continue };
        lags.push(d.max(0));
        if let Some(limit) = cycle_threshold(&it.cycle_str()) {
            if d > limit {
                overdue += 1;
            }
        }
    }

    let n = items.len();
    let with_preview: Vec<&Dataset> = items.iter().filter(|i| i.sample_rows > 0).collect();
    let rates: Vec<f64> = with_preview.iter().map(|i| i.empty_rate).collect();
    let mismatch: Vec<&Dataset> = items
        .iter()
        .filter(|i| {
            matches!(
                classify(i),
                SchemaKind::ColumnCount | SchemaKind::FieldSet | SchemaKind::NamingOnly
            )
        })
        .collect();
    let stopped: Vec<&Dataset> = items
        .iter()
        .filter(|i| crate::scoring::STOPPED_CYCLES.contains(&i.cycle_str().as_str()))
        .collect();

    let mut kinds = OrderedCount::default();
    for it in &items {
        kinds.add(classify(it).as_str().to_string());
    }

    let mut cycles = OrderedCount::default();
    for it in &items {
        cycles.add(it.cycle_str());
    }

    // 部门维度（保持首次出现顺序，再按 n 降序稳定排序）
    let mut dept_order: Vec<String> = Vec::new();
    let mut dept_map: HashMap<String, DeptStat> = HashMap::new();
    for i in &items {
        let name = i
            .dept
            .as_deref()
            .unwrap_or("未知")
            .rsplit('/')
            .next()
            .unwrap_or("未知")
            .to_string();
        if !dept_map.contains_key(&name) {
            dept_order.push(name.clone());
        }
        let s = dept_map.entry(name).or_default();
        s.n += 1;
        if i.sample_rows > 0 {
            s.preview += 1;
            s.empty_sum += i.empty_rate;
            s.empty_n += 1;
        }
        if i.all_empty_cols > 0 {
            s.all_empty += 1;
        }
        if matches!(
            classify(i),
            SchemaKind::ColumnCount | SchemaKind::FieldSet | SchemaKind::NamingOnly
        ) {
            s.mismatch += 1;
        }
        if i.desc_len < 10 {
            s.no_desc += 1;
        }
        if crate::scoring::STOPPED_CYCLES.contains(&i.cycle_str().as_str()) {
            s.stopped += 1;
        }
    }
    let mut dept_sorted: Vec<(&String, &DeptStat)> = dept_order
        .iter()
        .map(|k| (k, dept_map.get(k).unwrap()))
        .collect();
    dept_sorted.sort_by_key(|(_, s)| std::cmp::Reverse(s.n));

    let empty_median = median(rates.clone());
    let p90 = if rates.is_empty() {
        None
    } else {
        let mut sorted = rates.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(round4(sorted[(sorted.len() as f64 * 0.9) as usize]))
    };

    let mut by_dept = Map::new();
    for (k, v) in &dept_sorted {
        let mut obj = serde_json::to_value(v)?;
        if let Some(m) = obj.as_object_mut() {
            let avg = if v.empty_n > 0 {
                Some(round4(v.empty_sum / v.empty_n as f64))
            } else {
                None
            };
            m.insert("empty_avg".into(), json!(avg));
        }
        by_dept.insert((*k).clone(), obj);
    }

    let summary = json!({
        "audited": n,
        "withPreview": with_preview.len(),
        "emptyRateMean": (!rates.is_empty()).then(|| round4(rates.iter().sum::<f64>() / rates.len() as f64)),
        "emptyRateMedian": empty_median.map(round4),
        "emptyRateP90": p90,
        "emptyRateOver30": rates.iter().filter(|r| **r > 0.3).count(),
        "allEmptyColumnDatasets": items.iter().filter(|i| i.all_empty_cols > 0).count(),
        "schemaMismatch": mismatch.len(),
        "mismatchKinds": json_object(kinds.most_common(usize::MAX)),
        "noDescription": items.iter().filter(|i| i.desc_len < 10).count(),
        "noColumnList": items.iter().filter(|i| i.declared_cols == 0).count(),
        "stoppedUpdate": stopped.len(),
        "lagMedianDays": median(lags.iter().map(|d| *d as f64).collect()).map(|m| m as i64),
        "lagOver90Days": lags.iter().filter(|d| **d > 90).count(),
        "lagOver365Days": lags.iter().filter(|d| **d > 365).count(),
        "claimedCycleOverdue": overdue,
        "updateCycle": json_object(cycles.most_common(8)),
        "byDept": Value::Object(by_dept),
    });

    fs::create_dir_all(out_dir)?;
    fs::write(
        Path::new(out_dir).join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;

    // 榜单
    let mut ranked = items.clone();
    ranked.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut md = vec![
        "# 浙江开放数据健康榜（问题 Top 50）".to_string(),
        String::new(),
        format!(
            "体检范围：{n} 个数据集；有样例：{}；样例级平均空值率：{:.1}%；字段不一致：{}；停更标注：{}",
            with_preview.len(),
            summary["emptyRateMean"].as_f64().unwrap_or(0.0) * 100.0,
            mismatch.len(),
            stopped.len()
        ),
        String::new(),
        "| # | 数据集 | 部门 | 声明列 | 样例列 | 类型 | 空值率 | 全空列 | 停更 |".to_string(),
        "|---|---|---|---|---|---|---|---|---|".to_string(),
    ];
    for (k, i) in ranked.iter().take(50).enumerate() {
        md.push(format!(
            "| {} | {} | {} | {} | {} | {} | {:.1}% | {} | {} |",
            k + 1,
            truncate(&i.name, 28),
            i.dept
                .as_deref()
                .unwrap_or("")
                .rsplit('/')
                .next()
                .unwrap_or(""),
            i.declared_cols,
            i.sample_cols,
            classify(i).as_str(),
            i.empty_rate * 100.0,
            i.all_empty_cols,
            if crate::scoring::STOPPED_CYCLES.contains(&i.cycle_str().as_str()) {
                "是"
            } else {
                ""
            }
        ));
    }
    fs::write(
        Path::new(out_dir).join("leaderboard.md"),
        md.join("\n") + "\n",
    )?;

    // 部门榜
    let mut bl = vec![
        "# 部门榜（按数据集数）".to_string(),
        String::new(),
        "| 部门 | 数据集 | 有样例 | 平均空值率 | 全空列 | 字段不一致 | 说明不足 | 停更 |"
            .to_string(),
        "|---|---|---|---|---|---|---|---|".to_string(),
    ];
    for (d, s) in dept_sorted.iter().take(25) {
        let avg = if s.empty_n > 0 {
            format!("{:.1}%", s.empty_sum / s.empty_n as f64 * 100.0)
        } else {
            "—".to_string()
        };
        bl.push(format!(
            "| {d} | {} | {} | {avg} | {} | {} | {} | {} |",
            s.n, s.preview, s.all_empty, s.mismatch, s.no_desc, s.stopped
        ));
    }
    fs::write(
        Path::new(out_dir).join("leaderboard_dept.md"),
        bl.join("\n") + "\n",
    )?;

    Ok(summary)
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn json_object(entries: Vec<(String, usize)>) -> Value {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert(k, json!(v));
    }
    Value::Object(m)
}
