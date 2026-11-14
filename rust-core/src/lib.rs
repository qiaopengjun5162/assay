//! 数证 DataProof 核心纯函数。
//!
//! 这里只放**不碰网络、不碰文件**的逻辑，保证可以在 CI 里无依赖测试：
//! - 样例空值矩阵与列计数
//! - Lean 定理与模块的生成（黄金文件测试）
//! - 字段一致性分类（与 scripts/scoring.py 保持同一口径）

use serde::Serialize;

/// 字段声明与实际样例的对应关系。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SchemaKind {
    /// 列数与字段名都对得上
    Consistent,
    /// 列数不同 —— 声明 9 列、实际 8 列，是真的少了
    ColumnCount,
    /// 列数相同，但名字对不上且不是前缀关系
    FieldSet,
    /// 列数相同，声明名 = 前缀 + 样例名（洞头区常见写法）
    NamingOnly,
    /// 平台没给样例数据
    NoSample,
}

impl SchemaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SchemaKind::Consistent => "一致",
            SchemaKind::ColumnCount => "列数不一致",
            SchemaKind::FieldSet => "字段集合不一致",
            SchemaKind::NamingOnly => "仅命名不一致",
            SchemaKind::NoSample => "无样例",
        }
    }
}

pub fn classify_schema(
    declared_cols: usize,
    sample_cols: usize,
    missing: &[String],
    extra: &[String],
) -> SchemaKind {
    if sample_cols == 0 {
        return SchemaKind::NoSample;
    }
    if declared_cols == sample_cols && missing.is_empty() && extra.is_empty() {
        return SchemaKind::Consistent;
    }
    if declared_cols != sample_cols {
        return SchemaKind::ColumnCount;
    }
    if !missing.is_empty() && missing.len() == extra.len() {
        let matched = missing
            .iter()
            .filter(|m| {
                extra
                    .iter()
                    .any(|e| m.ends_with(e.as_str()) || e.ends_with(m.as_str()))
            })
            .count();
        if matched == missing.len() {
            return SchemaKind::NamingOnly;
        }
    }
    SchemaKind::FieldSet
}

pub mod matrix {
    /// 把一行原始值转成 0/1 空值向量（1 = 空）。缺失的单元格按空处理。
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
}

pub mod lean {
    use super::matrix;

    /// 单条结论：第 `col` 列空值恰好 `count` 条。
    pub fn count_empty_theorem(dataset: usize, col: usize, count: usize) -> String {
        format!(
            "theorem empty_{}_{} : countEmpty rows_{} {} = {} := by decide",
            dataset, col, dataset, col, count
        )
    }

    /// 一个数据集的行定义块。
    pub fn rows_block(name: &str, dataset: usize, rows: &[Vec<String>], ncols: usize) -> String {
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
            "-- {} | 样例 {}x{}\ndef rows_{} : List (List Nat) := [\n{}\n]",
            name,
            rows.len(),
            ncols,
            dataset,
            body.join(",\n")
        )
    }

    /// 完整模块。`checksum` 是源快照的哈希，写进注释钉住时点。
    pub fn module(checksum: &str, blocks: &[String], theorems: &[String]) -> String {
        format!(
            "/-\n  自动生成 by dataproof-core\n  平台: 浙江省公共数据开放平台\n  快照 SHA-256: {}\n  说明: 1 = 空值。改动任何数字，内核将报错。\n-/\n\n/-- 某列空值条数 -/\ndef countEmpty (rs : List (List Nat)) (col : Nat) : Nat :=\n  (rs.filter (fun r => r.getD col 0 == 1)).length\n\n{}\n\n{}\n",
            checksum,
            blocks.join("\n\n"),
            theorems.join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_detects_column_count_mismatch() {
        let k = classify_schema(9, 8, &["更新时间".into()], &[]);
        assert_eq!(k, SchemaKind::ColumnCount);
    }

    #[test]
    fn classify_detects_prefix_naming() {
        let missing = vec![
            "鹿西乡民宿企业名称".to_string(),
            "鹿西乡民宿房间数".to_string(),
        ];
        let extra = vec!["企业名称".to_string(), "房间数".to_string()];
        assert_eq!(
            classify_schema(2, 2, &missing, &extra),
            SchemaKind::NamingOnly
        );
    }

    #[test]
    fn classify_detects_field_set_mismatch() {
        let missing = vec!["甲公司名称".to_string()];
        let extra = vec!["联系人电话".to_string()];
        assert_eq!(
            classify_schema(1, 1, &missing, &extra),
            SchemaKind::FieldSet
        );
    }

    #[test]
    fn empty_bits_treats_missing_cells_as_empty() {
        let row = vec!["a".to_string(), "".to_string()];
        assert_eq!(matrix::empty_bits(&row, 3), vec![0, 1, 1]);
    }

    #[test]
    fn per_col_counts_are_correct() {
        let rows = vec![
            vec!["a".to_string(), "".to_string()],
            vec!["".to_string(), "".to_string()],
        ];
        assert_eq!(matrix::per_col_empties(&rows, 2), vec![1, 2]);
    }

    #[test]
    fn theorem_mentions_dataset_and_count() {
        let t = lean::count_empty_theorem(3, 7, 2);
        assert!(t.contains("empty_3_7"));
        assert!(t.contains("rows_3 7 = 2"));
        assert!(t.ends_with(":= by decide"));
    }
}
