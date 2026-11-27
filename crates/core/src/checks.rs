//! 检查项登记表：判定规则与阈值的唯一来源。
//!
//! 为什么要有这张表：阈值原先散在四个文件里。`desc_len < 10` 写在 `scoring`、
//! `aggregate`（两处）和前端（`app.rs` 的「有说明 / 说明过短」），`STOPPED_CYCLES`
//! 写在五个地方，`> 0.3` / `> 90` / `> 365` 各一处。改一处忘一处的后果不是编译错误，
//! 而是「首页说 1,234 条说明过短，详情页把同一条显示成『有说明』」——两个数字都
//! 出自同一份数据，却互相打脸，而且没有任何测试会红。本项目已经吃过两次这种亏
//! （`empty_avg` 反序列化失败静默归零、`fill_pct` 把最差显示成最好）。
//!
//! 所以规则集中到这里：`id`、人话、维度、权重、阈值只在 `checks.rs` 出现一次，
//! `scoring` / `aggregate` / `export` 全部从这里取；站点再把这张表原样印出来——
//! 「我们查了什么、阈值多少」不该要求读者去翻 Rust 源码。
//!
//! 前端是唯一的例外：`crates/web` 刻意不依赖本 crate（core 带 ureq 等网络依赖，
//! 拉进 wasm 会撑大产物），所以 `DESC_MIN` / `EMPTY_RATE_HIGH` / `STOPPED_CYCLES`
//! 在 `crates/web/src/constants.rs` 有一份镜像。**改这里的常量必须同步那份镜像**，
//! `scripts/check-thresholds.sh` 会在 CI 里逐项比对，漂移即红。
//!
//! 这里刻意不做「用配置文件写规则」：一个能解释表达式的小引擎会带来新的失败面
//! （拼错字段名、未知算子、配置与代码版本错位），而它们全都在确定性路径上。
//! 现在这张表是数据，执行它的 `fires()` 只有一处实现，不存在两条执行路径。

use crate::model::Dataset;
use crate::scoring::{classify, SchemaKind};

/// 基准日之后超过这么多天算「明显滞后」。只用于页面提示与分桶，不参与评分。
pub const LAG_WARN_DAYS: i64 = 90;
pub const LAG_LONG_DAYS: i64 = 365;

/// 字段说明少于这么多字算「说明过短」。
pub const DESC_MIN: usize = 10;

/// 空值率高于这个比例算「偏高」。
pub const EMPTY_RATE_HIGH: f64 = 0.3;

/// 空值率对问题分的贡献：按斜率计，再封顶。
/// 这是唯一不是「命中/未命中」二值的检查项——空值率本身是连续量，
/// 把它二值化会丢掉「30% 和 90% 不该同分」这个信息。
pub const EMPTY_SLOPE: f64 = 6.0;
pub const EMPTY_CAP: f64 = 3.0;

/// 各检查项命中后对问题分的贡献。改这里就是改排序，会动到冻结产物。
pub const W_SCHEMA: f64 = 3.0;
pub const W_NAMING: f64 = 1.0;
pub const W_ALL_EMPTY: f64 = 2.0;
pub const W_STOPPED: f64 = 1.0;
pub const W_DESC: f64 = 1.0;
pub const W_NO_PREVIEW: f64 = 1.0;

/// 规则类型。每个变体对应 `fires()` 里的一个分支——不是字符串查表，
/// 所以拼错名字编译期就挂，不会静默变成「永远不命中」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    SchemaCount,
    SchemaFieldSet,
    SchemaNamingOnly,
    AllEmptyColumn,
    EmptyRateHigh,
    StoppedUpdate,
    OverdueClaim,
    DescTooShort,
    NoPreview,
    PreviewHeaderOnly,
}

/// 一条检查项。字段全是 `'static`，整张表可以编译进二进制、也可以直接序列化。
#[derive(Debug, Clone, Copy)]
pub struct Check {
    pub id: &'static str,
    /// 人话。页面上直接显示，不出现「schemaMismatch」这类内部名。
    pub title: &'static str,
    /// GB/T 36344 维度。
    pub dim: &'static str,
    /// 命中后对问题分的贡献。0 表示不影响排序——这类项只做标注，
    /// 明写 0 比让人猜「为什么这条不改分数」诚实。
    pub weight: f64,
    /// 权重为什么是这个值 / 为什么不参与评分 / 阈值是多少。
    pub note: &'static str,
    pub kind: Kind,
}

