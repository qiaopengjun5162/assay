//! 通用 JSON 数据源适配器（参考实现）。
//!
//! 这一层回答一个问题：`assay` 的内核（`audit` / `evidence` / `proof`）到底能不能
//! 脱离浙江平台跑？答案是能——内核只依赖 [`CatalogSource`] 这个平台无关的接口，
//! 本模块就是第二个实现：从一个本地 JSON 文件读条目，不触网、不碰冻结产物。
//!
//! 设计意图不是「再做一个玩具」，而是把「换数据源」这件事缩到最小：
//! - 赛前：证明同一套内核能查非浙江的数据，`just reuse-demo` 可现场复现；
//! - 赛后：把 `open()` 里读文件换成拉真实日报/新闻源，其余代码一行不用改。
//!
//! ## 字段映射（以公众号日报条目为例）
//!
//! 一个新闻条目被当成一个「数据集」来体检：
//! - `declaredColumns` = 日报模板承诺的字段（标题/来源/链接/摘要/作者/时间/分类）
//! - 条目里**实际非空**的字段 = 样例表头
//! - 两者差集 → `missingFields` / `extraFields`（字段对不上）
//! - `publishedAt` → 目录记录的 `updateTime`，直接复用超期/停更检查
//! - `summary` 字数 → `descriptionLen`（说明是否认真写）
//!
//! 于是「日报声称每条都有摘要和作者，某源实际只给了标题和链接」这种货不对板，
//! 用的是和 24,322 个政府数据集完全同一套检查，不是另写一套。

use crate::api::{CatalogSource, DatasetMeta};
use crate::model::CatalogRecord;
use crate::{Error, Result};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// fixture 文件格式。字段用 camelCase，和仓库现有产物口径一致。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureFile {
    platform: String,
    /// 模板声明的字段全集（顺序保留，用于 missingFields 的对齐）
    declared_columns: Vec<String>,
    /// 声明的更新周期（可选），如「按日」「每日」
    #[serde(default)]
    cycle: Option<String>,
    items: Vec<Value>,
}

/// 从本地 JSON 文件读的只读数据源。
///
/// 全部字段在 `open()` 时一次性读入内存，之后只读；`Value` / `BTreeMap` 本身
/// 是 `Send + Sync`，所以不需要加锁就能满足 trait 的约束。
pub struct FixtureSource {
    // `platform()` 的返回类型是 &'static str（trait 定死），而平台名来自运行时的
    // fixture 文件。只在 open() 里泄漏一次（进程生命周期内一份，不是每次调用），
    // 把所有权转成 'static；这是不改 trait 的前提下最小的做法。
    platform: &'static str,
    declared: Vec<String>,
    by_id: BTreeMap<String, Value>,
    catalog: Vec<CatalogRecord>,
}

impl FixtureSource {
    /// 从一个 JSON fixture 构造数据源。
    ///
    /// 文件格式见模块文档；`items[].id` 必须唯一且非空，重复会让证据覆盖。
    pub fn open(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .map_err(|e| Error::Other(format!("读取 fixture {}: {e}", path.display())))?;
        let f: FixtureFile = serde_json::from_str(&text)
            .map_err(|e| Error::Other(format!("fixture 不是合法 JSON: {e}")))?;
        if f.declared_columns.is_empty() {
            return Err(Error::Other("fixture 的 declaredColumns 不能为空".into()));
        }
        let platform: &'static str = Box::leak(f.platform.into_boxed_str());

        let mut by_id = BTreeMap::new();
        let mut catalog = Vec::with_capacity(f.items.len());
        for item in &f.items {
            let id = item["id"].as_str().unwrap_or("").to_string();
            if id.is_empty() {
                return Err(Error::Other("fixture 有条目缺 id".into()));
            }
            if by_id.insert(id.clone(), item.clone()).is_some() {
                return Err(Error::Other(format!("fixture 的 id 重复: {id}")));
            }
            // 新闻条目的字段映射成目录记录：这几个字段正好覆盖聚合阶段要用的量。
            catalog.push(CatalogRecord {
                id,
                source_name: item["title"].as_str().unwrap_or("").to_string(),
                private_dept: item["source"].as_str().map(str::to_string),
                open_type: None,
                // 声明周期进 updateCycle → 停更检查（STOPPED 集合）能复用
                update_cycle: f.cycle.clone(),
                // publishedAt 进 updateTime → 超期检查（lagDays）能复用
                update_time: item["publishedAt"].as_str().map(str::to_string),
                data_size: None,
                area_type: None,
                region_id: None,
            });
        }

        Ok(Self {
            platform,
            declared: f.declared_columns,
            by_id,
            catalog,
        })
    }

