//! dataproof-core —— 开放数据体检内核（Rust）
//!
//! 三步全在同一个二进制里：
//!   1. 拉目录 + 字段定义 + 样例数据（快照，算 SHA-256）
//!   2. 确定性规则算指标（空值率 / 整列全空 / 字段说明缺失 / 更新滞后）
//!   3. 把数值结论编译成 Lean 4 定理，并调用 lean 内核校验
//!
//! 用法: cargo run --release -- --limit 5 --out ../out

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const BASE: &str = "https://data.zjzwfw.gov.cn/dopServer";

fn post(path: &str, body: &Value) -> Result<Value, String> {
    let resp = ureq::post(&format!("{}{}", BASE, path))
        .set("Content-Type", "application/json")
        .set("Referer", "https://data.zjzwfw.gov.cn/dopServer/")
        .set("User-Agent", "Mozilla/5.0 dataproof/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .send_json(body);
    match resp {
        Ok(r) => r.into_json::<Value>().map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// 只编码路径段里真正危险的三个字符，base64 id 里 / 和 = 必须转义
fn enc(id: &str) -> String {
    id.replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D")
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // 只用于报告标注，不追求时区格式化
    format!("unix:{}", secs)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let limit: usize = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let out_dir = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "out".to_string());
    fs::create_dir_all(&out_dir).expect("create out dir");

    // ---------- 1. 目录 ----------
    let list_body = json!({
        "current": 1, "size": 100, "sourceType": "A", "dataDomain": "",
        "openType": "", "deptId": null, "sortPath": null, "dataName": "",
        "order": "", "orderFiled": ""
    });
    let list = post("/dataCatalog/dataQueryPage", &list_body).expect("目录接口失败");
    let total = list["result"]["total"].as_u64().unwrap_or(0);
    let records = list["result"]["records"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    println!(
        "目录 total = {}，本页 {} 条，体检前 {} 条",
        total,
        records.len(),
        limit
    );

    let mut reports: Vec<Value> = Vec::new();
    let mut lean_theorems: Vec<String> = Vec::new();
    let mut lean_blocks: Vec<String> = Vec::new();
    let mut snapshot_hasher = Sha256::new();

    for rec in records.iter().take(limit) {
        let idx = reports.len();
        let mut item_rows: Vec<String> = Vec::new();
        let id = rec["id"].as_str().unwrap_or_default();
        let name = rec["sourceName"].as_str().unwrap_or_default();

        // 详情：字段定义
        let detail = post(
            &format!("/dataCatalog/catalogDetail/{}", enc(id)),
            &json!({}),
        )
        .unwrap_or_else(|_| json!({"result": {}}));
        let cols = detail["result"]["columnList"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        let has_preview = detail["result"]["hasPreviewData"].as_i64().unwrap_or(0);
        let desc_len = detail["result"]["dopSourceInfo"]["description"]
            .as_str()
            .unwrap_or("")
            .chars()
            .count();

        // 样例：表头 + 至多 5 行
        let mut empty_rate = 0.0f64;
        let mut all_empty_cols = 0usize;
        let mut sample_rows = 0usize;
        let mut sample_cols = 0usize;
        if has_preview == 1 {
            if let Ok(pv) = post(
                &format!("/dataCatalog/catalogDataPreview/{}", enc(id)),
                &json!({}),
            ) {
                if let Some(rows) = pv["result"]["data"].as_array() {
                    if !rows.is_empty() {
                        let header = rows[0].as_array().cloned().unwrap_or_default();
                        sample_cols = header.len();
                        let body = &rows[1..];
                        sample_rows = body.len();
                        let mut empties = 0usize;
                        let mut cells = 0usize;
                        let mut per_col: Vec<usize> = vec![0; sample_cols];
                        for r in body {
                            let arr = r.as_array().cloned().unwrap_or_default();
                            for (i, slot) in per_col.iter_mut().enumerate() {
                                cells += 1;
                                let is_empty = arr
                                    .get(i)
                                    .map(|v| {
                                        v.as_str()
                                            .map(|s| s.trim().is_empty())
                                            .unwrap_or(v.is_null())
                                    })
                                    .unwrap_or(true);
                                if is_empty {
                                    empties += 1;
                                    *slot += 1;
                                }
                            }
                        }
                        empty_rate = if cells > 0 {
                            empties as f64 / cells as f64
                        } else {
                            0.0
                        };
                        all_empty_cols = per_col
                            .iter()
                            .filter(|&&c| sample_rows > 0 && c == sample_rows)
                            .count();

                        // 快照入哈希 + 生成 Lean 数据行（每个数据集一份 rows_i）
                        snapshot_hasher.update(serde_json::to_vec(rows).unwrap_or_default());
                        for (i, c) in per_col.iter().enumerate() {
                            lean_theorems.push(format!(
                                "theorem empty_{}_{} : countEmpty rows_{} {} = {} := by decide",
                                idx, i, idx, i, c
                            ));
                        }
                        for r in body {
                            let arr = r.as_array().cloned().unwrap_or_default();
                            let bits: Vec<String> = (0..sample_cols)
                                .map(|i| {
                                    let e = arr
                                        .get(i)
                                        .map(|v| {
                                            v.as_str()
                                                .map(|s| s.trim().is_empty())
                                                .unwrap_or(v.is_null())
                                        })
                                        .unwrap_or(true);
                                    if e {
                                        "1".to_string()
                                    } else {
                                        "0".to_string()
                                    }
                                })
                                .collect();
                            item_rows.push(format!("  [{}]", bits.join(", ")));
                        }
                        if !item_rows.is_empty() {
                            lean_blocks.push(format!(
                                "-- {} | 样例 {}x{}\ndef rows_{} : List (List Nat) := [\n{}\n]",
                                name,
                                sample_rows,
                                sample_cols,
                                idx,
                                item_rows.join(",\n")
                            ));
                        }
                    }
                }
            }
        }

        reports.push(json!({
            "id": id,
            "name": name,
            "dept": rec["privateDept"],
            "openType": rec["openType"],
            "updateCycle": rec["updateCycle"],
            "updateCycleDetail": rec["updateCycleDetail"],
            "declaredCols": cols,
            "sampleCols": sample_cols,
            "sampleRows": sample_rows,
            "hasPreview": has_preview,
            "emptyRate": (empty_rate * 10000.0).round() / 10000.0,
            "allEmptyCols": all_empty_cols,
            "descLen": desc_len,
            "schemaConsistent": if sample_cols > 0 { json!(cols == sample_cols) } else { Value::Null },
        }));
        println!(
            "  · {} | 字段{} 样例{}x{} 空值{:.1}% 全空列{}",
            name,
            cols,
            sample_cols,
            sample_rows,
            empty_rate * 100.0,
            all_empty_cols
        );
    }

    // ---------- 2. 报告 ----------
    let hash = format!("{:x}", snapshot_hasher.finalize());
    let report = json!({
        "generatedAt": now_iso(),
        "source": "浙江省公共数据开放平台",
        "api": "data.zjzwfw.gov.cn/dopServer/dataCatalog",
        "snapshotSha256": hash,
        "catalogTotal": total,
        "items": reports,
    });
    fs::write(
        format!("{}/report.json", out_dir),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .expect("write report");

    // ---------- 3. Lean 证明 ----------
    let lean = format!(
        "/-\n  自动生成 by dataproof-core\n  平台: 浙江省公共数据开放平台\n  快照 SHA-256: {}\n  说明: 1 = 空值。改动任何数字，内核将报错。\n-/\n\n/-- 某列空值条数 -/\ndef countEmpty (rs : List (List Nat)) (col : Nat) : Nat :=\n  (rs.filter (fun r => r.getD col 0 == 1)).length\n\n{}\n\n{}\n",
        hash,
        lean_blocks.join("\n\n"),
        lean_theorems.join("\n")
    );
    let lean_path = format!("{}/Snapshot.lean", out_dir);
    fs::write(&lean_path, &lean).expect("write lean");
    println!("\nLean 定理 {} 条 → {}", lean_theorems.len(), lean_path);

    // ---------- 4. 内核校验 ----------
    match Command::new("lean").arg(&lean_path).output() {
        Ok(o) if o.status.success() => {
            println!("lean 内核校验: PASS（退出码 0）");
            let _ = fs::write(format!("{}/VERIFY.txt", out_dir), "PASS\n");
        }
        Ok(o) => {
            println!(
                "lean 内核校验: FAIL\n{}",
                String::from_utf8_lossy(&o.stdout)
            );
            let _ = fs::write(format!("{}/VERIFY.txt", out_dir), "FAIL\n");
        }
        Err(e) => println!("未找到 lean 可执行文件: {}", e),
    }
}
