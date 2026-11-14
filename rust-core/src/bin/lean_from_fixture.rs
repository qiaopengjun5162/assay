//! 从固定快照生成 Lean 模块，供黄金文件测试与 CI 使用（不访问网络）。
//!
//! 用法: lean_from_fixture [fixtures/snapshot.json]

use dataproof_core::{lean, matrix};
use serde_json::Value;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "fixtures/snapshot.json".to_string());
    let raw = std::fs::read_to_string(&path).expect("读不到快照文件");
    let doc: Value = serde_json::from_str(&raw).expect("快照不是合法 JSON");
    let checksum = doc["checksum"].as_str().unwrap_or("unknown");

    let mut blocks = Vec::new();
    let mut theorems = Vec::new();
    for (i, ds) in doc["datasets"]
        .as_array()
        .expect("缺 datasets")
        .iter()
        .enumerate()
    {
        let name = ds["name"].as_str().unwrap_or("未命名");
        let rows: Vec<Vec<String>> = ds["rows"]
            .as_array()
            .expect("缺 rows")
            .iter()
            .map(|r| {
                r.as_array()
                    .expect("行不是数组")
                    .iter()
                    .map(|v| v.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        blocks.push(lean::rows_block(name, i, &rows, ncols));
        for (col, count) in matrix::per_col_empties(&rows, ncols).iter().enumerate() {
            theorems.push(lean::count_empty_theorem(i, col, *count));
        }
    }

    print!("{}", lean::module(checksum, &blocks, &theorems));
}
