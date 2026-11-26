//! 全量体检：目录 → 详情 → 样例，逐条产出 `items.jsonl`。
//!
//! 三个产物必须一起落：
//! - `items.jsonl`：逐条体检结果（可断点续跑）
//! - `catalog_snapshot.json`：目录全量，聚合阶段用它算「距上次更新多少天」
//! - `run.json`：这次跑批是从什么时候开始的（`export` 的 `generatedAt` 从这读）
//!
//! 只落 items 不落目录快照，是新环境跑 `report` 时时效性整块为空的原因。
//! 不落 `run.json`，是页头日期只能写死、然后和证据里的 `fetchedAt` 对不上的原因。

use crate::api::CatalogSource;
use crate::matrix;
use crate::model::{CatalogRecord, Dataset};
use crate::{Error, Result};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

pub struct AuditOptions {
    pub threads: usize,
    pub limit: usize,
    pub out_dir: String,
}

pub struct AuditOutcome {
    pub audited: usize,
    pub failed: usize,
    pub elapsed_secs: f64,
}

/// 跑一次体检。`done.txt` 记录已完成条数，用于续跑。
pub fn audit(opts: &AuditOptions, client: &dyn CatalogSource) -> Result<AuditOutcome> {
    let t0 = Instant::now();
    fs::create_dir_all(&opts.out_dir)?;

    let (total, pages, _) = client.catalog_page(1)?;
    println!("目录: total={total} pages={pages} threads={}", opts.threads);
    let catalog = client.catalog_all(opts.threads)?;
    println!(
        "目录拉取完成: {} 条，耗时 {:.1}s",
        catalog.len(),
        t0.elapsed().as_secs_f64()
    );

    // 目录快照必须落盘：aggregate 靠 updateTime 算滞后天数
    let snap_path = Path::new(&opts.out_dir).join("catalog_snapshot.json");
    fs::write(&snap_path, serde_json::to_string(&catalog)?)?;
    println!("目录快照: {}（{} 条）", snap_path.display(), catalog.len());
    let catalog_total = catalog.len();

    let picked: Vec<CatalogRecord> = if opts.limit > 0 {
        catalog.into_iter().take(opts.limit).collect()
    } else {
        catalog
    };
    let jsonl_path = Path::new(&opts.out_dir).join("items.jsonl");
    let done_path = Path::new(&opts.out_dir).join("done.txt");
    let done_before: usize = fs::read_to_string(&done_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    if done_before > 0 {
        println!("检测到已完成 {done_before} 条，续跑");
    }

    // 快照清单：给 `export` 一个不靠人记的时间来源。
    // 只在「新一轮」时写：续跑沿用第一次开跑的日期，否则同批数据会显示成两天。
    let run_path = Path::new(&opts.out_dir).join("run.json");
    if done_before == 0 || !run_path.exists() {
        let manifest = serde_json::json!({
            "capturedAt": crate::util::now_iso8601(),
            // 数据源名：聚合阶段的榜单标题要用它，避免换源后标题还印着浙江。
            "platform": client.platform(),
            "catalogTotal": catalog_total,
            "limit": opts.limit,
            "threads": opts.threads,
        });
        fs::write(&run_path, serde_json::to_string_pretty(&manifest)?)?;
        println!("跑批清单: {}", run_path.display());
    }

    let t1 = Instant::now();
    let results: Mutex<Vec<Option<Dataset>>> = Mutex::new(vec![None; picked.len()]);
    let cursor = AtomicUsize::new(done_before);
    let done = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    std::thread::scope(|s| {
        for _ in 0..opts.threads.max(1) {
            s.spawn(|| loop {
                let i = cursor.fetch_add(1, Ordering::SeqCst);
                if i >= picked.len() {
                    break;
                }
                let rec = &picked[i];
                let ds = audit_one(rec, i, client, &failed);
                results.lock().unwrap()[i] = Some(ds);

                let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                if n.is_multiple_of(500) {
                    let rate = n as f64 / t1.elapsed().as_secs_f64().max(1.0);
                    let remain = (picked.len().saturating_sub(n)) as f64 / rate.max(0.01) / 60.0;
                    println!(
                        "  进度 {n}/{} ({:.1}%) {rate:.1} 条/秒 预计剩余 {remain:.1} 分钟",
                        picked.len(),
                        n as f64 / picked.len().max(1) as f64 * 100.0
                    );
                }
            });
        }
    });

    let rows = results.into_inner().unwrap();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&jsonl_path)?;
    let mut written = 0usize;
    for ds in rows.into_iter().flatten() {
        writeln!(file, "{}", serde_json::to_string(&ds)?)?;
        written += 1;
    }
    file.flush()?;
    fs::write(&done_path, format!("{}", done_before + written))?;

    let elapsed = t1.elapsed().as_secs_f64() / 60.0;
    let failed = failed.load(Ordering::SeqCst);
    println!("体检完成: {written} 条，详情失败 {failed} 条，耗时 {elapsed:.1} 分钟");
    Ok(AuditOutcome {
        audited: written,
        failed,
        elapsed_secs: elapsed * 60.0,
    })
}

fn audit_one(
    rec: &CatalogRecord,
    index: usize,
    client: &dyn CatalogSource,
    failed: &AtomicUsize,
) -> Dataset {
    let mut d = Dataset {
        i: Some(index),
        id: rec.id.clone(),
        name: rec.source_name.clone(),
        dept: rec.private_dept.clone(),
        open_type: rec.open_type.clone(),
        update_cycle: rec.update_cycle.clone().map(serde_json::Value::String),
        data_size: rec.data_size.clone(),
        area_type: rec.area_type.clone(),
        ..Default::default()
    };

    let meta = match client.meta(&rec.id) {
        Ok(m) => m,
        Err(e) => {
            failed.fetch_add(1, Ordering::SeqCst);
            d.detail_error = Some(e.to_string());
            return d;
        }
    };
    let cols = meta.columns;
    d.declared_cols = cols.len();
    d.has_preview = if meta.has_preview { 1 } else { 0 };
    d.desc_len = meta.description_len;

    if d.has_preview != 1 {
        return d;
    }
    let table = match client.rows(&rec.id, 10) {
        Ok(t) => t,
        Err(e) => {
            d.preview_error = Some(e.to_string());
            return d;
        }
    };
    if table.is_empty() {
        return d;
    }
    let header = &table[0];
    let body = &table[1..];
    let ncols = header.len();
    d.sample_cols = ncols;
    d.sample_rows = body.len();
    if body.is_empty() || ncols == 0 {
        return d;
    }
    d.empty_rate = matrix::empty_rate(body, ncols);
    d.all_empty_cols = matrix::all_empty_columns(body, ncols);

    // 保持声明顺序 / 表头顺序，和旧版产物的数组顺序一致
    let declared: BTreeSet<&str> = cols.iter().map(|s| s.as_str()).collect();
    let actual: BTreeSet<&str> = header.iter().map(|s| s.as_str()).collect();
    d.missing_fields = cols
        .iter()
        .filter(|c| !actual.contains(c.as_str()))
        .cloned()
        .collect();
    d.extra_fields = header
        .iter()
        .filter(|h| !declared.contains(h.as_str()))
        .cloned()
        .collect();
    d
}

/// 读回 items.jsonl，跳过坏行。
pub fn read_items(path: &str) -> Result<Vec<Dataset>> {
    let text = fs::read_to_string(path).map_err(Error::Io)?;
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}
