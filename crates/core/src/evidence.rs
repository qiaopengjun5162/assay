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

/// 一份证据的复核结论。
pub struct Check {
    pub name: String,
    /// 文件读到了、哈希也算得出来，但和索引记录的不一致
    pub mismatched: bool,
}

pub struct CheckReport {
    pub checks: Vec<Check>,
    /// 索引里识别出的证据条目；0 说明索引格式不对，不能算通过
    pub listed: usize,
    pub matched: usize,
}

impl CheckReport {
    pub fn passed(&self) -> bool {
        self.listed > 0 && self.matched == self.listed
    }
}

/// 复核 `index_path` 索引里的每份证据：重读两个原始文件，按与 `capture_one`
/// 完全相同的字节顺序（detail ‖ 分隔符 ‖ preview）重算哈希，和索引值逐字符比对。
///
/// 这是 `capture_one` 的逆操作。索引接受 `evidence.json`（`items[]`）与
/// `web/data.json`（`datasets[].evidence`）两种形态。
pub fn verify_index(index_path: &Path, evidence_dir: &Path) -> Result<CheckReport> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(index_path)?)?;
    let mut checks = Vec::new();

    for e in entries(&value) {
        let dir = evidence_dir.join(safe_id(&e.id));
        let mismatched = match (
            fs::read(dir.join("detail.json")),
            fs::read(dir.join("preview.json")),
        ) {
            (Ok(detail), Ok(preview)) => {
                let mut h = Sha256::new();
                h.update(&detail);
                h.update(b"\n--split--\n");
                h.update(&preview);
                hex(&h.finalize()) != e.sha256
            }
            // 缺文件按「不一致」上报，比沉默略过安全
            _ => true,
        };
        checks.push(Check {
            name: e.name,
            mismatched,
        });
    }

    let listed = checks.len();
    let matched = checks.iter().filter(|c| !c.mismatched).count();
    Ok(CheckReport {
        checks,
        listed,
        matched,
    })
}

/// 从索引里取证据条目。`evidence.json` 的顶层数组本身就是条目，
/// `web/data.json` 的条目嵌在 `datasets[].evidence` 下；两者靠有没有 `sha256` 区分。
fn entries(value: &serde_json::Value) -> Vec<EvidenceEntry> {
    let Some(serde_json::Value::Array(list)) = value.get("items").or_else(|| value.get("datasets"))
    else {
        return Vec::new();
    };
    let nested = !list.first().is_some_and(|v| v.get("sha256").is_some());
    list.iter()
        .filter_map(|d| {
            if nested {
                d.get("evidence").filter(|e| !e.is_null())
            } else {
                Some(d)
            }
        })
        .filter_map(|e| serde_json::from_value::<EvidenceEntry>(e.clone()).ok())
        .filter(|e| !e.sha256.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, sha: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "name": format!("数据集 {id}"), "fetchedAt": "2026-11-14T21:33:45+08:00",
            "detailBytes": 3, "previewBytes": 5, "sha256": sha,
            "files": [format!("evidence/{}/detail.json", safe_id(id))],
        })
    }

    /// 写两份证据文件，返回 (索引路径, 证据目录)；`sha` 用 capture_one 的同一算法算。
    fn fixture(dir: &Path, tampered: bool) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = dir.join("evidence");
        let mut items = Vec::new();
        for id in ["aaa", "bbb"] {
            let folder = root.join(safe_id(id));
            fs::create_dir_all(&folder).unwrap();
            let mut detail = format!(r#"{{"id":"{id}"}}"#).into_bytes();
            let preview = format!(r#"{{"rows":["{id}"]}}"#).into_bytes();
            let mut h = Sha256::new();
            h.update(&detail);
            h.update(b"\n--split--\n");
            h.update(&preview);
            let sha = hex(&h.finalize());
            if tampered && id == "bbb" {
                detail.push(b' ');
            }
            fs::write(folder.join("detail.json"), &detail).unwrap();
            fs::write(folder.join("preview.json"), &preview).unwrap();
            items.push(entry(id, &sha));
        }
        let index = dir.join("evidence.json");
        fs::write(
            &index,
            serde_json::to_vec(&serde_json::json!({ "items": items })).unwrap(),
        )
        .unwrap();
        (index, root)
    }

    fn tmp(name: &str) -> std::path::PathBuf {
        let p =
            std::env::temp_dir().join(format!("dataproof-evidence-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn intact_evidence_passes() {
        let dir = tmp("intact");
        let (index, root) = fixture(&dir, false);
        let report = verify_index(&index, &root).unwrap();
        assert_eq!(report.listed, 2);
        assert_eq!(report.matched, 2);
        assert!(report.passed());
    }

    #[test]
    fn one_byte_extra_fails() {
        let dir = tmp("tampered");
        let (index, root) = fixture(&dir, true);
        let report = verify_index(&index, &root).unwrap();
        assert_eq!(report.matched, 1);
        assert!(!report.passed());
        assert!(report
            .checks
            .iter()
            .any(|c| c.mismatched && c.name == "数据集 bbb"));
    }

    #[test]
    fn missing_file_fails() {
        let dir = tmp("missing");
        let (index, root) = fixture(&dir, false);
        fs::remove_file(root.join("aaa").join("preview.json")).unwrap();
        let report = verify_index(&index, &root).unwrap();
        assert_eq!(report.matched, 1);
        assert!(!report.passed());
    }

    #[test]
    fn data_json_nested_entries_are_read() {
        let dir = tmp("nested");
        let (_, root) = fixture(&dir, false);
        let mut sha = Sha256::new();
        sha.update(b"");
        let index = dir.join("data.json");
        let ev = entry("ccc", &hex(&Sha256::digest(b"")));
        fs::write(
            &index,
            serde_json::to_vec(&serde_json::json!({
                "datasets": [
                    { "id": "aaa", "evidence": ev },
                    { "id": "bbb", "evidence": serde_json::Value::Null },
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let report = verify_index(&index, &root).unwrap();
        assert_eq!(report.listed, 1, "null evidence 与无证据条目都要跳过");
    }

    #[test]
    fn empty_index_is_not_a_pass() {
        let dir = tmp("empty");
        let index = dir.join("evidence.json");
        fs::write(&index, br#"{"items":[]}"#).unwrap();
        let report = verify_index(&index, &dir).unwrap();
        assert_eq!(report.listed, 0);
        assert!(!report.passed());
    }

    #[test]
    fn unreachable_index_is_an_error() {
        let dir = tmp("nope");
        assert!(verify_index(&dir.join("missing.json"), &dir).is_err());
    }
}
