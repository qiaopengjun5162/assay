//! 迷你折线的纯坐标逻辑。刻意不依赖 DOM / SVG 类型，native 下可单测——
//! 曲线画歪了不会报错，只会让人读到错的趋势，所以坐标必须先算对。

/// 把一串数映射成 SVG 路径（`M x y L x y …`），画在 `w × h` 的框里，四周留 `pad`。
///
/// 规则：
/// - 空数组 → 空串（不画，也不返回一个假的空路径）；
/// - 一个点 → 一个点（`M x y`），由调用方决定要不要画成圆点；
/// - 全等 → 水平线走在中间（最大最小相等时除法会炸，也不该把平线贴到顶上）。
pub fn path(values: &[f64], w: f64, h: f64, pad: f64) -> String {
    let pts = points(values, w, h, pad);
    let mut out = String::new();
    for (i, (x, y)) in pts.iter().enumerate() {
        let sep = if i == 0 { 'M' } else { 'L' };
        if i > 0 {
            out.push(' ');
        }
        out.push(sep);
        out.push(' ');
        out.push_str(&fmt(*x));
        out.push(' ');
        out.push_str(&fmt(*y));
    }
    out
}

/// 与 [`path`] 同一套坐标，单独暴露给「要不要画点」的调用方。
pub fn points(values: &[f64], w: f64, h: f64, pad: f64) -> Vec<(f64, f64)> {
    if values.is_empty() {
        return Vec::new();
    }
    let inner_w = (w - 2.0 * pad).max(0.0);
    let inner_h = (h - 2.0 * pad).max(0.0);
    let (min, max) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(*v), hi.max(*v))
        });
    let span = max - min;
    let step = if values.len() > 1 {
        inner_w / (values.len() - 1) as f64
    } else {
        0.0
    };
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = pad + step * i as f64;
            // y 轴向下：值越大越靠上
            let ratio = if span > 0.0 { (v - min) / span } else { 0.5 };
            let y = pad + inner_h * (1.0 - ratio);
            (x, y)
        })
        .collect()
}

/// 两位小数足够：SVG 里再多位只是把 `data.json` 之外的产物撑大。
fn fmt(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// 与上一点相比的变化，给人看的一句话。`None` 表示「只有一个点，没得比」。
pub fn trend(current: f64, previous: f64, unit: &str) -> String {
    let d = current - previous;
    if d.abs() < 1e-9 {
        return format!("与上次持平（{}{unit}）", fmt(current));
    }
    let word = if d > 0.0 { "上升" } else { "下降" };
    format!("较上次{word} {}{unit}", fmt(d.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_series_draws_nothing() {
        assert_eq!(path(&[], 100.0, 30.0, 4.0), "");
        assert!(points(&[], 100.0, 30.0, 4.0).is_empty());
    }

    #[test]
    fn single_point_sits_on_the_left() {
        let p = points(&[5.0], 100.0, 30.0, 4.0);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].0, 4.0, "只有一个点时横坐标是左边界");
        assert!(path(&[5.0], 100.0, 30.0, 4.0).starts_with("M "));
    }

    #[test]
    fn two_points_reach_both_edges() {
        let p = points(&[0.0, 10.0], 100.0, 30.0, 4.0);
        assert_eq!(p[0].0, 4.0);
        assert_eq!(p[1].0, 96.0);
        // 值小在下、值大在上
        assert!(p[0].1 > p[1].1, "0 应当在 10 的下方");
    }

    #[test]
    fn flat_series_sits_in_the_middle_not_at_top() {
        let p = points(&[7.0, 7.0, 7.0], 100.0, 30.0, 4.0);
        let mid = 4.0 + (30.0 - 8.0) / 2.0;
        for (_, y) in p {
            assert!((y - mid).abs() < 1e-9, "平线走中间：{y} vs {mid}");
        }
    }

    #[test]
    fn path_uses_m_then_l_and_has_no_empty_numbers() {
        let d = path(&[1.0, 2.0, 3.0], 100.0, 30.0, 4.0);
        assert!(d.starts_with("M "), "{d}");
        assert_eq!(d.matches('M').count(), 1);
        assert_eq!(d.matches('L').count(), 2);
        assert!(!d.contains("NaN") && !d.contains("inf"), "{d}");
    }

    #[test]
    fn numbers_are_trimmed() {
        assert_eq!(fmt(4.0), "4");
        assert_eq!(fmt(4.50), "4.5");
        assert_eq!(fmt(4.567), "4.57");
    }

    #[test]
    fn trend_words() {
        assert_eq!(trend(12.0, 10.0, " 个"), "较上次上升 2 个");
        assert_eq!(trend(8.0, 10.0, " 个"), "较上次下降 2 个");
        assert_eq!(trend(10.0, 10.0, " 个"), "与上次持平（10 个）");
    }
}
