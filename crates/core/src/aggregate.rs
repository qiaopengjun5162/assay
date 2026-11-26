//! 聚合：items.jsonl + 目录快照 → summary.json / leaderboard.md / leaderboard_dept.md。
//!
//! 所有口径与旧版 `scripts/aggregate.py` 一致；对比测试直接拿真实产物核对数字。

use crate::model::{CatalogRecord, Dataset, DeptStat};
use crate::scoring::{classify, score};
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

/// 部门统计的 JSON 形状。**这是 `web/data.json` 里 `byDept` 的公开契约**。
///
/// `emptyAvg` 是派生值（`empty_sum / empty_n`），`DeptStat` 上那个同义字段标了
/// `skip_serializing`，所以只能在这里手算并插入。其余多词键（`all_empty` 等）
/// 仍是 snake_case：前端 `DeptRow` 只读 `n / stopped / mismatch / emptyAvg`，
/// 没人读的键不值得为风格统一去改结构体。
///
/// 公开是为了让 `tests/artifact_parity.rs` 能拿它跟**入库的** `web/data.json`
/// 对键名——那次事故是「代码改了 key，产物没重跑，前端加 alias 兜住」，
/// 于是发布的契约与代码的行为长期不是一回事，而没有任何检查会红。
///
/// # Errors
///
/// `DeptStat` 序列化失败时向上抛（字段都是标量，实际不会发生）。
pub fn dept_object(v: &DeptStat) -> Result<Value> {
    let mut obj = serde_json::to_value(v)?;
    if let Some(m) = obj.as_object_mut() {
        let avg = if v.empty_n > 0 {
            Some(round4(v.empty_sum / v.empty_n as f64))
        } else {
            None
        };
        m.insert("emptyAvg".into(), json!(avg));
    }
    Ok(obj)
}

/// 榜单标题。
///
/// 浙江是本项目的原生场景，标题沿用既有叫法（页面、文档、路演稿都对着它）。
/// 其他数据源**不许顶这个地名**：用 qunmind 的新闻条目跑一遍，榜首却印着
/// 「浙江开放数据健康榜」，读者第一眼就被误导。缺平台信息时用中性标题，不猜。
fn board_title(platform: Option<&str>) -> String {
    match platform {
        Some(p) if p.contains("浙江") => "浙江开放数据健康榜（问题 Top 50）".to_string(),
        Some(p) => format!("{p} · 数据体检榜（问题 Top 50）"),
        None => "数据体检健康榜（问题 Top 50）".to_string(),
    }
}

/// 榜单「说明」单元格：字数，不足阈值时加标记。
///
/// 只印字数不标「不足」，读者无法解释「这一行声明列、样例列、空值率、全空列全都正常，
/// 凭什么排在问题榜第一」。阈值取自 `checks::DESC_MIN`，与检查项同源，不另写字面量。
fn desc_cell(len: usize) -> String {
    if len < crate::checks::DESC_MIN {
        format!("{len}（不足）")
    } else {
        len.to_string()
    }
}

