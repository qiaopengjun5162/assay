//! 新闻条目数据源：直接读上游导出的 `PublicNewsItem` 数组。
//!
//! 与 [`crate::fixture_source`] 的分工：
//!
//! - `fixture_source` 读 assay 自有格式（camelCase、必填 `id`、带 `declaredColumns` 包装）；
//! - 本模块读**上游原样数组**（snake_case、无 `id`、`Option` 字段），`id` 由 assay 派生。
//!
//! 这样上游（qunmind）不需要为了兼容 assay 改数据结构，assay 也不用依赖它的 crate——
//! 两边只共享一个 JSON 形状，是最松的耦合。
//!
//! ## 声明列只取 7 个
//!
//! 日报模板承诺的是「标题 / 来源 / 链接 / 摘要 / 作者 / 时间 / 分类」。上游的
//! `score` / `comments` / `ai_score` 是排序用的内部量，不是对读者的承诺；把它们算进
//! 声明列，会把「内部字段为空」误判成「日报缺字段」——那是口径错误，不是数据问题。
//!
//! ## 时间格式
//!
//! 上游用 `to_rfc3339()` 产出 `2026-11-25T08:00:00+00:00`。`aggregate::parse_update_time`
//! 按固定位置读（前 10 位日期、`11..19` 时分秒），所以 RFC3339 能直接解析，不需要归一化。
//! 这条行为有单测钉住——它一旦被改成「只认空格分隔」，超期检查会静默失效。

use crate::api::{CatalogSource, DatasetMeta};
use crate::model::CatalogRecord;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// 写进产物与页面的平台名，标明这批条目的来源形状。
const PLATFORM: &str = "公众号日报·新闻条目（qunmind 形状）";

/// 日报模板承诺的字段。顺序即表头顺序，也是 `missingFields` 的对齐基准。
const DECLARED: [&str; 7] = [
    "title",
    "source",
    "url",
    "summary",
    "author",
    "published_at",
    "category",
];

/// 上游一条新闻。字段名与 `qunmind::source::PublicNewsItem` 逐字对齐。
///
/// 不加 `deny_unknown_fields`：上游以后加字段不该让体检崩，那些字段只是不参与判定。
#[derive(Debug, Clone, Deserialize, Serialize)]
struct NewsItem {
    #[serde(default)]
    source: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    category: Option<String>,
}

/// 从上游新闻条目数组读的只读数据源。
pub struct NewsSource {
    by_id: BTreeMap<String, NewsItem>,
    catalog: Vec<CatalogRecord>,
}

impl NewsSource {
    /// 从一份 `PublicNewsItem` 数组构造数据源。
    ///
    /// # Errors
    ///
    /// - 文件读不到 / 不是合法 JSON 数组；
    /// - 某条既没有 `url` 也没有 `title`，无法派生稳定 id；
    /// - 两条派生出同一个 id（同一条 url 出现两次会让证据互相覆盖，宁可报错）。
    pub fn open(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .map_err(|e| Error::Other(format!("读取新闻条目 {}: {e}", path.display())))?;
        let items: Vec<NewsItem> = serde_json::from_str(&text)
            .map_err(|e| Error::Other(format!("新闻条目不是合法 JSON 数组: {e}")))?;

        let mut by_id = BTreeMap::new();
        let mut catalog = Vec::with_capacity(items.len());
        for item in items {
            let id = derive_id(&item)?;
            if by_id.contains_key(&id) {
                return Err(Error::Other(format!(
                    "新闻条目重复：id={id}（同一条 url 出现两次会让证据互相覆盖）"
                )));
            }
            // 名称优先用标题；没有标题时退回 url，至少让人知道是哪条。
            let display = if item.title.trim().is_empty() {
                item.url.clone()
            } else {
                item.title.clone()
            };
            catalog.push(CatalogRecord {
                id: id.clone(),
                source_name: display,
                // 上游的 source 是「哪家媒体」，正好对应部门/来源维度。
                private_dept: non_empty(&item.source),
                open_type: None,
                // 新闻是日更，周期固定；这样停更词表检查也能复用。
                update_cycle: Some("按日".to_string()),
                data_size: None,
                area_type: None,
                // RFC3339 原样进 update_time，parse_update_time 能按位置解析。
                update_time: item.published_at.clone(),
                region_id: None,
            });
            by_id.insert(id, item);
        }

        Ok(Self { by_id, catalog })
    }

