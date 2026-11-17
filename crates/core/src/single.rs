//! 单数据集报告：把一条记录渲染成可交付的 Markdown。
//!
//! 场景：数据上架审核时，需要的不是整张榜单，而是「这一份数据集」的
//! 字段差异、样例质量、证据哈希与 AI 解读——一页纸能贴进评审流程。

use crate::api::preview_rows;
use crate::model::{Dataset, Explanation};
use crate::scoring::{classify, score};
use crate::util::safe_id;
use crate::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

pub struct Evidence {
    pub detail_sha: String,
    pub preview_sha: String,
}

/// 从证据包里读证据哈希与样例行（存在才读，不存在返回 None）。
pub fn load_evidence(out_dir: &str, id: &str) -> (Option<Evidence>, Option<Vec<Vec<String>>>) {
    let dir = Path::new(out_dir).join("evidence").join(safe_id(id));
    let hash_of = |name: &str| {
        std::fs::read(dir.join(name))
            .ok()
            .map(|b| crate::evidence::hex(&Sha256::digest(&b)))
    };
    let ev = match (hash_of("detail.json"), hash_of("preview.json")) {
        (Some(detail_sha), Some(preview_sha)) => Some(Evidence {
            detail_sha,
            preview_sha,
        }),
        _ => None,
    };
    let rows = std::fs::read(dir.join("preview.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .map(|v| preview_rows(&v))
        .filter(|r| !r.is_empty());
    (ev, rows)
}

/// 读解释层产物，只留通过护栏的条目（`status == "pass"`）。
///
/// 未通过的解读留在 `explanations.json` 里备查，但不进报告——页面上也是同样口径。
pub fn load_explanations(out_dir: &str) -> HashMap<String, String> {
    std::fs::read(Path::new(out_dir).join("explanations.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| serde_json::from_value::<Vec<Explanation>>(v["items"].clone()).ok())
        .map(|list| {
            list.into_iter()
                .filter(|e| e.status == "pass" && !e.text.is_empty())
                .map(|e| (e.id, e.text))
                .collect()
        })
        .unwrap_or_default()
}

/// 批量渲染问题最多的前 `top` 个数据集，写到 `dest/<safe_id>.md`。
///
/// 与 `proof` / `evidence` 同一选择口径（有样例、按问题分降序），
/// 这样报告、证明、证据三份产物覆盖的是同一批数据集。
pub fn render_top(
    items: &[Dataset],
    top: usize,
    out_dir: &str,
    dest: &str,
    explanations: &HashMap<String, String>,
) -> Result<RenderOutcome> {
    let mut picked: Vec<&Dataset> = items.iter().filter(|d| d.has_preview == 1).collect();
    picked.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    picked.truncate(top);

    std::fs::create_dir_all(dest)?;
    let mut written = 0;
    for d in &picked {
        let (ev, rows) = load_evidence(out_dir, &d.id);
        let md = render(
            d,
            d.lag_days,
            ev.as_ref(),
            rows.as_deref(),
            explanations.get(&d.id).map(String::as_str),
        );
        std::fs::write(Path::new(dest).join(format!("{}.md", safe_id(&d.id))), md)?;
        written += 1;
    }
    Ok(RenderOutcome { written })
}

pub struct RenderOutcome {
    pub written: usize,
}

pub fn render(
    d: &Dataset,
    lag: Option<i64>,
    ev: Option<&Evidence>,
    rows: Option<&[Vec<String>]>,
    explain: Option<&str>,
) -> String {
    let mut s = String::new();
    s.push_str(&format!("# {}\n\n", d.name));
    s.push_str(&format!("- 数据集 ID：`{}`\n", d.id));
    s.push_str(&format!(
        "- 提供部门：{}\n",
        d.dept.as_deref().unwrap_or("未标注")
    ));
    s.push_str(&format!(
        "- 开放类型：{}\n",
        d.open_type.as_deref().unwrap_or("未标注")
    ));
    s.push_str(&format!(
        "- 数据量：{}\n",
        d.data_size.as_deref().unwrap_or("未标注")
    ));
    if let Some(days) = lag {
        s.push_str(&format!("- 距基准日：{} 天\n", days));
    }
    s.push('\n');

    s.push_str("## 体检结论\n\n");
    s.push_str(&format!("- 字段一致性：{}\n", classify(d).as_str()));
    s.push_str(&format!(
        "- 声明列数 {} / 样例列数 {} / 样例行数 {}\n",
        d.declared_cols, d.sample_cols, d.sample_rows
    ));
    s.push_str(&format!("- 样例空值率：{:.2}%\n", d.empty_rate * 100.0));
    s.push_str(&format!("- 整列为空：{} 列\n", d.all_empty_cols));
    s.push_str(&format!("- 字段说明长度：{} 字\n", d.desc_len));
    s.push_str(&format!("- 问题分：{:.2}（仅用于排序）\n", score(d)));
    if !d.missing_fields.is_empty() {
        s.push_str(&format!(
            "- 声明里有、样例里没有：{}\n",
            d.missing_fields.join("、")
        ));
    }
    if !d.extra_fields.is_empty() {
        s.push_str(&format!(
            "- 样例里有、声明里没有：{}\n",
            d.extra_fields.join("、")
        ));
    }
    s.push('\n');

    if let Some(rows) = rows {
        s.push_str("## 样例（原始快照）\n\n");
        let take = rows.len().min(6);
        for r in &rows[..take] {
            s.push_str(&format!("| {} |\n", r.join(" | ")));
        }
        s.push('\n');
    }

    if let Some(t) = explain {
        s.push_str("## AI 解读（数字已核对）\n\n");
        s.push_str(t);
        s.push_str("\n\n");
    }

    if let Some(e) = ev {
        s.push_str("## 证据\n\n");
        s.push_str(&format!("- detail.json SHA-256：`{}`\n", e.detail_sha));
        s.push_str(&format!("- preview.json SHA-256：`{}`\n", e.preview_sha));
        s.push_str("\n> 哈希对应抓取时点的原始响应字节，可重新计算核对。\n");
    }
    s
}