pub fn aggregate(items_path: &str, catalog_path: &str, out_dir: &str) -> Result<Value> {
    let items = crate::audit::read_items(items_path)?;
    let catalog = load_catalog(catalog_path)?;
    let by_id: HashMap<&str, &CatalogRecord> = catalog.iter().map(|r| (r.id.as_str(), r)).collect();

    // 榜单标题里的数据源名：`audit` 把它写进同目录的 `run.json`。
    // 不能写死「浙江」——换个源（比如公众号日报条目）跑出来的榜会顶着浙江地名，
    // 读者第一眼就被误导。缺这个字段的老批次退回中性标题，不猜。
    let platform = fs::read_to_string(Path::new(out_dir).join("run.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v["platform"].as_str().map(str::to_string));

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
        // 「逾期」的判定只有 `checks::Facts::overdue` 一处实现，
        // 这里不再自己写 `d > limit`——两处各写一遍就是下一个分叉点。
        if crate::checks::Facts::with_catalog(it, Some(&ut)).overdue() {
            overdue += 1;
        }
    }

    let n = items.len();
    let with_preview: Vec<&Dataset> = items.iter().filter(|i| i.sample_rows > 0).collect();
    let rates: Vec<f64> = with_preview.iter().map(|i| i.empty_rate).collect();
    // 「结构对不上」的口径只有 `checks::schema_mismatch` 一处实现。
    // 原先这里手写三选一、`SchemaKind::is_mismatch()` 里手写两选一，
    // 两个数（1,284 / 1,256）谁先被用谁就错。
    let mismatch: Vec<&Dataset> = items
        .iter()
        .filter(|i| crate::checks::schema_mismatch(&crate::checks::Facts::of(i)))
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
        if crate::checks::schema_mismatch(&crate::checks::Facts::of(i)) {
            s.mismatch += 1;
        }
        if i.desc_len < crate::checks::DESC_MIN {
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
        by_dept.insert((*k).clone(), dept_object(v)?);
    }

    let summary = json!({
        "audited": n,
        "withPreview": with_preview.len(),
        "emptyRateMean": (!rates.is_empty()).then(|| round4(rates.iter().sum::<f64>() / rates.len() as f64)),
        "emptyRateMedian": empty_median.map(round4),
        "emptyRateP90": p90,
        "emptyRateOver30": rates.iter().filter(|r| **r > crate::checks::EMPTY_RATE_HIGH).count(),
        "allEmptyColumnDatasets": items.iter().filter(|i| i.all_empty_cols > 0).count(),
        "schemaMismatch": mismatch.len(),
        "mismatchKinds": json_object(kinds.most_common(usize::MAX)),
        "noDescription": items.iter().filter(|i| i.desc_len < crate::checks::DESC_MIN).count(),
        "noColumnList": items.iter().filter(|i| i.declared_cols == 0).count(),
        "stoppedUpdate": stopped.len(),
        "lagMedianDays": median(lags.iter().map(|d| *d as f64).collect()).map(|m| m as i64),
        "lagOver90Days": lags.iter().filter(|d| **d > crate::checks::LAG_WARN_DAYS).count(),
        "lagOver365Days": lags.iter().filter(|d| **d > crate::checks::LAG_LONG_DAYS).count(),
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
        format!("# {}", board_title(platform.as_deref())),
        String::new(),
        format!(
            "体检范围：{n} 个数据集；有样例：{}；样例级平均空值率：{:.1}%；字段不一致：{}；停更标注：{}；超 90 天未更新：{}",
            with_preview.len(),
            summary["emptyRateMean"].as_f64().unwrap_or(0.0) * 100.0,
            mismatch.len(),
            stopped.len(),
            // 时效性不进分数（`overdue-claim` 权重 0），但读者必须看得见——
            // 新闻批次里「一年前的旧闻混进今天的日报」比「摘要太短」严重得多，
            // 只把它留在 summary.json 里等于没报。
            summary["lagOver90Days"].as_u64().unwrap_or(0)
        ),
        String::new(),
        "| # | 数据集 | 部门 | 声明列 | 样例列 | 类型 | 空值率 | 全空列 | 说明 | 停更 |"
            .to_string(),
        "|---|---|---|---|---|---|---|---|---|---|".to_string(),
    ];
    for (k, i) in ranked.iter().take(50).enumerate() {
        md.push(format!(
            "| {} | {} | {} | {} | {} | {} | {:.1}% | {} | {} | {} |",
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
            desc_cell(i.desc_len),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 榜单标题不能把「浙江」硬编码到别的数据源头上。
    ///
    /// 用 qunmind 的新闻条目跑一遍，榜首印着「浙江开放数据健康榜」——数据是新闻，
    /// 地名却是浙江。这是最容易被忽略的误导：标题没人当数据处理，也就没人校验。
    #[test]
    fn board_title_never_claims_zhejiang_for_other_platforms() {
        assert_eq!(
            board_title(Some("浙江省公共数据开放平台")),
            "浙江开放数据健康榜（问题 Top 50）",
            "浙江场景沿用既有叫法"
        );
        assert_eq!(
            board_title(Some("公众号日报（qunmind 条目）")),
            "公众号日报（qunmind 条目） · 数据体检榜（问题 Top 50）"
        );
        assert!(
            !board_title(Some("公众号日报（qunmind 条目）")).contains("浙江"),
            "非浙江数据源不许出现浙江地名"
        );
        assert_eq!(
            board_title(None),
            "数据体检健康榜（问题 Top 50）",
            "缺少平台信息时用中性标题，不猜"
        );
    }

    /// 「说明」列必须能让读者看出这一行为什么上榜。
    ///
    /// 新闻条目批次里 49/59 命中「说明不足」，但榜单其余列全是 0——没有这一列，
    /// 读者只会看到一片「一致 / 0.0% / 0」挤在问题榜前面。
    #[test]
    fn desc_cell_marks_short_descriptions() {
        let min = crate::checks::DESC_MIN;
        assert_eq!(desc_cell(0), "0（不足）");
        assert_eq!(desc_cell(min - 1), format!("{}（不足）", min - 1));
        assert_eq!(desc_cell(min), min.to_string(), "恰好达到阈值不算不足");
        assert_eq!(desc_cell(120), "120");
    }

    /// 部门维度的 key 必须是前端认得的 camelCase。
    ///
    /// 这个 bug 在页面上表现为「平均空值率全是 0.0%」——不报错、不崩，
    /// 只是安静地显示成 0，比崩溃更难发现。所以钉成测试。
    #[test]
    fn dept_object_uses_camel_case_keys() {
        let v = DeptStat {
            n: 4,
            empty_sum: 0.5,
            empty_n: 2,
            ..Default::default()
        };
        let obj = dept_object(&v).unwrap();

        assert_eq!(obj["emptyAvg"], json!(0.25), "空值率应按 camelCase 给出");
        assert!(
            obj.get("empty_avg").is_none(),
            "snake_case key 会让前端静默读成 0：{obj}"
        );
        // 单词类字段（n / stopped / mismatch）大小写一致，两边都读得到
        assert_eq!(obj["n"], json!(4));
    }

    /// 部门里一条样例都没有时，平均空值率是「不知道」（null），不是 0。
    #[test]
    fn dept_object_reports_null_when_no_samples() {
        let v = DeptStat {
            n: 3,
            empty_n: 0,
            ..Default::default()
        };
        assert_eq!(dept_object(&v).unwrap()["emptyAvg"], Value::Null);
    }
}