    /// 一条条目 → 样例表格：第一行是「实际非空」的字段，第二行是值。
    ///
    /// 表头**不是**声明全集，而是条目真正给了的字段——否则 audit 发现不了
    /// 「模板说有、实际没给」。
    fn table(&self, item: &NewsItem) -> Vec<Vec<String>> {
        let pairs: [(&str, Option<&str>); 7] = [
            ("title", Some(item.title.as_str())),
            ("source", Some(item.source.as_str())),
            ("url", Some(item.url.as_str())),
            ("summary", item.summary.as_deref()),
            ("author", item.author.as_deref()),
            ("published_at", item.published_at.as_deref()),
            ("category", item.category.as_deref()),
        ];
        let mut header = Vec::new();
        let mut values = Vec::new();
        for (name, v) in pairs {
            if let Some(s) = v {
                if !s.trim().is_empty() {
                    header.push(name.to_string());
                    values.push(s.to_string());
                }
            }
        }
        vec![header, values]
    }

    fn item(&self, id: &str) -> Result<&NewsItem> {
        self.by_id
            .get(id)
            .ok_or_else(|| Error::Other(format!("新闻条目里没有 id: {id}")))
    }
}

/// 把可选字符串的空白值收成 `None`，避免空字符串被当成一个「来源」统计。
fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// 派生稳定 id。
///
/// 优先用 `url`；没有 url 时退化到 `title|published_at`。
/// 取 sha256 前 16 个 hex：够短、可安全落盘、500 条量级碰撞概率可忽略。
/// **不用** `safe_id(url)`：长链接会被截断到 64 字符而撞车，中文标题更会被替换成
/// 一串下划线；哈希对这两点都免疫。
fn derive_id(item: &NewsItem) -> Result<String> {
    let key = if !item.url.trim().is_empty() {
        item.url.trim().to_string()
    } else {
        let title = item.title.trim();
        if title.is_empty() {
            return Err(Error::Other(
                "新闻条目既没有 url 也没有 title，无法派生稳定 id".into(),
            ));
        }
        match item.published_at.as_deref().map(str::trim) {
            Some(d) if !d.is_empty() => format!("{title}|{d}"),
            _ => title.to_string(),
        }
    };
    let digest = Sha256::digest(key.as_bytes());
    // `{:x}` 输出 64 个 hex；取 `n_` + 16 位 = 18 字符。
    Ok(format!("n_{digest:x}")[..18].to_string())
}

impl CatalogSource for NewsSource {
    fn platform(&self) -> &str {
        PLATFORM
    }

    fn catalog_page(&self, current: usize) -> Result<(usize, usize, Vec<CatalogRecord>)> {
        const PER_PAGE: usize = 100;
        let total = self.catalog.len();
        let pages = total.div_ceil(PER_PAGE);
        let page = if current == 0 || current > pages {
            Vec::new()
        } else {
            self.catalog
                .iter()
                .skip((current - 1) * PER_PAGE)
                .take(PER_PAGE)
                .cloned()
                .collect()
        };
        Ok((total, pages, page))
    }

    fn catalog_all(&self, _threads: usize) -> Result<Vec<CatalogRecord>> {
        Ok(self.catalog.clone())
    }

    fn meta(&self, id: &str) -> Result<DatasetMeta> {
        let item = self.item(id)?;
        Ok(DatasetMeta {
            columns: DECLARED.iter().map(|s| s.to_string()).collect(),
            has_preview: true,
            description_len: item.summary.as_deref().unwrap_or("").chars().count(),
        })
    }

    fn rows(&self, id: &str, _size: usize) -> Result<Vec<Vec<String>>> {
        Ok(self.table(self.item(id)?))
    }

