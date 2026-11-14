//! 全量体检：24,322 个数据集逐个体检，逐条落盘 JSONL。
//!
//! 用法：
//!   ./full_audit --threads 8 --out ../out-full
//!
//! 阶段：
//!   1. 并发拉目录（244 页 × 100）
//!   2. 并发拉详情 + 样例；逐条写 JSONL（中断可续跑，只追加已完成）
//!   3. 打印汇总（部门分布、空值率分布、字段不一致数）
//!
//! 说明：为了原型速度，post/enc 与 dataproof-core 重复实现了一份。
//! 后续抽成 lib.rs 共享，暂不重构。

use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const BASE: &str = "https://data.zjzwfw.gov.cn/dopServer";

fn post(path: &str, body: &Value) -> Result<Value, String> {
    let mut last = String::new();
    for attempt in 0..3 {
        let r = ureq::post(&format!("{}{}", BASE, path))
            .set("Content-Type", "application/json")
            .set("Referer", "https://data.zjzwfw.gov.cn/dopServer/")
            .set("User-Agent", "Mozilla/5.0 dataproof/0.1")
            .timeout(Duration::from_secs(30))
            .send_json(body);
        match r {
            Ok(resp) => match resp.into_json::<Value>() {
                Ok(v) => return Ok(v),
                Err(e) => last = format!("parse: {}", e),
            },
            Err(e) => last = format!("http: {}", e),
        }
        thread::sleep(Duration::from_millis(300 * (attempt as u64 + 1)));
    }
    Err(last)
}

fn enc(id: &str) -> String {
    id.replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D")
}

