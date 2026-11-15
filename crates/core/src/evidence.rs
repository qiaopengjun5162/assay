//! 证据包：把接口原始响应逐字节落盘，并算一个能钉住时点的哈希。
//!
//! 这里刻意不经过任何模型——2026-11-14 实测：`read_url` 读该平台详情页返回 0 字节文档，
//! 模型仍然「抄」出了不存在的数字。存证只能确定性抓取。

use crate::api::CatalogSource;
use crate::model::{Dataset, EvidenceEntry};
use crate::util::{now_iso8601, safe_id};
use crate::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct EvidenceOutcome {
    pub captured: usize,
    pub total: usize,
}

/// 给问题最多的前 `top` 个数据集各存一份原始响应。
pub fn capture_top(
    items: &[Dataset],
    top: usize,
    out_dir: &str,
    client: &dyn CatalogSource,
    threads: usize,
) -> Result<EvidenceOutcome> {
    let mut picked: Vec<&Dataset> = items.iter().filter(|d| d.has_preview == 1).collect();
    picked.sort_by(|a, b| {
        crate::scoring::score(b)
            .partial_cmp(&crate::scoring::score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    picked.truncate(top);

    let root = Path::new(out_dir).join("evidence");
    fs::create_dir_all(&root)?;
    let cursor = AtomicUsize::new(0);
    let entries: Mutex<Vec<Option<EvidenceEntry>>> = Mutex::new(vec![None; picked.len()]);

    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| loop {
                let i = cursor.fetch_add(1, Ordering::SeqCst);
                if i >= picked.len() {
                    break;
                }
                let d = picked[i];
                match capture_one(d, &root, client) {
                    Ok(e) => entries.lock().unwrap()[i] = Some(e),
                    Err(err) => eprintln!("  ERR {}: {err}", d.name),
                }
            });
        }
    });

    let entries: Vec<EvidenceEntry> = entries
        .into_inner()
        .unwrap()
        .into_iter()
        .flatten()
        .collect();
    let index = serde_json::json!({
        "capturedAt": now_iso8601(),
        "source": "浙江省公共数据开放平台接口原始响应（确定性抓取，不经模型）",
        "api": ["/dataCatalog/catalogDetail/{id}", "/dataCatalog/catalogDataPreview/{id}"],
        "total": picked.len(),
        "captured": entries.len(),
        "items": entries,
    });
    fs::write(
        Path::new(out_dir).join("evidence.json"),
        serde_json::to_string_pretty(&index)?,
    )?;
    Ok(EvidenceOutcome {
        captured: entries.len(),
        total: picked.len(),
    })
}

fn capture_one(d: &Dataset, root: &Path, client: &dyn CatalogSource) -> Result<EvidenceEntry> {
    let folder = root.join(safe_id(&d.id));
    fs::create_dir_all(&folder)?;
    let detail = client.detail_raw(&d.id)?;
    let preview = client.preview_raw(&d.id, 10)?;
    fs::write(folder.join("detail.json"), &detail)?;
    fs::write(folder.join("preview.json"), &preview)?;

    let mut h = Sha256::new();
    h.update(&detail);
    h.update(b"\n--split--\n");
    h.update(&preview);
    let sha = hex(&h.finalize());

    Ok(EvidenceEntry {
        id: d.id.clone(),
        name: d.name.clone(),
        fetched_at: now_iso8601(),
        detail_bytes: detail.len(),
        preview_bytes: preview.len(),
        sha256: sha,
        files: vec![
            format!("evidence/{}/detail.json", safe_id(&d.id)),
            format!("evidence/{}/preview.json", safe_id(&d.id)),
        ],
    })
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}
