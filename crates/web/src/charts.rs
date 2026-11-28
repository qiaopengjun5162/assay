//! 图表坐标的纯计算。刻意不依赖 DOM / Leptos，native 下可单测——
//! 图的坐标算错了不会报错，只会让人读到一张好看但说谎的图。
//!
//! 这里只做「数据 → 几何」，不碰颜色、文案、SVG 元素。视图层拿到坐标自己去画，
//! 这样同一套计算既能被单测钉住，也不会被 wasm 特有的 API 绑住。

/// 一条带标签的数据，按值排序后画条形图。
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub label: String,
    pub value: u64,
}

/// 把键值映射成按值降序的条形数据，最多取 `limit` 条。
///
/// 排序用 `(值降序, 标签升序)`：值相同的时候按标签定序，
/// 否则两个平台谁排前面会随 HashMap 迭代顺序抖动，图每次刷新长得不一样。
pub fn top_bars<K: AsRef<str>>(items: &[(K, u64)], limit: usize) -> Vec<Bar> {
    let mut bars: Vec<Bar> = items
        .iter()
        .map(|(k, v)| Bar {
            label: k.as_ref().to_string(),
            value: *v,
        })
        .collect();
    bars.sort_by(|a, b| b.value.cmp(&a.value).then(a.label.cmp(&b.label)));
    bars.truncate(limit);
    bars
}

/// 一组百分比扇区，按给定顺序画环形图（顺序由调用方定，保证图例稳定）。
#[derive(Debug, Clone, PartialEq)]
pub struct Slice {
    pub label: String,
    pub value: u64,
    pub pct: f64,
}

/// 把一组计数算成百分比扇区。分母为 0 时全部百分比为 0，不返回假比例。
///
/// 百分比保留一位小数用于图例；累积角度用原始计数算，避免逐段四舍五入后
/// 最后一段合不上 360°。
pub fn slices(items: &[(String, u64)]) -> Vec<Slice> {
    let total: u64 = items.iter().map(|(_, v)| *v).sum();
    items
        .iter()
        .map(|(label, value)| Slice {
            label: label.clone(),
            value: *value,
            pct: if total > 0 {
                (*value as f64 / total as f64) * 100.0
            } else {
                0.0
            },
        })
        .collect()
}

/// 环形图一段弧的 SVG path。圆心 (cx, cy)，外半径 r_out、内半径 r_in。
///
/// 角度从 12 点开始顺时针。整圈（start==end）画两个半圆而不是一个退化的弧，
/// 否则 SVG 会把它当空路径；零长度段返回空串，调用方跳过即可。
pub fn donut_arc(cx: f64, cy: f64, r_out: f64, r_in: f64, start: f64, end: f64) -> String {
    if end <= start || r_out <= 0.0 || r_in <= 0.0 || r_in >= r_out {
        return String::new();
    }
    let sweep = end - start;
    if sweep >= 360.0 - 1e-9 {
        // 整圈：两个半圆拼出带孔的圆环
        return format!(
            "M {cx} {cy_out_top} A {r_out} {r_out} 0 1 1 {cx} {cy_out_bottom} \
             A {r_out} {r_out} 0 1 1 {cx} {cy_out_top} Z \
             M {cx} {cy_in_top} A {r_in} {r_in} 0 1 0 {cx} {cy_in_bottom} \
             A {r_in} {r_in} 0 1 0 {cx} {cy_in_top} Z",
            cy_out_top = fmt(cy - r_out),
            cy_out_bottom = fmt(cy + r_out),
            cy_in_top = fmt(cy - r_in),
            cy_in_bottom = fmt(cy + r_in),
        );
    }
    let (x1, y1) = polar(cx, cy, r_out, start);
    let (x2, y2) = polar(cx, cy, r_out, end);
    let (x3, y3) = polar(cx, cy, r_in, end);
    let (x4, y4) = polar(cx, cy, r_in, start);
    let large = if sweep > 180.0 { 1 } else { 0 };
    format!(
        "M {x1} {y1} A {r_out} {r_out} 0 {large} 1 {x2} {y2} \
         L {x3} {y3} A {r_in} {r_in} 0 {large} 0 {x4} {y4} Z"
    )
}

/// 角度（度，从 12 点顺时针）转笛卡尔坐标。
fn polar(cx: f64, cy: f64, r: f64, deg: f64) -> (String, String) {
    let rad = (deg - 90.0) * std::f64::consts::PI / 180.0;
    (fmt(cx + r * rad.cos()), fmt(cy + r * rad.sin()))
}

/// 每段的累积角度，给环形图逐段调用 [`donut_arc`]。
///
/// 最后一段直接补到 360.0，消除浮点累积误差造成的缝。
pub fn arc_bounds(slices: &[Slice]) -> Vec<(f64, f64)> {
    let total: f64 = slices.iter().map(|s| s.value as f64).sum();
    if total <= 0.0 {
        return vec![(0.0, 0.0); slices.len()];
    }
    let mut acc = 0.0;
    let mut out = Vec::with_capacity(slices.len());
    for (i, s) in slices.iter().enumerate() {
        let start = acc;
        acc += s.value as f64 / total * 360.0;
        let end = if i + 1 == slices.len() { 360.0 } else { acc };
        out.push((start, end));
    }
    out
}

