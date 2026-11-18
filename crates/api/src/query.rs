//! 榜单查询：参数归一化 + SQL 片段白名单。
//!
//! 这里刻意**不**做动态拼接。排序键只有三种，用 `match` 映射到三条固定 SQL；
//! 过滤条件全部走可空参数（`$2 is null or ...`），一条 prepared statement 覆盖
//! 所有组合。这样做的理由很实际：动态拼 SQL 是这个项目里唯一可能被注入的地方，
//! 而它换来的灵活性这里并不需要。
//!
//! 因此本模块是纯函数，不碰数据库——CI 里能跑，不需要 Postgres。

use serde::Deserialize;

/// 默认每页条数。前端一屏约 20 行，留一倍余量给滚动预取。
const DEFAULT_SIZE: u32 = 50;
/// 单页上限。防的是「`size=100000` 把库拖死」，不是防前端。
const MAX_SIZE: u32 = 500;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    /// 问题分降序（默认）
    #[default]
    Score,
    /// 样例空值率降序
    Empty,
    /// 停更时长降序
    Lag,
}

impl SortKey {
    /// 排序片段。**这是白名单**：只有这三个返回值，不接受外部字符串。
    pub fn order_by(self) -> &'static str {
        match self {
            SortKey::Score => "o.score desc, d.ext_id",
            SortKey::Empty => "o.empty_rate desc, d.ext_id",
            SortKey::Lag => "o.lag_days desc nulls last, d.ext_id",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetQuery {
    /// 查哪一次 run；省略则取该平台最近一次
    #[serde(default)]
    pub run: Option<i64>,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub dept: Option<String>,
    #[serde(default)]
    pub sort: SortKey,
    /// 只看停更
    #[serde(default)]
    pub stopped: bool,
    /// 只看字段对不上
    #[serde(default)]
    pub mismatch: bool,
    /// 空值率下限；<=0 表示不过滤
    #[serde(default)]
    pub min_empty: f64,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub size: Option<u32>,
}

/// 归一化后的查询：空串当未传、页码与页大小夹到合法区间。
#[derive(Debug, Clone, PartialEq)]
pub struct Normalized {
    pub q: Option<String>,
    pub dept: Option<String>,
    pub sort: SortKey,
    pub stopped: bool,
    pub mismatch: bool,
    pub min_empty: f64,
    pub page: u32,
    pub size: u32,
}

impl DatasetQuery {
    pub fn normalize(self) -> Normalized {
        // 空白搜索词等同于没传：不然会退化成 `ilike '%%'`，白白扫一遍全表
        let clean = |s: Option<String>| s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        Normalized {
            q: clean(self.q),
            dept: clean(self.dept),
            sort: self.sort,
            stopped: self.stopped,
            mismatch: self.mismatch,
            // 负数与 NaN 都归零（不过滤）
            min_empty: if self.min_empty.is_finite() && self.min_empty > 0.0 {
                self.min_empty.min(1.0)
            } else {
                0.0
            },
            page: self.page.unwrap_or(1).max(1),
            size: self.size.unwrap_or(DEFAULT_SIZE).clamp(1, MAX_SIZE),
        }
    }
}

impl Normalized {
    pub fn offset(&self) -> i64 {
        i64::from(self.page.saturating_sub(1)) * i64::from(self.size)
    }

    /// 过滤条件。`$1` 是 run_id，`$2..$6` 依次是搜索词 / 部门 / 停更 / 字段不一致 / 空值下限。
    pub fn where_clause() -> &'static str {
        r"o.run_id = $1
      and ($2::text is null
           or d.name ilike '%' || $2 || '%'
           or coalesce(d.dept, '') ilike '%' || $2 || '%')
      and ($3::text is null or d.dept = $3)
      and (not $4::boolean or o.stopped)
      and (not $5::boolean or o.schema_kind like '%不一致%')
      and ($6::real <= 0 or o.empty_rate > $6)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(size: Option<u32>, page: Option<u32>) -> DatasetQuery {
        DatasetQuery {
            run: None,
            q: None,
            dept: None,
            sort: SortKey::default(),
            stopped: false,
            mismatch: false,
            min_empty: 0.0,
            page,
            size,
        }
    }

    #[test]
    fn blank_search_terms_are_dropped() {
        // 空格搜索词如果不清理，会变成 ilike '%%' —— 全表扫，看起来却像"查询成功"
        let n = DatasetQuery {
            q: Some("   ".into()),
            dept: Some("".into()),
            ..q(None, None)
        }
        .normalize();
        assert_eq!(n.q, None);
        assert_eq!(n.dept, None);
    }

    #[test]
    fn search_terms_are_trimmed_not_mangled() {
        let n = DatasetQuery {
            q: Some("  票务 ".into()),
            ..q(None, None)
        }
        .normalize();
        assert_eq!(n.q.as_deref(), Some("票务"));
    }

    #[test]
    fn page_size_is_clamped() {
        assert_eq!(q(Some(0), None).normalize().size, 1, "0 要夹到 1");
        assert_eq!(q(Some(100_000), None).normalize().size, MAX_SIZE);
        assert_eq!(q(None, None).normalize().size, DEFAULT_SIZE);
        assert_eq!(q(None, Some(0)).normalize().page, 1, "页码从 1 起");
    }

    #[test]
    fn min_empty_rejects_negative_and_nan() {
        let base = q(None, None);
        for v in [-1.0, f64::NAN, f64::INFINITY] {
            let n = DatasetQuery {
                min_empty: v,
                ..base.clone()
            }
            .normalize();
            assert_eq!(n.min_empty, 0.0, "非法下限应归零，实际 {v}");
        }
        let n = DatasetQuery {
            min_empty: 0.3,
            ..base
        }
        .normalize();
        assert!((n.min_empty - 0.3).abs() < 1e-9);
    }

    #[test]
    fn offset_accumulates_without_panicking_on_overflow() {
        let n = DatasetQuery {
            page: Some(u32::MAX),
            size: Some(MAX_SIZE),
            ..q(None, None)
        }
        .normalize();
        // 只要求不 panic 且非负：真翻到 i64 溢出时前端早就该改用别的查询了
        assert!(n.offset() >= 0);
        assert_eq!(
            q(None, Some(3)).normalize().offset(),
            2 * i64::from(DEFAULT_SIZE)
        );
    }

    #[test]
    fn sort_keys_are_a_closed_whitelist() {
        // 三个返回值都是固定字符串，外部无法把任意文本拼进 order by
        let all = [SortKey::Score, SortKey::Empty, SortKey::Lag];
        for k in all {
            let s = k.order_by();
            assert!(s.contains("desc"), "排序方向必须是固定的：{s}");
            assert!(
                !s.contains(';') && !s.contains("--"),
                "排序片段不能含 SQL 语法字符：{s}"
            );
        }
        // 尾随 ext_id 保证同分时顺序稳定，否则翻页会重复/漏行
        assert!(SortKey::Score.order_by().ends_with("d.ext_id"));
        assert!(SortKey::Empty.order_by().ends_with("d.ext_id"));
        assert!(SortKey::Lag.order_by().ends_with("d.ext_id"));
    }
}
