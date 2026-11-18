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
use std::process::Command;

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

    let out = Command::new("lean").arg(&lean_path).output();
    let (passed, stdout) = match out {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr),
        ),
        Err(e) => (false, format!("无法执行 lean: {e}")),
    };
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
    Command::new("lean")
        .arg("--version")
        .output()
        .map(|_| ())
        .map_err(|_| Error::Other("找不到 lean，请先安装 elan/lean4".into()))
}