/// `data.json` 里 `checks` 数组的一行：一条检查项 + 本批命中数。
///
/// 单独定义，而不是在 `export` 里手写 `json!({...})`。手写的话，`Check` 加字段时
/// 导出不会跟着变——产物会悄悄少一个键，**而且没有任何编译错误**，只有读者
/// 拿到 `undefined`。有了结构体，键名由 derive 决定，`tests/artifact_parity.rs`
/// 再拿入库的 `web/data.json` 比对一次，两处都堵上。
///
/// 只导出**会被读者看到的**字段：`kind` 是内部判定分派用的，不进产物；
/// `Check::kind` 与 `id` 一一对应，前端也不需要它来判断什么。
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRow {
    pub id: &'static str,
    pub title: &'static str,
    pub dim: &'static str,
    /// 命中后对问题分的贡献。0 表示只标注、不参与排序。
    pub weight: f64,
    pub note: &'static str,
    /// 本批这条检查命中了多少条数据集。
    pub hits: usize,
    /// 分母：本批体检了多少条。与 `hits` 同屏显示才读得出比例。
    pub of: usize,
}

impl CheckRow {
    /// 从 `tally()` 的一项构造。`of` 是本批体检总数（`hits` 的分母）。
    #[must_use]
    pub fn new(c: &'static Check, hits: usize, of: usize) -> Self {
        Self {
            id: c.id,
            title: c.title,
            dim: c.dim,
            weight: c.weight,
            note: c.note,
            hits,
            of,
        }
    }
}

/// 判定一条数据需要看的全部输入。
///
/// `update_time` 来自目录快照而不是 `Dataset` 本身——所以「逾期」这条检查
/// 在没有目录文件时会老实返回 false，而不是拿别的字段顶替。
#[derive(Debug, Clone, Copy)]
pub struct Facts<'a> {
    pub d: &'a Dataset,
    pub update_time: Option<&'a str>,
}

impl<'a> Facts<'a> {
    /// 只有跑批产物、没有目录快照时的构造：逾期类检查一律不命中。
    pub fn of(d: &'a Dataset) -> Self {
        Self {
            d,
            update_time: None,
        }
    }

    pub fn with_catalog(d: &'a Dataset, update_time: Option<&'a str>) -> Self {
        Self { d, update_time }
    }

    pub fn lag_days(&self) -> Option<i64> {
        self.update_time.and_then(crate::aggregate::lag_days)
    }

    /// 声称的周期都过了还没更新。周期不在已知词表里（比如「按季」没有阈值）时
    /// 返回 false：算不了就说算不了，不猜。
    pub fn overdue(&self) -> bool {
        match (
            crate::aggregate::cycle_threshold(&self.d.cycle_str()),
            self.lag_days(),
        ) {
            (Some(limit), Some(lag)) => lag > limit,
            _ => false,
        }
    }
}

/// 全表。顺序 = 评分时的相加顺序，改动会让浮点末位变化（最后统一 `round2`，
/// 但别赌它一定盖得住），所以新增项请加在末尾。
pub const ALL: &[Check] = &[
    Check {
        id: "schema-count",
        title: "声明列数与样例列数不一致",
        dim: "一致性",
        weight: W_SCHEMA,
        note: "结构性缺失或多出，权重 3",
        kind: Kind::SchemaCount,
    },
    Check {
        id: "schema-field-set",
        title: "字段集合对不上，且不是部门前缀造成的",
        dim: "一致性",
        weight: W_SCHEMA,
        note: "结构性不一致，权重 3",
        kind: Kind::SchemaFieldSet,
    },
    Check {
        id: "schema-naming-only",
        title: "字段名只差部门前缀（如「鹿西乡房间数」对「房间数」）",
        dim: "一致性",
        weight: W_NAMING,
        note: "命名习惯差异，不是数据问题，权重 1",
        kind: Kind::SchemaNamingOnly,
    },
    Check {
        id: "all-empty-column",
        title: "存在整列全空的字段",
        dim: "完整性",
        weight: W_ALL_EMPTY,
        note: "整列无值，权重 2",
        kind: Kind::AllEmptyColumn,
    },
    Check {
        id: "empty-rate-high",
        title: "样例空值率超过 30%",
        dim: "完整性",
        weight: 0.0,
        note: "不按二值计分：按空值率×6 计分，上限 3 分",
        kind: Kind::EmptyRateHigh,
    },
    Check {
        id: "stopped-update",
        title: "平台自己标了停更（-1 / 999 / 不更新）",
        dim: "时效性",
        weight: W_STOPPED,
        note: "平台声明停止更新，权重 1",
        kind: Kind::StoppedUpdate,
    },
    Check {
        id: "overdue-claim",
        title: "超过自称的更新周期仍未更新",
        dim: "时效性",
        weight: 0.0,
        note: "只做标注，不进问题分——它衡量的是「承诺没兑现」，与数据本身质量是两件事",
        kind: Kind::OverdueClaim,
    },
    Check {
        id: "desc-too-short",
        title: "字段说明不足 10 字",
        dim: "规范性",
        weight: W_DESC,
        note: "说明过短，无法据此使用，权重 1",
        kind: Kind::DescTooShort,
    },
    Check {
        id: "no-preview",
        title: "平台不提供样例数据",
        dim: "可访问性",
        weight: W_NO_PREVIEW,
        note: "拿不到表头与行，权重 1",
        kind: Kind::NoPreview,
    },
    Check {
        id: "preview-header-only",
        title: "预览只返回了表头，没有数据行",
        dim: "可访问性",
        weight: 0.0,
        note: "与 no-preview 是两件事：这个平台给了预览，但一行数据都没有。\
               也不等于 summary.withPreview——那个数的是「行数 > 0」",
        kind: Kind::PreviewHeaderOnly,
    },
];