/// 直方图分桶：按固定宽度把值分到 `[0,w),[w,2w)…`，最后一桶标 `≥上限`。
///
/// 只统计 `Some` 且非负的值；缺失值单独计数，绝不补 0 混进分布——
/// 「不知道」画成 0 会让人以为这批数据很新。
pub fn histogram(values: &[Option<i64>], width: i64, buckets: usize) -> Vec<Bar> {
    let mut counts = vec![0u64; buckets + 1];
    let mut missing = 0u64;
    for v in values {
        match v {
            Some(n) if *n >= 0 => {
                let idx = (*n / width).min(buckets as i64) as usize;
                counts[idx] += 1;
            }
            Some(_) | None => missing += 1,
        }
    }
    let mut out: Vec<Bar> = counts
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            let label = if i == buckets {
                format!("≥{} 天", width * buckets as i64)
            } else {
                format!("{}–{} 天", width * i as i64, width * (i + 1) as i64 - 1)
            };
            Bar { label, value }
        })
        .collect();
    if missing > 0 {
        out.push(Bar {
            label: "无日期".to_string(),
            value: missing,
        });
    }
    out
}

/// 条形图的像素高度：最大值映射到 `max_h`，0 映射到 0。全是 0 时全 0。
pub fn bar_height(value: u64, max_value: u64, max_h: f64) -> f64 {
    if max_value == 0 {
        0.0
    } else {
        (value as f64 / max_value as f64) * max_h
    }
}

fn fmt(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_bars_sort_by_value_then_label() {
        let items = [("b", 5u64), ("a", 5), ("c", 9)];
        let bars = top_bars(&items, 2);
        assert_eq!(bars[0].label, "c");
        assert_eq!(bars[1].label, "a", "同值时按标签升序，结果稳定");
        assert_eq!(bars.len(), 2);
    }

    #[test]
    fn empty_items_produce_no_bars() {
        let empty: [(&str, u64); 0] = [];
        assert!(top_bars(&empty, 10).is_empty());
        assert!(slices(&[]).is_empty());
    }

    #[test]
    fn slices_sum_to_100_and_handle_zero_total() {
        let ss = slices(&[("A".into(), 1), ("B".into(), 3)]);
        assert!((ss[0].pct - 25.0).abs() < 1e-9);
        assert!((ss[1].pct - 75.0).abs() < 1e-9);
        let zero = slices(&[("A".into(), 0), ("B".into(), 0)]);
        assert!(zero.iter().all(|s| s.pct == 0.0));
    }

    #[test]
    fn arc_bounds_cover_full_circle_without_gaps() {
        let ss = slices(&[("A".into(), 1), ("B".into(), 2), ("C".into(), 3)]);
        let bounds = arc_bounds(&ss);
        assert_eq!(bounds.len(), 3);
        assert_eq!(bounds[0].0, 0.0);
        assert!((bounds[0].1 - 60.0).abs() < 1e-9);
        assert!((bounds[2].1 - 360.0).abs() < 1e-9, "末段补到 360 消除缝");
    }

    #[test]
    fn donut_arc_rejects_zero_and_full_circle_draws_ring() {
        assert_eq!(donut_arc(50.0, 50.0, 40.0, 25.0, 0.0, 0.0), "");
        let full = donut_arc(50.0, 50.0, 40.0, 25.0, 0.0, 360.0);
        assert!(full.starts_with("M "));
        assert!(full.contains("A 40 40"), "整圈用两个半圆，不是退化弧");
    }

    #[test]
    fn donut_arc_small_sweep_is_not_large_arc() {
        let arc = donut_arc(50.0, 50.0, 40.0, 25.0, 0.0, 30.0);
        assert!(arc.contains("0 1"), "小于 180 度不能用 large-arc 标记");
        assert!(!arc.contains("NaN") && !arc.contains("inf"));
    }

    #[test]
    fn histogram_buckets_and_missing_are_separate() {
        let vals = [Some(0), Some(29), Some(30), Some(120), None, Some(-1)];
        let bars = histogram(&vals, 30, 4);
        assert_eq!(bars[0].value, 2, "0–29 天两例");
        assert_eq!(bars[1].value, 1, "30–59 天一例");
        assert_eq!(bars[4].value, 1, "≥120 天一例");
        let last = bars.last().unwrap();
        assert_eq!(last.label, "无日期");
        assert_eq!(last.value, 2, "缺失与负数一起单列，不混进 0 天");
    }

    #[test]
    fn bar_height_handles_all_zero() {
        assert_eq!(bar_height(5, 0, 100.0), 0.0);
        assert_eq!(bar_height(5, 10, 100.0), 50.0);
    }
}
