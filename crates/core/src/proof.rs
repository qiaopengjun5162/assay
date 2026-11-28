//! 证明生成：把样例快照变成 Lean 定理，交给内核逐条穷举验证。
//!
//! 优先读证据包里的 `preview.json`——证的就是已经落盘并算过哈希的那份字节，
//! 而不是重新抓一遍。这样「证明对象」和「存证对象」是同一份数据。

use crate::api::CatalogSource;
use crate::lean;
use crate::model::{Dataset, ProofEntry};
use crate::util::safe_id;
use crate::{Error, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub struct ProofOutcome {
    pub theorems: usize,
    pub entries: usize,
    pub passed: bool,
    pub lean_stdout: String,
}

pub fn prove_top(
    items: &[Dataset],
    top: usize,
    out_dir: &str,
    client: &dyn CatalogSource,
) -> Result<ProofOutcome> {
    prove_top_with(items, top, out_dir, client, run_lean)
}

/// 执行 Lean 内核。抽成函数是让「生成」与「验证」在测试里能分开：
/// 生成逻辑要能离线校验，不该因为机器上没装 lean 就一行都测不了。
fn run_lean(path: &Path) -> (bool, String) {
    match lean::lean_command().arg(path).output() {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr),
        ),
        Err(e) => (false, format!("无法执行 lean: {e}")),
    }
}

