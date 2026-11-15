//! 空值矩阵：把样例行转成 0/1 向量，并做列计数。
//!
//! 约定：1 = 空值，缺失的单元格按空处理。这个约定同时写进了 Lean 证明的注释。

/// 一行的空值位图。缺失的单元格按空处理。
pub fn empty_bits(row: &[String], ncols: usize) -> Vec<u8> {
    (0..ncols)
        .map(|i| match row.get(i) {
            Some(v) if !v.trim().is_empty() => 0,
            _ => 1,
        })
        .collect()
}

/// 每列空值条数。
pub fn per_col_empties(rows: &[Vec<String>], ncols: usize) -> Vec<usize> {
    let mut out = vec![0usize; ncols];
    for r in rows {
        for (i, bit) in empty_bits(r, ncols).iter().enumerate() {
            out[i] += *bit as usize;
        }
    }
    out
}

/// 样例整体的空值率（0~1）。
pub fn empty_rate(rows: &[Vec<String>], ncols: usize) -> f64 {
    if rows.is_empty() || ncols == 0 {
        return 0.0;
    }
    let empty: usize = rows
        .iter()
        .map(|r| {
            empty_bits(r, ncols)
                .iter()
                .map(|b| *b as usize)
                .sum::<usize>()
        })
        .sum();
    let cells = rows.len() * ncols;
    round4(empty as f64 / cells as f64)
}

/// 整列为空的列数。
pub fn all_empty_columns(rows: &[Vec<String>], ncols: usize) -> usize {
    per_col_empties(rows, ncols)
        .iter()
        .filter(|&&c| !rows.is_empty() && c == rows.len())
        .count()
}

fn round4(x: f64) -> f64 {
    crate::util::round_to(x, 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_cells_count_as_empty() {
        let row = vec!["a".to_string(), "".to_string()];
        assert_eq!(empty_bits(&row, 3), vec![0, 1, 1]);
    }

    #[test]
    fn per_col_counts_are_correct() {
        let rows = vec![
            vec!["a".to_string(), "".to_string()],
            vec!["".to_string(), "".to_string()],
        ];
        assert_eq!(per_col_empties(&rows, 2), vec![1, 2]);
        assert_eq!(all_empty_columns(&rows, 2), 1);
        assert_eq!(empty_rate(&rows, 2), 0.75);
    }
}