/// 是否属于「字段结构对不上」。三个 schema 变体的并集，也就是
/// `summary.schemaMismatch` 的口径（含「仅命名不一致」）。
///
/// 这个函数存在的理由：这个口径原来写在两处——`aggregate` 里一个三选一的
/// `matches!`，`SchemaKind::is_mismatch()` 里另一个两选一的版本。后者没人调用，
/// 但数字是 1,256，前者 1,284，谁先被拿去用谁就把统计搞错。
pub fn schema_mismatch(f: &Facts) -> bool {
    ALL.iter()
        .filter(|c| {
            matches!(
                c.kind,
                Kind::SchemaCount | Kind::SchemaFieldSet | Kind::SchemaNamingOnly
            )
        })
        .any(|c| fires(c, f))
}

/// 这条检查在给定数据上是否命中。
pub fn fires(c: &Check, f: &Facts) -> bool {
    match c.kind {
        Kind::SchemaCount => classify(f.d) == SchemaKind::ColumnCount,
        Kind::SchemaFieldSet => classify(f.d) == SchemaKind::FieldSet,
        Kind::SchemaNamingOnly => classify(f.d) == SchemaKind::NamingOnly,
        Kind::AllEmptyColumn => f.d.all_empty_cols > 0,
        Kind::EmptyRateHigh => f.d.empty_rate > EMPTY_RATE_HIGH,
        Kind::StoppedUpdate => crate::scoring::STOPPED_CYCLES.contains(&f.d.cycle_str().as_str()),
        Kind::OverdueClaim => f.overdue(),
        Kind::DescTooShort => f.d.desc_len < DESC_MIN,
        Kind::NoPreview => f.d.has_preview == 0,
        Kind::PreviewHeaderOnly => f.d.has_preview != 0 && f.d.sample_rows == 0,
    }
}