fn prove_top_with<F>(
    items: &[Dataset],
    top: usize,
    out_dir: &str,
    client: &dyn CatalogSource,
    verify: F,
) -> Result<ProofOutcome>
where
    F: Fn(&Path) -> (bool, String),
{
    let mut picked: Vec<&Dataset> = items.iter().filter(|d| d.has_preview == 1).collect();
    picked.sort_by(|a, b| {
        crate::scoring::score(b)
            .partial_cmp(&crate::scoring::score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    picked.truncate(top);

    let mut blocks = Vec::new();
    let mut theorems = Vec::new();
    let mut meta = Vec::new();

    for (k, d) in picked.iter().enumerate() {
        let rows = match load_rows(d, out_dir, client) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };
        let header = &rows[0];
        let body = &rows[1..];
        if body.is_empty() {
            continue;
        }
        let ncols = header.len();
        blocks.push(lean::rows_block(
            &d.name,
            k,
            body,
            ncols,
            d.empty_rate * 100.0,
        ));
        for c in 0..ncols {
            let count = crate::matrix::per_col_empties(body, ncols)[c];
            theorems.push(lean::count_empty_theorem(k, c, count));
        }
        meta.push(ProofEntry {
            id: d.id.clone(),
            name: d.name.clone(),
            index: k,
            dept: d.dept.clone(),
            rows: body.len(),
            cols: ncols,
            empty_rate: d.empty_rate,
            all_empty_cols: d.all_empty_cols,
            missing_fields: d.missing_fields.clone(),
            sha256: sha_rows(&rows),
            source: Some(if evidence_exists(d, out_dir) {
                "snapshot".into()
            } else {
                "live".into()
            }),
        });
    }

    let module = lean::module(&blocks, &theorems);
    let lean_path = Path::new(out_dir).join("TopProofs.lean");
    fs::write(&lean_path, &module)?;

    let (passed, stdout) = verify(&lean_path);
    fs::write(
        Path::new(out_dir).join("verify.txt"),
        if passed {
            "PASS\n".to_string()
        } else {
            format!("FAIL\n{stdout}")
        },
    )?;
    fs::write(
        Path::new(out_dir).join("proofs.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;

    let mut md = vec![
        format!("# 榜单前 {} 名的机器可复核证明", meta.len()),
        String::new(),
        format!(
            "运行 `lean TopProofs.lean` 的结果：**{}**",
            if passed { "PASS" } else { "FAIL" }
        ),
        String::new(),
        "| # | 数据集 | 样例 | 空值率 | 定理数 | 快照 SHA-256 |".into(),
        "|---|---|---|---|---|---|".into(),
    ];
    for m in &meta {
        md.push(format!(
            "| {} | {} | {}×{} | {:.1}% | {} | `{}…` |",
            m.index,
            m.name.chars().take(26).collect::<String>(),
            m.rows,
            m.cols,
            m.empty_rate * 100.0,
            m.cols,
            &m.sha256[..16]
        ));
    }
    fs::write(Path::new(out_dir).join("proofs.md"), md.join("\n") + "\n")?;

    Ok(ProofOutcome {
        theorems: theorems.len(),
        entries: meta.len(),
        passed,
        lean_stdout: stdout,
    })
}

fn evidence_exists(d: &Dataset, out_dir: &str) -> bool {
    Path::new(out_dir)
        .join("evidence")
        .join(safe_id(&d.id))
        .join("preview.json")
        .exists()
}

fn load_rows(d: &Dataset, out_dir: &str, client: &dyn CatalogSource) -> Option<Vec<Vec<String>>> {
    let path = Path::new(out_dir)
        .join("evidence")
        .join(safe_id(&d.id))
        .join("preview.json");
    if let Ok(bytes) = fs::read(&path) {
        // 快照优先，且由适配器解析——证明对象必须和存证对象是同一份字节。
        if let Ok(rows) = client.rows_from_raw(&bytes) {
            return Some(rows);
        }
    }
    client.rows(&d.id, 10).ok()
}

/// 与 Python `json.dumps(rows, ensure_ascii=False)` 同格式：分隔符带空格。
fn python_json(rows: &[Vec<String>]) -> String {
    let parts: Vec<String> = rows
        .iter()
        .map(|r| {
            let cells: Vec<String> = r
                .iter()
                .map(|c| serde_json::to_string(c).unwrap_or_else(|_| "\"\"".into()))
                .collect();
            format!("[{}]", cells.join(", "))
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

fn sha_rows(rows: &[Vec<String>]) -> String {
    let mut h = Sha256::new();
    h.update(python_json(rows).as_bytes());
    crate::evidence::hex(&h.finalize())
}

/// 供 CLI 使用：`lean` 不在 PATH 时给出可操作的错误。
pub fn require_lean() -> Result<()> {
    lean::lean_command()
        .arg("--version")
        .output()
        .map(|_| ())
        .map_err(|_| Error::Other("找不到 lean，请先安装 elan/lean4".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DatasetMeta;
    use crate::model::CatalogRecord;
    use serde_json::{json, Value};
    use std::path::PathBuf;

    /// 只提供快照解析：这些测试把 `preview.json` 直接写到临时目录，
    /// 走「快照优先」路径，不需要任何网络。
    struct SnapshotSource;

    impl CatalogSource for SnapshotSource {
        fn platform(&self) -> &str {
            "测试平台"
        }
        fn catalog_page(&self, _: usize) -> Result<(usize, usize, Vec<CatalogRecord>)> {
            Err(Error::Other("测试用不到目录".into()))
        }
        fn catalog_all(&self, _: usize) -> Result<Vec<CatalogRecord>> {
            Err(Error::Other("测试用不到目录".into()))
        }
        fn meta(&self, _: &str) -> Result<DatasetMeta> {
            Err(Error::Other("测试用不到元信息".into()))
        }
        fn rows(&self, _: &str, _: usize) -> Result<Vec<Vec<String>>> {
            Err(Error::Other("测试只走快照，不应联网".into()))
        }
        fn rows_from_raw(&self, raw: &[u8]) -> Result<Vec<Vec<String>>> {
            let v: Value = serde_json::from_slice(raw)
                .map_err(|e| Error::Other(format!("测试快照非法 JSON: {e}")))?;
            Ok(crate::api::preview_rows(&v))
        }
        fn detail_raw(&self, _: &str) -> Result<Vec<u8>> {
            Err(Error::Other("测试用不到原始详情".into()))
        }
        fn preview_raw(&self, _: &str, _: usize) -> Result<Vec<u8>> {
            Err(Error::Other("测试用不到原始预览".into()))
        }
    }

    fn tmpdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "assay-proof-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn dataset(id: &str, has_preview: i64) -> Dataset {
        Dataset {
            id: id.into(),
            name: format!("数据集{id}"),
            has_preview,
            declared_cols: 2,
            sample_cols: 2,
            sample_rows: 2,
            empty_rate: 0.25,
            ..Default::default()
        }
    }

    fn put_snapshot(dir: &Path, d: &Dataset, data: Value) {
        let p = dir
            .join("evidence")
            .join(safe_id(&d.id))
            .join("preview.json");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, serde_json::to_vec(&data).unwrap()).unwrap();
    }

    fn sample_rows() -> Value {
        json!({"result": {"data": [
            ["字段A", "字段B"],
            ["", "2"],
            ["v", null]
        ]}})
    }

    /// 生成的定理、元信息、快照来源都必须来自同一份落盘字节。
    /// 这里用假 Lean，机器没装 lean 也能验证生成侧。
    #[test]
    fn prove_top_generates_from_snapshot() {
        let dir = tmpdir("ok");
        let d = dataset("abc", 1);
        put_snapshot(&dir, &d, sample_rows());

        let out = prove_top_with(
            &[d.clone(), dataset("no-preview", 0)],
            10,
            dir.to_str().unwrap(),
            &SnapshotSource,
            |_| (true, "ok".into()),
        )
        .unwrap();
        assert_eq!(out.entries, 1, "没有样例的数据集不该进入证明");
        assert_eq!(out.theorems, 2, "两列应生成两条计数定理");
        assert!(out.passed);
        assert_eq!(out.lean_stdout, "ok");

        let proofs: Vec<serde_json::Value> =
            serde_json::from_slice(&fs::read(dir.join("proofs.json")).unwrap()).unwrap();
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0]["source"], "snapshot");
        assert_eq!(proofs[0]["id"], "abc");
        assert_eq!(proofs[0]["rows"], 2);
        assert_eq!(proofs[0]["sha256"].as_str().unwrap().len(), 64);

        assert!(
            String::from_utf8_lossy(&fs::read(dir.join("TopProofs.lean")).unwrap())
                .contains("theorem")
        );
        assert_eq!(
            fs::read_to_string(dir.join("verify.txt")).unwrap(),
            "PASS\n"
        );
        assert!(fs::read_to_string(dir.join("proofs.md"))
            .unwrap()
            .contains("PASS"));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Lean 失败时要把内核输出原样落盘，而不是只写 FAIL 让人猜。
    #[test]
    fn prove_top_keeps_lean_output_on_failure() {
        let dir = tmpdir("fail");
        let d = dataset("abc", 1);
        put_snapshot(&dir, &d, sample_rows());

        let out = prove_top_with(&[d], 10, dir.to_str().unwrap(), &SnapshotSource, |_| {
            (false, "error: unsolved goals".into())
        })
        .unwrap();
        assert!(!out.passed);
        assert_eq!(out.lean_stdout, "error: unsolved goals");
        let verify = fs::read_to_string(dir.join("verify.txt")).unwrap();
        assert!(verify.starts_with("FAIL\nerror: unsolved goals"));
        fs::remove_dir_all(&dir).unwrap();
    }

    /// 对应线上 109/500：平台只给表头，没有可证命题，跳过而不是硬凑定理。
    #[test]
    fn prove_top_skips_header_only_preview() {
        let dir = tmpdir("header");
        let d = dataset("abc", 1);
        put_snapshot(&dir, &d, json!({"result": {"data": [["字段A", "字段B"]]}}));

        let out = prove_top_with(&[d], 10, dir.to_str().unwrap(), &SnapshotSource, |_| {
            (true, "ok".into())
        })
        .unwrap();
        assert_eq!(out.entries, 0);
        assert_eq!(out.theorems, 0);
        let proofs: Vec<serde_json::Value> =
            serde_json::from_slice(&fs::read(dir.join("proofs.json")).unwrap()).unwrap();
        assert!(proofs.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
