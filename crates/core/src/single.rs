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

/// 站点根地址，与 `crates/web/index.html` 的 canonical / og:url 保持一致。
/// 报告会被下载到本机，回链必须是绝对 URL：相对路径在 file:// 下没有共同根目录。
pub const SITE_ROOT: &str = "https://qiaopengjun5162.github.io/assay/";

/// hash 路由里的 id 必须逐字节百分号编码。id 常带 `+` `/` `=`：
/// `+` 不编码会在 URL 里变成空格，`=` 和 `/` 会改变 hash 的解析结果。
/// 与前端 `urlencoding()` 同一套规则（RFC 3986 unreserved 字符原样保留）。
pub fn hash_id(id: &str) -> String {
    id.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// 报告的「在线详情」回链。门禁按这个函数逐份核对，别改格式。
pub fn detail_url(id: &str) -> String {
    format!("{SITE_ROOT}#/detail/{}", hash_id(id))
}

/// 报告的「在线证明」回链。没有证明的条目页面会说明原因，所以这里不判断覆盖范围。
pub fn proof_url(id: &str) -> String {
    format!("{SITE_ROOT}#/proof/{}", hash_id(id))
}

pub struct Evidence {
    pub detail_sha: String,
    pub preview_sha: String,
}

/// 从指定证据根目录读证据哈希与样例行（存在才读，不存在返回 None）。
pub fn load_evidence_dir(
    evidence_dir: &Path,
    id: &str,
) -> (Option<Evidence>, Option<Vec<Vec<String>>>) {
    let dir = evidence_dir.join(safe_id(id));
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

/// 从 `<out>/evidence` 读。保留这个入口给早期调用方。
pub fn load_evidence(out_dir: &str, id: &str) -> (Option<Evidence>, Option<Vec<Vec<String>>>) {
    load_evidence_dir(&Path::new(out_dir).join("evidence"), id)
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
    render_top_with_evidence(
        items,
        top,
        &Path::new(out_dir).join("evidence"),
        dest,
        explanations,
    )
}

/// 同上，但证据根目录可指定。
///
/// 为什么需要：11-14 的 `out-full/evidence` 只有 100 份，11-18 重抓的 500 份
/// 入库在 `web/evidence`。报告里的哈希必须来自**入库的那批**，否则读者逐个核对全部失败。
pub fn render_top_with_evidence(
    items: &[Dataset],
    top: usize,
    evidence_dir: &Path,
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
        let (ev, rows) = load_evidence_dir(evidence_dir, &d.id);
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
    // 报告会被下载/转发，离开站点后读者没法回到这一条。两条回链放在最上面：
    // 详情页看判定过程，证明页看内核复核。URL 用绝对地址，file:// 下也能点。
    s.push_str(&format!("- 在线详情：{}\n", detail_url(&d.id)));
    s.push_str(&format!("- 在线证明：{}\n", proof_url(&d.id)));
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

/// 报告里写的两个哈希（抽不出来就是 `None`）。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReportHashes {
    pub detail: Option<String>,
    pub preview: Option<String>,
}

/// 从报告正文里抽 SHA-256。
///
/// 认的格式就是 `render` 写出来的那两行：
/// `- detail.json SHA-256：\`<64 位十六进制>\``。
/// 用字符串查找而不是正则：格式只有一处产生，正则只会多一个依赖和一个
/// 更难读的失败信息。
///
/// 抽不到就返回 `None`——**不要**在这里判失败，让调用方区分「报告缺这一段」
/// 和「报告不存在」。
#[must_use]
pub fn parse_report_hashes(md: &str) -> ReportHashes {
    let pick = |name: &str| -> Option<String> {
        let pat = format!("{name} SHA-256：`");
        let start = md.find(&pat)? + pat.len();
        let rest = &md[start..];
        let end = rest.find('`')?;
        let hex = rest.get(..end)?;
        (hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| hex.to_string())
    };
    ReportHashes {
        detail: pick("detail.json"),
        preview: pick("preview.json"),
    }
}

/// 一份报告的复核结果。
pub struct ReportCheck {
    /// 报告文件名（`<safe_id>.md`）。
    pub name: String,
    /// `None` = 对得上。否则是一句人话说明哪里不对。
    pub problem: Option<String>,
}

/// 复核「入库的报告」与「入库的证据」是不是同一批。
///
/// 为什么需要：`web/reports/*.md` 里印着两个 SHA-256，读者会拿它去核对
/// `web/evidence/` 里的文件。报告和证据**必须来自同一次抓取**。用错了 `--out`
/// 就会生成一批哈希指向别处的报告——每份报告看着都完整，而读者逐份核对全部失败。
/// 这个坑不是假想的：用 11-14 的 `out-full` 给 11-18 的证据补报告，就会这样。
///
/// 判据来自 `data.json`：它列出哪些数据集有证据；对这些数据集，报告必须存在、
/// 且其中两个哈希等于证据文件重算的结果。
///
/// # Errors
///
/// 索引读不了 / 不是合法 JSON / `datasets` 不是数组时返回错误——**不静默返回空**，
/// 空的结果会被读成「全部通过」。
pub fn verify_reports(
    index: &Path,
    reports_dir: &Path,
    evidence_dir: &Path,
) -> Result<Vec<ReportCheck>> {
    let raw = std::fs::read(index)?;
    let v: serde_json::Value = serde_json::from_slice(&raw)?;
    let rows = v
        .get("datasets")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| crate::Error::Other(format!("{} 里没有 datasets 数组", index.display())))?;

    let mut out = Vec::new();
    for row in rows {
        // 没有证据就没有报告，不算问题：报告集合与证据集合同口径。
        if row.get("evidence").is_none_or(serde_json::Value::is_null) {
            continue;
        }
        let Some(id) = row.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let safe = safe_id(id);
        let name = format!("{safe}.md");
        let path = reports_dir.join(&name);
        let Ok(md) = std::fs::read_to_string(&path) else {
            out.push(ReportCheck {
                name,
                problem: Some("报告不存在（证据有、报告没有，集合不一致）".into()),
            });
            continue;
        };
        let got = parse_report_hashes(&md);
        // 回链也是产物的一部分：报告离开站点后，读者靠这两行回到详情页和证明页。
        // 只对有证据的报告校验——报告集合与证据集合同口径。
        let missing_link = if !md.contains(&format!("- 在线详情：{}", detail_url(id))) {
            Some(format!("报告缺在线详情回链（id={id}）"))
        } else if !md.contains(&format!("- 在线证明：{}", proof_url(id))) {
            Some(format!("报告缺在线证明回链（id={id}）"))
        } else {
            None
        };
        let dir = evidence_dir.join(&safe);
        let actual = |f: &str| -> Option<String> {
            std::fs::read(dir.join(f))
                .ok()
                .map(|b| crate::evidence::hex(&Sha256::digest(&b)))
        };
        let problem = match (
            got.detail,
            got.preview,
            actual("detail.json"),
            actual("preview.json"),
        ) {
            (None, _, _, _) => Some("报告里没有 detail.json 的哈希（旧格式或被人改过）".into()),
            (_, None, _, _) => Some("报告里没有 preview.json 的哈希".into()),
            (Some(_), Some(_), None, _) => Some(format!("证据缺 detail.json：{}", dir.display())),
            (_, _, _, None) => Some(format!("证据缺 preview.json：{}", dir.display())),
            (Some(d), Some(p), Some(ad), Some(ap)) => {
                if d != ad {
                    Some(format!(
                        "detail.json 哈希对不上：报告 {d}，实际 {ad}（不是同一批抓取）"
                    ))
                } else if p != ap {
                    Some(format!(
                        "preview.json 哈希对不上：报告 {p}，实际 {ap}（不是同一批抓取）"
                    ))
                } else {
                    None
                }
            }
        }
        .or(missing_link);
        out.push(ReportCheck { name, problem });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &str = "806da7bc0abb3800ff668e2d70bac762b7d9e1a4f01014db36c55dfe5ba7cb18";
    const P: &str = "df8f095ed7fadf10ddac578a7206e92f354b4100aa2d9f8793de2a5e5a9b0f30";

    #[test]
    fn parses_both_hashes_from_a_real_report() {
        let md = format!(
            "## 证据\n\n- detail.json SHA-256：`{D}`\n- preview.json SHA-256：`{P}`\n\n> 哈希\n"
        );
        let h = parse_report_hashes(&md);
        assert_eq!(h.detail.as_deref(), Some(D));
        assert_eq!(h.preview.as_deref(), Some(P));
    }

    #[test]
    fn missing_section_yields_none_not_a_fake_hash() {
        let md = "## 结论\n\n这一份没有证据段。\n";
        let h = parse_report_hashes(md);
        assert_eq!(
            h.detail, None,
            "没有就别编一个，调用方要能区分「缺」和「错」"
        );
        assert_eq!(h.preview, None);
    }

    /// 长度不对的串不能当哈希用——否则比对会变成「两个都短，所以相等」。
    #[test]
    fn rejects_short_or_non_hex_values() {
        let md = "- detail.json SHA-256：`abcd`\n- preview.json SHA-256：`zzzz`\n";
        let h = parse_report_hashes(md);
        assert_eq!(h.detail, None, "4 位不是 SHA-256");
        assert_eq!(h.preview, None, "非十六进制不接受");
    }

    /// 报告里有两段证据时，第一条才算（`render` 不会产生这种输入，但不能错取）。
    #[test]
    fn takes_the_first_occurrence() {
        let md = format!(
            "- detail.json SHA-256：`{D}`\n- detail.json SHA-256：`{P}`\n- preview.json SHA-256：`{P}`\n"
        );
        assert_eq!(parse_report_hashes(&md).detail.as_deref(), Some(D));
    }

    // id 里的 '+' 若不编码，会被浏览器当空格，深链直接落在「找不到这个数据集」。
    #[test]
    fn backlinks_encode_id_bytes_like_the_frontend() {
        assert_eq!(
            hash_id("ab+cd/ef=中文"),
            "ab%2Bcd%2Fef%3D%E4%B8%AD%E6%96%87"
        );
        assert_eq!(
            detail_url("a+b=="),
            format!("{SITE_ROOT}#/detail/a%2Bb%3D%3D")
        );
        assert_eq!(
            proof_url("a+b=="),
            format!("{SITE_ROOT}#/proof/a%2Bb%3D%3D")
        );
    }
}