    /// 从已存证的原始字节解析表格——证明必须针对快照，不能重新读上游。
    fn rows_from_raw(&self, raw: &[u8]) -> Result<Vec<Vec<String>>> {
        let v: Value = serde_json::from_slice(raw)
            .map_err(|e| Error::Other(format!("新闻条目快照不是合法 JSON: {e}")))?;
        let header = v["columns"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| c.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default();
        let mut out = vec![header];
        if let Some(rows) = v["rows"].as_array() {
            for r in rows {
                out.push(
                    r.as_array()
                        .map(|a| {
                            a.iter()
                                .map(|c| c.as_str().unwrap_or("").to_string())
                                .collect()
                        })
                        .unwrap_or_default(),
                );
            }
        }
        Ok(out)
    }

    fn detail_raw(&self, id: &str) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self.item(id)?)?)
    }

    fn preview_raw(&self, id: &str, _size: usize) -> Result<Vec<u8>> {
        let table = self.table(self.item(id)?);
        // rows 必须是「行的数组」：table[1] 是单行，要再包一层，
        // 否则 rows_from_raw 会把每个单元格当成一行。
        let body = match table.get(1) {
            Some(row) => vec![row.clone()],
            None => Vec::new(),
        };
        let v = serde_json::json!({
            "columns": table.first().cloned().unwrap_or_default(),
            "rows": body,
        });
        Ok(serde_json::to_vec(&v)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_fixture(tag: &str, body: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "assay-news-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        p
    }

    const SAMPLE: &str = r#"[
        {"source":"36氪","title":"完整条目","url":"https://a/1","summary":"一段摘要",
         "author":"张三","published_at":"2026-11-25T08:00:00+00:00","category":"模型"},
        {"source":"某站","title":"缺字段条目","url":"https://a/2",
         "published_at":"2025-03-01T00:00:00Z"}
    ]"#;

    #[test]
    fn derives_stable_id_from_url() {
        let p = write_fixture("stable", SAMPLE);
        let a = NewsSource::open(&p).unwrap();
        let b = NewsSource::open(&p).unwrap();
        let (_, _, pa) = a.catalog_page(1).unwrap();
        let (_, _, pb) = b.catalog_page(1).unwrap();
        assert_eq!(pa[0].id, pb[0].id, "同一份输入必须派生出同一个 id");
        assert!(pa[0].id.starts_with("n_"));
        assert_eq!(pa[0].id.len(), 18, "n_ + 16 hex");
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn falls_back_to_title_when_url_missing() {
        let p = write_fixture(
            "nourl",
            r#"[{"title":"没有链接的条目","published_at":"2026-11-20"}]"#,
        );
        let src = NewsSource::open(&p).unwrap();
        let (total, _, page) = src.catalog_page(1).unwrap();
        assert_eq!(total, 1);
        assert!(page[0].id.starts_with("n_"));
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn rejects_item_without_url_and_title() {
        let p = write_fixture("noid", r#"[{"source":"某站"}]"#);
        let err = NewsSource::open(&p)
            .err()
            .expect("无 url 无 title 必须报错");
        assert!(err.to_string().contains("无法派生稳定 id"), "{err}");
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn rejects_duplicate_urls() {
        let p = write_fixture(
            "dup",
            r#"[{"title":"a","url":"https://same"},{"title":"b","url":"https://same"}]"#,
        );
        let err = NewsSource::open(&p).err().expect("同 url 两条必须报错");
        assert!(err.to_string().contains("新闻条目重复"), "{err}");
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn actual_header_drops_missing_fields() {
        let p = write_fixture("hdr", SAMPLE);
        let src = NewsSource::open(&p).unwrap();
        let (_, _, page) = src.catalog_page(1).unwrap();
        let full = src.rows(&page[0].id, 10).unwrap();
        assert_eq!(full[0].len(), 7, "字段齐全的条目表头 7 列");
        let thin = src.rows(&page[1].id, 10).unwrap();
        assert_eq!(thin[0], vec!["title", "source", "url", "published_at"]);
        assert!(
            !thin[0].contains(&"summary".to_string()),
            "缺的字段不进表头"
        );
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn declared_columns_exclude_internal_score_fields() {
        let p = write_fixture("decl", SAMPLE);
        let src = NewsSource::open(&p).unwrap();
        let (_, _, page) = src.catalog_page(1).unwrap();
        let m = src.meta(&page[0].id).unwrap();
        assert_eq!(m.columns.len(), 7, "声明列只有日报承诺的 7 个");
        for internal in ["score", "comments", "ai_score"] {
            assert!(
                !m.columns.contains(&internal.to_string()),
                "内部排序字段 {internal} 不能算进声明列"
            );
        }
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn rfc3339_published_at_still_yields_lag_days() {
        // 上游用 to_rfc3339()，带 T 和时区。parse_update_time 按固定位置读，
        // 这条断言把「RFC3339 能解析」钉住——它一旦坏掉，超期检查会静默失效。
        let p = write_fixture(
            "rfc",
            r#"[{"title":"老条目","url":"https://old","published_at":"2025-01-01T08:00:00+00:00"}]"#,
        );
        let src = NewsSource::open(&p).unwrap();
        let (_, _, page) = src.catalog_page(1).unwrap();
        let lag = crate::aggregate::lag_days(page[0].update_time.as_deref().unwrap());
        assert!(lag.is_some(), "RFC3339 时间必须能算出 lagDays");
        assert!(lag.unwrap() > 90, "2025-01-01 相对基准日必须判为超期");
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn raw_snapshot_roundtrips_to_same_table() {
        let p = write_fixture("raw", SAMPLE);
        let src = NewsSource::open(&p).unwrap();
        let (_, _, page) = src.catalog_page(1).unwrap();
        let fresh = src.rows(&page[1].id, 10).unwrap();
        let from_raw = src
            .rows_from_raw(&src.preview_raw(&page[1].id, 10).unwrap())
            .unwrap();
        assert_eq!(fresh, from_raw, "证明针对的快照必须与实时表格一致");
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn blank_source_becomes_none_not_empty_string() {
        let p = write_fixture("blank", r#"[{"title":"x","url":"https://x"}]"#);
        let src = NewsSource::open(&p).unwrap();
        let (_, _, page) = src.catalog_page(1).unwrap();
        assert_eq!(page[0].private_dept, None, "空来源不能污染部门维度");
        std::fs::remove_file(&p).unwrap();
    }
}
