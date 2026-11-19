//! 跑批历史：把每次体检的几个标量指标记成一条，攒成曲线。
//!
//! **为什么单独存一份日志**：`summary.json` 每次跑批被覆盖，`data.json` 也一样。
//! 「指标是变好了还是变坏了」需要跨跑批比较，而跨跑批的信息一旦只活在人脑里，
//! 下一次就没人说得清——这正是 `drift` 存在的原因，但 `drift` 只比「这一轮 vs 基线」，
//! 曲线要的是「每一轮」。
//!
//! **为什么不进数据库**：站点是静态的，站点数据进 git 就能被任何人 checkout 出来
//! 复核。一条记录几十字节，五年周报也就 260 条、十几 KB。
//!
//! **为什么不记问题分**：分数是本项目的尺子，权重一改，历史分数就不可比。
//! 只记**不随权重变**的原始计数（命中数 / 总量），曲线才经得起「你们是不是调了权重」这一问。

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 保留多少条。五年周报（260 周）足够，超出丢最旧的。
pub const MAX_POINTS: usize = 260;

/// 一次跑批的指标快照。字段都是原始计数，不含派生分。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    /// `YYYY-MM-DD`，来自跑批的 `run.json`，不是「今天」——补跑旧快照时两者不同
    pub date: String,
    pub audited: u64,
    pub with_preview: u64,
    /// 超过 90 天未更新
    pub lag_over90: u64,
    /// 声明列数与样例列数不符
    pub mismatch: u64,
    /// 存在整列全空
    pub all_empty: u64,
    /// 平台自标停更
    pub stopped: u64,
    /// 样例空值率 > 30%
    pub empty_over30: u64,
}

impl Point {
    /// 从 `summary.json` 取一条。
    ///
    /// 任何必需字段缺失就返回 `None`：**宁可不记，也不记 0**。
    /// 缺字段时补 0 会在曲线上画出一个「问题降到 0」的假好消息，
    /// 而它实际含义是「这个数我们不知道」。
    pub fn from_summary(date: &str, summary: &Value) -> Option<Self> {
        if !is_iso_date(date) {
            return None;
        }
        Some(Self {
            date: date.to_string(),
            audited: num(summary, "audited")?,
            with_preview: num(summary, "withPreview")?,
            lag_over90: num(summary, "lagOver90Days")?,
            mismatch: num(summary, "schemaMismatch")?,
            all_empty: num(summary, "allEmptyColumnDatasets")?,
            stopped: num(summary, "stoppedUpdate")?,
            empty_over30: num(summary, "emptyRateOver30")?,
        })
    }

    /// 这一点的「滞后占比」，给曲线用。分母为 0 时返回 `None`（不是 0）。
    pub fn lag_ratio(&self) -> Option<f64> {
        (self.audited > 0).then(|| self.lag_over90 as f64 / self.audited as f64)
    }

    /// 与上一点相比的变化量（`self - prev`）。同一日期返回 `None`。
    pub fn delta(&self, prev: &Point) -> Option<i64> {
        (self.date != prev.date).then(|| self.lag_over90 as i64 - prev.lag_over90 as i64)
    }
}

/// 合并一条新记录：同日期**替换**（不是追加），按日期升序，只留最近 `MAX_POINTS` 条。
///
/// 同日期替换是因为周报可能手动重跑：重跑应当修正当天的数，而不是在图上画两个点。
/// 排序按字符串比较即可——`YYYY-MM-DD` 的字典序等于时间序。
pub fn merge(mut existing: Vec<Point>, new: Point) -> Vec<Point> {
    existing.retain(|p| p.date != new.date);
    existing.push(new);
    existing.sort_by(|a, b| a.date.cmp(&b.date));
    if existing.len() > MAX_POINTS {
        let drop = existing.len() - MAX_POINTS;
        existing.drain(..drop);
    }
    existing
}

/// 读日志。文件不存在当作「还没开始攒」，返回空；文件坏了**报错**——
/// 坏文件静默变空等于把历史删了，而且下一次写回会真的把它删掉。
pub fn read_log(path: &str) -> Result<Vec<Point>> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::Io(e)),
    };
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let points: Vec<Point> = serde_json::from_str(&text)
        .map_err(|e| Error::Other(format!("历史日志 {path} 解析失败：{e}")))?;
    Ok(points)
}

/// 写日志：排序后的定长数组，缩进 2。格式固定，diff 才读得懂。
pub fn write_log(path: &str, points: &[Point]) -> Result<()> {
    if let Some(dir) = std::path::Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    std::fs::write(path, serde_json::to_string_pretty(points)? + "\n")?;
    Ok(())
}

/// 追加一条并写回，返回合并后的全量（方便调用方打印「共 N 点」）。
pub fn append(path: &str, point: Point) -> Result<Vec<Point>> {
    let merged = merge(read_log(path)?, point);
    write_log(path, &merged)?;
    Ok(merged)
}

fn num(v: &Value, key: &str) -> Option<u64> {
    v.get(key).and_then(Value::as_u64)
}

