//! 前端判定阈值的唯一来源。
//!
//! 这些值必须与 `crates/core/src/checks.rs` / `scoring.rs` 对应常量**逐字相等**。
//! web crate 刻意不依赖 core（core 带着 ureq 等网络依赖，拉进 wasm 会撑大产物），
//! 所以没法 `use assay_core::checks::*`；代价就是要有 `scripts/check-thresholds.sh`
//! 这道门禁，逐项比对两边，改一处忘另一边就当场红。
//!
//! 历史事故：阈值曾散在四个文件，首页说 1,234 条「说明过短」、详情页把同一条
//! 显示成「有说明」，两份数字出自同一份数据却互相打脸，且没有任何测试会红。
//! 见 `crates/core/src/checks.rs` 模块文档与 D30。

/// 说明文字少于这么多字算「说明过短」。对应 `checks::DESC_MIN`。
pub const DESC_MIN: usize = 10;

/// 空值率高于这个比例算「偏高」。对应 `checks::EMPTY_RATE_HIGH`。
pub const EMPTY_RATE_HIGH: f64 = 0.3;

/// 平台用这几种写法表达「停更」。对应 `scoring::STOPPED_CYCLES`。
///
/// 不要从 `cycle_label() == "停更"` 反推——那等于把判定建立在展示文案上，
/// 改一句翻译就会让筛选器和评分悄悄分叉。
pub const STOPPED_CYCLES: [&str; 3] = ["-1", "999", "不更新"];
