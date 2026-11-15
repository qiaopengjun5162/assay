//! 时间与文本小工具（不引第三方时间库）。

/// 当前时间的 ISO-8601（UTC+8），形如 `2026-11-15T12:30:00+08:00`。
pub fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let local = secs + 8 * 3600;
    let days = local.div_euclid(86_400);
    let rem = local.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}+08:00",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// 公历→纪元日（Howard Hinnant）。
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 保留 `digits` 位小数。语义与 Python `round()` 一致：
/// 按真实二进制值做「最近舍入、半值取偶」。
///
/// 不用 `x * 10^n` 再取整——缩放会引入新的误差，
/// 0.15535 这类数会从「略小于半值」变成「恰好半值」，结果就差一位。
/// 直接走格式化（Rust 的 `{:.n}` 与 Python 的 round 同口径）再解析回来。
pub fn round_to(x: f64, digits: u32) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{:.*}", digits as usize, x).parse().unwrap_or(x)
}

/// 文件名安全化：只保留字母数字与 `- _`。
pub fn safe_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_roundtrip() {
        let z = days_from_civil(2026, 11, 14);
        assert_eq!(civil_from_days(z), (2026, 11, 14));
    }

    #[test]
    fn safe_id_strips_plus_and_slash() {
        assert_eq!(safe_id("a+b/c=="), "a_b_c__");
    }

    #[test]
    fn iso_format_shape() {
        let s = now_iso8601();
        assert_eq!(s.len(), 25);
        assert!(s.ends_with("+08:00"));
    }
}