fn list_body(page: usize) -> Value {
    json!({
        "current": page, "size": 100, "sourceType": "A", "dataDomain": "",
        "openType": "", "deptId": null, "sortPath": null, "dataName": "",
        "order": "", "orderFiled": ""
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let threads: usize = args
        .iter()
        .position(|a| a == "--threads")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let out_dir = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "out-full".into());
    fs::create_dir_all(&out_dir).unwrap();

    // ---------- 阶段 1：目录 ----------
    let t_all = Instant::now();
    let first = post("/dataCatalog/dataQueryPage", &list_body(1)).expect("目录首页失败");
    let total = first["result"]["total"].as_u64().unwrap_or(0) as usize;
    let pages = first["result"]["pages"].as_u64().unwrap_or(0) as usize;
    println!("目录: total={} pages={} threads={}", total, pages, threads);

    let page_slots: Arc<Mutex<Vec<Option<Vec<Value>>>>> =
        Arc::new(Mutex::new((0..pages).map(|_| None).collect()));
    let page_counter = Arc::new(AtomicUsize::new(1)); // 第 1 页已有，从 2 开始
    {
        let mut hs = Vec::new();
        for _ in 0..threads {
            let slots = Arc::clone(&page_slots);
            let ctr = Arc::clone(&page_counter);
            hs.push(thread::spawn(move || loop {
                let p = ctr.fetch_add(1, Ordering::SeqCst);
                if p > pages {
                    break;
                }
                if let Ok(v) = post("/dataCatalog/dataQueryPage", &list_body(p)) {
                    let recs = v["result"]["records"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    slots.lock().unwrap()[p - 1] = Some(recs);
                }
            }));
        }
        for h in hs {
            h.join().ok();
        }
    }
    let mut recs_vec: Vec<Value> = Vec::with_capacity(total);
    {
        let slots = page_slots.lock().unwrap();
        let got = slots.iter().filter(|s| s.is_some()).count();
        println!(
            "目录拉取完成: {}/{} 页，耗时 {:.1}s",
            got,
            pages,
            t_all.elapsed().as_secs_f64()
        );
        for recs in slots.iter().flatten() {
            recs_vec.extend(recs.iter().cloned());
        }
    }
    let limit: usize = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let records: Arc<Vec<Value>> = Arc::new(if limit > 0 {
        recs_vec.into_iter().take(limit).collect()
    } else {
        recs_vec
    });
    println!("记录数: {}（目录声明 {}）", records.len(), total);

    // ---------- 阶段 2：逐个体检 ----------
    let jsonl_path = format!("{}/items.jsonl", out_dir);
    let done_path = format!("{}/done.txt", out_dir);
    let done_count = fs::read_to_string(&done_path)
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);
    if done_count > 0 {
        println!(
            "检测到已完成 {} 条，续跑（本轮从 {} 开始）",
            done_count, done_count
        );
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&jsonl_path)
        .unwrap();
    let writer = Arc::new(Mutex::new(BufWriter::new(file)));
    let cursor = Arc::new(AtomicUsize::new(done_count));
    let ok_count = Arc::new(AtomicUsize::new(0));
    let err_count = Arc::new(AtomicUsize::new(0));
    let t2 = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..threads {
        let recs = records.clone();
        let cur = Arc::clone(&cursor);
        let okc = Arc::clone(&ok_count);
        let errc = Arc::clone(&err_count);
        let w = Arc::clone(&writer);
        handles.push(thread::spawn(move || loop {
            let i = cur.fetch_add(1, Ordering::SeqCst);
            if i >= recs.len() {
                break;
            }
            let rec = &recs[i];
            let id = rec["id"].as_str().unwrap_or_default();
            let name = rec["sourceName"].as_str().unwrap_or_default();

            let mut item = json!({
                "i": i,
                "id": id,
                "name": name,
                "dept": rec["privateDept"],
                "openType": rec["openType"],
                "updateCycle": rec["updateCycle"],
                "updateCycleDetail": rec["updateCycleDetail"],
                "dataSize": rec["dataSize"],
                "areaType": rec["areaType"],
            });

            match post(
                &format!("/dataCatalog/catalogDetail/{}", enc(id)),
                &json!({}),
            ) {
                Ok(d) => {
                    let cols = d["result"]["columnList"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    let has_preview = d["result"]["hasPreviewData"].as_i64().unwrap_or(0);
                    let desc_len = d["result"]["dopSourceInfo"]["description"]
                        .as_str()
                        .unwrap_or("")
                        .chars()
                        .count();
                    item["declaredCols"] = json!(cols.len());
                    item["descLen"] = json!(desc_len);
                    item["hasPreview"] = json!(has_preview);

                    if has_preview == 1 {
                        match post(
                            &format!("/dataCatalog/catalogDataPreview/{}", enc(id)),
                            &json!({}),
                        ) {
                            Ok(pv) => {
                                let rows =
                                    pv["result"]["data"].as_array().cloned().unwrap_or_default();
                                if !rows.is_empty() {
                                    let header = rows[0].as_array().cloned().unwrap_or_default();
                                    let body = &rows[1..];
                                    let ncols = header.len();
                                    let nrows = body.len();
                                    let mut empties = 0usize;
                                    let mut cells = 0usize;
                                    let mut per_col = vec![0usize; ncols];
                                    for r in body {
                                        let arr = r.as_array().cloned().unwrap_or_default();
                                        for (c, slot) in per_col.iter_mut().enumerate() {
                                            cells += 1;
                                            let e = arr
                                                .get(c)
                                                .map(|v| {
                                                    v.as_str()
                                                        .map(|s| s.trim().is_empty())
                                                        .unwrap_or(v.is_null())
                                                })
                                                .unwrap_or(true);
                                            if e {
                                                empties += 1;
                                                *slot += 1;
                                            }
                                        }
                                    }
                                    let header_names: Vec<String> = header
                                        .iter()
                                        .map(|v| v.as_str().unwrap_or("").to_string())
                                        .collect();
                                    let declared_names: Vec<String> = cols
                                        .iter()
                                        .map(|c| {
                                            c["colNameCn"]
                                                .as_str()
                                                .or(c["colName"].as_str())
                                                .unwrap_or("")
                                                .to_string()
                                        })
                                        .collect();
                                    let missing: Vec<&String> = declared_names
                                        .iter()
                                        .filter(|n| !header_names.contains(n))
                                        .collect();
                                    let extra: Vec<&String> = header_names
                                        .iter()
                                        .filter(|n| !declared_names.contains(n))
                                        .collect();

                                    item["sampleCols"] = json!(ncols);
                                    item["sampleRows"] = json!(nrows);
                                    item["emptyRate"] = json!(if cells > 0 {
                                        (empties as f64 / cells as f64 * 10000.0).round() / 10000.0
                                    } else {
                                        0.0
                                    });
                                    item["allEmptyCols"] = json!(per_col
                                        .iter()
                                        .filter(|&&c| nrows > 0 && c == nrows)
                                        .count());
                                    item["missingFields"] = json!(missing);
                                    item["extraFields"] = json!(extra);
                                }
                            }
                            Err(e) => {
                                item["previewError"] = json!(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    errc.fetch_add(1, Ordering::SeqCst);
                    item["detailError"] = json!(e);
                }
            }

            okc.fetch_add(1, Ordering::SeqCst);
            let done = okc.load(Ordering::SeqCst);
            {
                let mut w = w.lock().unwrap();
                writeln!(w, "{}", item).ok();
                if done.is_multiple_of(50) {
                    w.flush().ok();
                }
            }
            if done.is_multiple_of(500) {
                let secs = t2.elapsed().as_secs_f64();
                let rate = done as f64 / secs.max(1.0);
                let remain = (recs.len().saturating_sub(done)) as f64 / rate.max(0.01);
                println!(
                    "  进度 {}/{} ({:.1}%) {:.1} 条/秒 预计剩余 {:.1} 分钟",
                    done,
                    recs.len(),
                    done as f64 / recs.len() as f64 * 100.0,
                    rate,
                    remain / 60.0
                );
            }
        }));
    }
    for h in handles {
        h.join().ok();
    }
    writer.lock().unwrap().flush().ok();

    let ok = ok_count.load(Ordering::SeqCst);
    let err = err_count.load(Ordering::SeqCst);
    fs::write(&done_path, format!("{}", done_count + ok)).ok();
    println!(
        "\n体检完成: {} 条，详情失败 {} 条，耗时 {:.1} 分钟",
        ok,
        err,
        t2.elapsed().as_secs_f64() / 60.0
    );
    println!("明细: {}", jsonl_path);
}
