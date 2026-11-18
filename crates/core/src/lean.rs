//! Lean 4 证明生成。
//!
//! 只做一件事：把「某列空值恰好 N 条」写成内核可以穷举验证的定理。
//! 数据以二维列表嵌入模块，1 表示空值；改任何数字，`by decide` 都会报错。

use crate::matrix;

/// 单条结论：第 `col` 列空值恰好 `count` 条。
pub fn count_empty_theorem(dataset: usize, col: usize, count: usize) -> String {
    format!(
        "theorem empty_{dataset}_{col} : countEmpty rows_{dataset} {col} = {count} := by decide"
    )
}

/// 一个数据集的行定义块。`rate_percent` 只用于注释，方便人核对。
pub fn rows_block(
    name: &str,
    dataset: usize,
    rows: &[Vec<String>],
    ncols: usize,
    rate_percent: f64,
) -> String {
    let body: Vec<String> = rows
        .iter()
        .map(|r| {
            let bits: Vec<String> = matrix::empty_bits(r, ncols)
                .iter()
                .map(|b| b.to_string())
                .collect();
            format!("  [{}]", bits.join(", "))
        })
        .collect();
    format!(
        "-- {dataset}：{name} | 样例 {}x{ncols} | 空值率 {rate_percent:.1}%\ndef rows_{dataset} : List (List Nat) := [\n{}\n]",
        rows.len(),
        body.join(",\n")
    )
}

pub const COUNT_EMPTY_FN: &str = "/-- 某列空值条数 -/
def countEmpty (rs : List (List Nat)) (col : Nat) : Nat :=
  (rs.filter (fun r => r.getD col 0 == 1)).length";

/// 拼出完整模块：头注释 + 计数函数 + 数据块 + 定理。
pub fn module(blocks: &[String], theorems: &[String]) -> String {
    format!(
        "/-\n  自动生成 by assay\n  平台: 浙江省公共数据开放平台\n  说明: 1 = 空值；每行对应一条真实样例记录。改动任何数字，内核报错。\n-/\n\n{}\n\n{}\n\n{}\n",
        COUNT_EMPTY_FN,
        blocks.join("\n\n"),
        theorems.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theorem_shape_is_stable() {
        let t = count_empty_theorem(3, 7, 2);
        assert!(t.starts_with("theorem empty_3_7 :"));
        assert!(t.contains("rows_3 7 = 2"));
        assert!(t.ends_with(":= by decide"));
    }

    #[test]
    fn rows_block_encodes_empties() {
        let rows = vec![
            vec!["a".to_string(), "".to_string()],
            vec!["".to_string(), "b".to_string()],
        ];
        let b = rows_block("示例", 0, &rows, 2, 50.0);
        assert!(b.contains("[0, 1]"));
        assert!(b.contains("[1, 0]"));
        assert!(b.contains("空值率 50.0%"));
    }

    #[test]
    fn module_has_no_sorry() {
        let m = module(&["def rows_0 : List (List Nat) := []".into()], &[]);
        assert!(!m.contains("sorry"));
        assert!(m.contains("def countEmpty"));
    }
}