    /// 一条条目 → 样例表格：第一行是「实际非空」的字段，第二行是值。
    ///
    /// 关键点：表头**不是**声明字段全集，而是条目真正给了的字段。
    /// 这样 audit 算 `missingFields` 时才能发现「模板说有、实际没给」。
    fn table(&self, item: &Value) -> Vec<Vec<String>> {
        let mut header = Vec::new();
        let mut values = Vec::new();
        for col in &self.declared {
            let v = &item[col];
            let non_empty = match v {
                Value::Null => false,
                Value::String(s) => !s.trim().is_empty(),
                _ => true,
            };
            if non_empty {
                header.push(col.clone());
                values.push(match v {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    other => other.to_string(),
                });
            }
        }
        vec![header, values]
    }

    fn item(&self, id: &str) -> Result<&Value> {
        self.by_id
            .get(id)
            .ok_or_else(|| Error::Other(format!("fixture 里没有 id: {id}")))
    }
}

impl CatalogSource for FixtureSource {
    fn platform(&self) -> &'static str {
        self.platform
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
        let summary = item["summary"].as_str().unwrap_or("");
        Ok(DatasetMeta {
            columns: self.declared.clone(),
            has_preview: true,
            description_len: summary.chars().count(),
        })
    }

    fn rows(&self, id: &str, _size: usize) -> Result<Vec<Vec<String>>> {
        Ok(self.table(self.item(id)?))
    }

    /// 从已存证的原始字节解析表格——证明必须针对快照，不能重新读 fixture。
    /// `preview_raw` 存的就是 `{"columns":[...],"rows":[[...]]}`，和这里对称。
    fn rows_from_raw(&self, raw: &[u8]) -> Result<Vec<Vec<String>>> {
        let v: Value = serde_json::from_slice(raw)
            .map_err(|e| Error::Other(format!("fixture 快照不是合法 JSON: {e}")))?;
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
        // rows 必须是「行的数组」：table[1] 是单行（Vec<String>），要再包一层，
        // 否则 rows_from_raw 会把每个单元格当成一行，快照就散架了（单测抓到过）。
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
            "assay-fixture-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        p
    }

    const SAMPLE: &str = r#"{
        "platform": "公众号日报·AI 新闻条目",
        "declaredColumns": ["title", "source", "url", "summary", "author", "publishedAt", "category"],
        "cycle": "按日",
        "items": [
            {"id": "n1", "title": "完整条目", "source": "36氪", "url": "https://a", "summary": "一段摘要", "author": "张三", "publishedAt": "2026-11-13", "category": "AI"},
            {"id": "n2", "title": "缺字段条目", "source": "某站", "url": "https://b", "publishedAt": "2025-03-01"}
        ]
    }"#;

    #[test]
    fn maps_news_item_to_catalog_record() {
        let src = FixtureSource::open(&write_fixture("map", SAMPLE)).unwrap();
        assert_eq!(src.platform(), "公众号日报·AI 新闻条目");
        let (total, pages, page) = src.catalog_page(1).unwrap();
        assert_eq!((total, pages), (2, 1));
        assert_eq!(page[0].source_name, "完整条目");
        assert_eq!(page[0].private_dept.as_deref(), Some("36氪"));
        assert_eq!(page[0].update_time.as_deref(), Some("2026-11-13"));
        assert_eq!(page[0].update_cycle.as_deref(), Some("按日"));
    }

    #[test]
    fn actual_header_drops_missing_fields() {
        let src = FixtureSource::open(&write_fixture("hdr", SAMPLE)).unwrap();
        let full = src.rows("n1", 10).unwrap();
        assert_eq!(full[0].len(), 7, "字段齐全的条目表头 7 列");
        let thin = src.rows("n2", 10).unwrap();
        assert_eq!(thin[0], vec!["title", "source", "url", "publishedAt"]);
        assert!(
            !thin[0].contains(&"summary".to_string()),
            "缺的字段不进表头"
        );
    }

    #[test]
    fn meta_uses_declared_columns_and_summary_len() {
        let src = FixtureSource::open(&write_fixture("meta", SAMPLE)).unwrap();
        let m = src.meta("n1").unwrap();
        assert_eq!(m.columns.len(), 7, "声明字段是全量模板");
        assert_eq!(m.description_len, 4, "摘要字数");
        assert!(m.has_preview);
    }

    #[test]
    fn raw_snapshot_roundtrips_to_same_table() {
        let src = FixtureSource::open(&write_fixture("raw", SAMPLE)).unwrap();
        let fresh = src.rows("n2", 10).unwrap();
        let from_raw = src
            .rows_from_raw(&src.preview_raw("n2", 10).unwrap())
            .unwrap();
        assert_eq!(fresh, from_raw, "证明针对的快照必须与实时表格一致");
    }

    #[test]
    fn rejects_empty_declared_and_duplicate_id() {
        let empty = r#"{"platform":"x","declaredColumns":[],"items":[]}"#;
        assert!(FixtureSource::open(&write_fixture("empty", empty)).is_err());
        let dup = r#"{"platform":"x","declaredColumns":["a"],"items":[
            {"id":"same"},{"id":"same"}]}"#;
        assert!(FixtureSource::open(&write_fixture("dup", dup)).is_err());
    }
}
