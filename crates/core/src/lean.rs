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

/// 项目锁定的 Lean 工具链，编译时从仓库根 `lean-toolchain` 嵌入。
///
/// 为什么编译时嵌入而不是运行时读文件：Releases 里的压缩包只有一个 `assay`
/// 二进制，用户所在目录没有 `lean-toolchain`。elan 在运行目录找不到锁文件时，
/// 会转去安装自己的默认版本——实测卡在 v4.36 的安装锁上，只打印一行
/// `waiting for previous installation request to finish` 就长时间不动。
/// 把版本编进二进制，`prove` 在任何目录跑都对得齐仓库真正验证过的版本。
///
/// 代价：本 crate 依赖仓库根的 `lean-toolchain`，不能脱离仓库单独 `cargo package`。
/// assay 只通过 GitHub Releases 分发、不发布到 crates.io，这个取舍可接受。
pub const LOCKED_TOOLCHAIN_FILE: &str = include_str!("../../../lean-toolchain");

/// 去掉行尾换行后的工具链标识，例如 `leanprover/lean4:v4.27.0`。
pub fn locked_toolchain() -> &'static str {
    LOCKED_TOOLCHAIN_FILE.trim()
}

/// 决定要不要给 `lean` 注入 `ELAN_TOOLCHAIN`。
///
/// 用户已经显式指定时为 `None`，让用户的选择优先；否则返回项目锁定版本。
/// 抽成纯函数是为了单测不碰进程级环境变量——并行测试里改环境变量会互相干扰。
pub fn toolchain_override(current: Option<&std::ffi::OsStr>) -> Option<&'static str> {
    if current.is_some() {
        None
    } else {
        Some(locked_toolchain())
    }
}

/// 构造一个对齐项目版本的 `lean` 命令。
///
/// 只额外设置 `ELAN_TOOLCHAIN`；非 elan 安装的 lean 会忽略它，行为不变。
pub fn lean_command() -> std::process::Command {
    let mut cmd = std::process::Command::new("lean");
    let current = std::env::var_os("ELAN_TOOLCHAIN");
    if let Some(tc) = toolchain_override(current.as_deref()) {
        cmd.env("ELAN_TOOLCHAIN", tc);
    }
    cmd
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

    #[test]
    fn locked_toolchain_is_well_formed() {
        let tc = locked_toolchain();
        assert!(
            tc.starts_with("leanprover/lean4:v"),
            "锁定的工具链格式不对: {tc:?}"
        );
        assert!(!tc.contains('\n'), "不应残留换行: {tc:?}");
    }

    #[test]
    fn explicit_env_wins_over_pin() {
        let own = std::ffi::OsStr::new("leanprover/lean4:v4.35.0");
        assert_eq!(toolchain_override(Some(own)), None);
    }

    #[test]
    fn missing_env_falls_back_to_pin() {
        assert_eq!(toolchain_override(None), Some(locked_toolchain()));
    }
}