/// `YYYY-MM-DD` 且月日在范围内。只做形状校验：历史日志里的日期是人读的，
/// 但错的日期（`2026-13-40`）会让曲线排序错乱，所以在入口挡住。
pub fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 {
        return false;
    }
    let shaped = b.iter().enumerate().all(|(i, c)| match i {
        4 | 7 => *c == b'-',
        _ => c.is_ascii_digit(),
    });
    if !shaped {
        return false;
    }
    let month: u32 = s[5..7].parse().unwrap_or(0);
    let day: u32 = s[8..10].parse().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn summary() -> Value {
        json!({
            "audited": 24322,
            "withPreview": 19830,
            "lagOver90Days": 13665,
            "schemaMismatch": 1284,
            "allEmptyColumnDatasets": 3976,
            "stoppedUpdate": 8843,
            "emptyRateOver30": 1097,
        })
    }

    fn point(date: &str, lag: u64) -> Point {
        Point {
            date: date.into(),
            audited: 100,
            with_preview: 90,
            lag_over90: lag,
            mismatch: 1,
            all_empty: 2,
            stopped: 3,
            empty_over30: 4,
        }
    }

    #[test]
    fn reads_all_seven_metrics() {
        let p = Point::from_summary("2026-11-14", &summary()).expect("应当能读");
        assert_eq!(p.audited, 24322);
        assert_eq!(p.with_preview, 19830);
        assert_eq!(p.lag_over90, 13665);
        assert_eq!(p.mismatch, 1284);
        assert_eq!(p.all_empty, 3976);
        assert_eq!(p.stopped, 8843);
        assert_eq!(p.empty_over30, 1097);
    }

    #[test]
    fn missing_metric_is_none_not_zero() {
        // 少一个字段就整条不记：补 0 会在曲线上画出假的「降到 0」
        let mut s = summary();
        s.as_object_mut().unwrap().remove("schemaMismatch");
        assert!(Point::from_summary("2026-11-14", &s).is_none());
    }

    #[test]
    fn bad_date_is_rejected() {
        assert!(Point::from_summary("2026-11", &summary()).is_none());
        assert!(Point::from_summary("2026-13-01", &summary()).is_none());
        assert!(Point::from_summary("2026-11-32", &summary()).is_none());
        assert!(Point::from_summary("14/11/2026", &summary()).is_none());
    }

    #[test]
    fn valid_dates_pass() {
        for d in ["2026-11-14", "2026-01-31", "2025-12-01"] {
            assert!(is_iso_date(d), "{d} 应当通过");
        }
    }

    #[test]
    fn merge_sorts_and_dedupes_same_date() {
        let points = merge(vec![point("2026-11-28", 3)], point("2026-11-14", 1));
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].date, "2026-11-14", "应当按日期升序");

        // 同一天重跑：替换，不是追加
        let points = merge(points, point("2026-11-28", 9));
        assert_eq!(points.len(), 2, "同日期必须替换");
        assert_eq!(points[1].lag_over90, 9, "取新值");
    }

    #[test]
    fn merge_caps_at_max_points_keeping_newest() {
        let mut all: Vec<Point> = (0..MAX_POINTS + 5)
            .map(|i| point(&format!("2025-01-{:02}", i % 28 + 1), i as u64))
            .collect();
        all.sort_by(|a, b| a.date.cmp(&b.date));
        let merged = merge(all, point("2026-12-07", 42));
        assert_eq!(merged.len(), MAX_POINTS);
        assert_eq!(merged.last().unwrap().date, "2026-12-07", "最新的必须在");
    }

    #[test]
    fn ratio_is_none_when_nothing_audited() {
        let mut p = point("2026-11-14", 10);
        assert_eq!(p.lag_ratio(), Some(0.1));
        p.audited = 0;
        assert_eq!(p.lag_ratio(), None, "分母为 0 是「不知道」，不是 0");
    }

    #[test]
    fn delta_compares_two_dates() {
        let a = point("2026-11-14", 10);
        let b = point("2026-11-21", 7);
        assert_eq!(b.delta(&a), Some(-3));
        assert_eq!(a.delta(&a), None, "同一天不比");
    }

    #[test]
    fn read_missing_file_is_empty_not_error() {
        let dir = std::env::temp_dir().join(format!("assay-history-{}", std::process::id()));
        let path = dir.join("nope.json");
        assert!(read_log(path.to_str().unwrap()).unwrap().is_empty());
    }

    #[test]
    fn corrupt_log_is_an_error() {
        let dir = std::env::temp_dir().join(format!("assay-history-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("summary.json");
        std::fs::write(&path, "{ not json").unwrap();
        let err = read_log(path.to_str().unwrap()).unwrap_err().to_string();
        assert!(err.contains("解析失败"), "错误要说清是哪个文件坏了：{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_round_trips_and_is_idempotent_per_date() {
        let dir = std::env::temp_dir().join(format!("assay-history-rt-{}", std::process::id()));
        let path = dir.join("summary.json");
        let p = path.to_str().unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        let one = append(p, point("2026-11-14", 5)).unwrap();
        assert_eq!(one.len(), 1);

        let two = append(p, point("2026-11-21", 6)).unwrap();
        assert_eq!(two.len(), 2);

        // 同一天再跑：条数不变，值更新
        let again = append(p, point("2026-11-21", 7)).unwrap();
        assert_eq!(again.len(), 2);
        assert_eq!(read_log(p).unwrap()[1].lag_over90, 7);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