/// 每条检查在整个批次上命中了多少条。返回顺序与 `ALL` 一致。
pub fn tally<'a, I>(facts: I) -> Vec<(&'static Check, usize)>
where
    I: IntoIterator<Item = Facts<'a>>,
{
    let mut hits = vec![0usize; ALL.len()];
    for f in facts {
        for (i, c) in ALL.iter().enumerate() {
            if fires(c, &f) {
                hits[i] += 1;
            }
        }
    }
    ALL.iter().zip(hits).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Dataset;

    fn ds(declared: usize, sample: usize) -> Dataset {
        Dataset {
            declared_cols: declared,
            sample_cols: sample,
            has_preview: 1,
            desc_len: 40,
            ..Default::default()
        }
    }

    /// id 与 kind 都不能重复。重复 id 会让「按 id 取权重」取到错的那条，
    /// 重复 kind 会让同一条规则被算两次分——两种错误在运行时都看不出来。
    #[test]
    fn ids_and_kinds_are_unique() {
        for (i, a) in ALL.iter().enumerate() {
            for b in ALL.iter().skip(i + 1) {
                assert_ne!(a.id, b.id, "检查项 id 重复：{}", a.id);
                assert_ne!(a.kind, b.kind, "Kind 重复：{}", a.id);
            }
        }
    }

    /// 三个 schema 变体互斥：一条数据最多命中其中一个，
    /// 否则权重会叠着加，分数直接翻倍。
    #[test]
    fn schema_kinds_are_mutually_exclusive() {
        let cases = [
            ds(4, 4), // 一致
            ds(9, 8), // 列数
            Dataset {
                declared_cols: 1,
                sample_cols: 1,
                missing_fields: vec!["甲公司名称".into()],
                extra_fields: vec!["联系人电话".into()],
                has_preview: 1,
                desc_len: 40,
                ..Default::default()
            }, // 字段集合
        ];
        for d in cases {
            let f = Facts::of(&d);
            let hit = ALL
                .iter()
                .filter(|c| {
                    matches!(
                        c.kind,
                        Kind::SchemaCount | Kind::SchemaFieldSet | Kind::SchemaNamingOnly
                    ) && fires(c, &f)
                })
                .count();
            assert!(hit <= 1, "schema 检查组命中了 {hit} 条：{:?}", d.id);
        }
    }

    #[test]
    fn thresholds_are_the_documented_values() {
        // 这些数字是公开承诺，改了就要改文档，所以钉住。
        assert_eq!(DESC_MIN, 10);
        assert_eq!(EMPTY_RATE_HIGH, 0.3);
        assert_eq!(LAG_WARN_DAYS, 90);
        assert_eq!(LAG_LONG_DAYS, 365);
        let w: f64 = ALL.iter().map(|c| c.weight).sum();
        assert_eq!(w, 3.0 + 3.0 + 1.0 + 2.0 + 1.0 + 1.0 + 1.0);
    }

    fn by_id(id: &str) -> &'static Check {
        ALL.iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("没有这条检查：{id}"))
    }

    #[test]
    fn desc_boundary_is_strictly_less_than_min() {
        let c = by_id("desc-too-short");
        let mut d = ds(1, 1);
        d.desc_len = DESC_MIN - 1;
        assert!(fires(c, &Facts::of(&d)));
        d.desc_len = DESC_MIN;
        assert!(!fires(c, &Facts::of(&d)));
    }

    /// 没有目录快照时必须老实说「算不了」，而不是拿别的字段顶替。
    #[test]
    fn overdue_needs_catalog_time() {
        let mut d = ds(1, 1);
        d.update_cycle = Some(serde_json::json!("1"));
        assert!(!Facts::of(&d).overdue());
        assert!(Facts::with_catalog(&d, Some("2020-01-01")).overdue());
        assert!(!Facts::with_catalog(&d, None).overdue());
    }

    /// 周期不在已知词表里（按季没有阈值）时不算逾期——算不了就明说算不了。
    #[test]
    fn unknown_cycle_is_not_overdue() {
        let mut d = ds(1, 1);
        d.update_cycle = Some(serde_json::json!("按季"));
        assert!(!Facts::with_catalog(&d, Some("2020-01-01")).overdue());
    }

    /// 「结构对不上」必须包含「仅命名不一致」——与 `summary.schemaMismatch` 同口径。
    ///
    /// 这条测试是为了钉住一次真实的冲突：`aggregate` 里手写三选一（1,284 条），
    /// `SchemaKind::is_mismatch()` 里手写两选一（1,256 条），差的就是命名差异那 28 条。
    /// 后者没人调用，但谁先拿去用谁就把统计搞错，且不会报错。
    #[test]
    fn schema_mismatch_includes_naming_only() {
        let naming = Dataset {
            declared_cols: 2,
            sample_cols: 2,
            missing_fields: vec!["鹿西乡民宿房间数".into(), "鹿西乡民宿床位数".into()],
            extra_fields: vec!["房间数".into(), "床位数".into()],
            has_preview: 1,
            desc_len: 40,
            ..Default::default()
        };
        assert_eq!(classify(&naming), SchemaKind::NamingOnly);
        assert!(schema_mismatch(&Facts::of(&naming)));

        let clean = ds(3, 3);
        assert!(!schema_mismatch(&Facts::of(&clean)));
    }

    /// 「预览只有表头」与「没有预览」是两件事，不能合并计数。
    #[test]
    fn header_only_is_not_no_preview() {
        let mut d = ds(4, 4);
        d.has_preview = 1;
        d.sample_rows = 0;
        d.sample_cols = 4;
        let f = Facts::of(&d);
        assert!(fires(by_id("preview-header-only"), &f));
        assert!(!fires(by_id("no-preview"), &f));

        let mut none = ds(0, 0);
        none.has_preview = 0;
        let g = Facts::of(&none);
        assert!(fires(by_id("no-preview"), &g));
        assert!(!fires(by_id("preview-header-only"), &g));
    }

    #[test]
    fn tally_counts_each_check_independently() {
        let a = Dataset {
            declared_cols: 9,
            sample_cols: 8,
            has_preview: 1,
            desc_len: 40,
            ..Default::default()
        };
        let b = Dataset {
            declared_cols: 3,
            sample_cols: 3,
            has_preview: 1,
            desc_len: 3,
            ..Default::default()
        };
        let t = tally([Facts::of(&a), Facts::of(&b)]);
        let get = |id: &str| {
            t.iter()
                .find(|(c, _)| c.id == id)
                .map(|(_, n)| *n)
                .unwrap_or(0)
        };
        assert_eq!(get("schema-count"), 1);
        assert_eq!(get("desc-too-short"), 1);
        assert_eq!(get("no-preview"), 0);
        assert_eq!(t.len(), ALL.len());
    }
}
