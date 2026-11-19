//! 平台接口客户端。
//!
//! 三个端点都免登录：目录分页、字段定义、样例预览。
//! 分页参数是 `current` / `size`——用 `page` 之类的名字会被静默忽略并返回第一页，
//! 所以这里把请求体固定在该客户端里，避免调用方再猜。

use crate::model::CatalogRecord;
use crate::{Error, Result};
use serde_json::{json, Value};
use std::time::Duration;

pub const BASE: &str = "https://data.zjzwfw.gov.cn/dopServer";
const TIMEOUT: Duration = Duration::from_secs(30);

/// id 里含 `+ / =`，必须逐字符编码，否则服务端解不出来。
pub fn enc(id: &str) -> String {
    id.replace('+', "%2B")
        .replace('/', "%2F")
        .replace('=', "%3D")
}

/// 平台无关的数据集元信息。
///
/// 为什么要有这个结构：各平台返回的 JSON 形状差别很大（浙江是
/// `result.columnList[].colNameCn`，上海、山东是别的路径），如果让
/// `audit` 自己去 `detail["result"]["columnList"]` 取值，那 `CatalogSource`
/// 就只是个名字——换平台照样要改下游。凡是「平台形状」的知识都收在适配器里，
/// 跨过 trait 边界的只有这个结构。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct DatasetMeta {
    /// 声明的字段名，顺序即声明顺序。
    pub columns: Vec<String>,
    /// 平台是否提供样例数据。
    pub has_preview: bool,
    /// 字段说明的字数。只要长度，不要内容——判断的是「有没有认真写说明」。
    pub description_len: usize,
}

/// 平台适配层：换一个开放数据平台，只实现这个 trait，其余流程不用动。
///
/// 新平台需要实现的方法只有六个：目录分页 / 目录全量 / 元信息 / 样例行 /
/// 两份原始字节。**不要**在 `meta` / `rows` 之外暴露平台原始 JSON，
/// 否则下游又会开始按某个平台的结构取值。
///
/// 约定：`detail_raw` / `preview_raw` 返回**原始响应字节**，
/// 证据包要对它算哈希，不能经过二次序列化。
pub trait CatalogSource: Send + Sync {
    /// 展示用平台名（写进产物与页面）
    fn platform(&self) -> &'static str;
    fn catalog_page(&self, current: usize) -> Result<(usize, usize, Vec<CatalogRecord>)>;
    fn catalog_all(&self, threads: usize) -> Result<Vec<CatalogRecord>>;
    /// 声明的字段、是否有样例、说明字数。
    fn meta(&self, id: &str) -> Result<DatasetMeta>;
    /// 样例表格，**第一行是表头**。
    fn rows(&self, id: &str, size: usize) -> Result<Vec<Vec<String>>>;
    /// 从**已存证的原始字节**里解析出同一张表。
    ///
    /// 证明必须针对快照，而不是重新抓一遍（D20）。所以快照的解析也要归适配器管：
    /// 每个平台的 `preview.json` 形状不同，下游不该知道怎么读。
    fn rows_from_raw(&self, raw: &[u8]) -> Result<Vec<Vec<String>>>;
    fn detail_raw(&self, id: &str) -> Result<Vec<u8>>;
    fn preview_raw(&self, id: &str, size: usize) -> Result<Vec<u8>>;
}

pub struct Client {
    agent: ureq::Agent,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(TIMEOUT)
            .user_agent("Mozilla/5.0 assay/0.2")
            .build();
        Self { agent }
    }

    fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let mut last = String::from("未知错误");
        for attempt in 0..3 {
            match self
                .agent
                .post(&format!("{BASE}{path}"))
                .set("Content-Type", "application/json")
                .set("Referer", "https://data.zjzwfw.gov.cn/dopServer/")
                .send_json(body)
            {
                Ok(resp) => match resp.into_json::<Value>() {
                    Ok(v) => return Ok(v),
                    Err(e) => last = format!("解析失败: {e}"),
                },
                Err(e) => last = format!("HTTP 失败: {e}"),
            }
            std::thread::sleep(Duration::from_millis(300 * (attempt + 1) as u64));
        }
        Err(Error::Http(format!("{path}: {last}")))
    }

    /// 原始响应字节——证据包要存原样，不能存二次序列化的结果。
    pub fn post_raw(&self, path: &str, body: &Value) -> Result<Vec<u8>> {
        let mut last = String::from("未知错误");
        for attempt in 0..3 {
            match self
                .agent
                .post(&format!("{BASE}{path}"))
                .set("Content-Type", "application/json")
                .set("Referer", "https://data.zjzwfw.gov.cn/dopServer/")
                .send_json(body)
            {
                Ok(resp) => {
                    let mut buf = Vec::new();
                    use std::io::Read;
                    let mut r = resp.into_reader();
                    match r.read_to_end(&mut buf) {
                        Ok(_) => return Ok(buf),
                        Err(e) => last = format!("读取失败: {e}"),
                    }
                }
                Err(e) => last = format!("HTTP 失败: {e}"),
            }
            std::thread::sleep(Duration::from_millis(300 * (attempt + 1) as u64));
        }
        Err(Error::Http(format!("{path}: {last}")))
    }

    /// 目录一页（100 条）。`current` 从 1 开始。
    pub fn catalog_page(&self, current: usize) -> Result<(usize, usize, Vec<CatalogRecord>)> {
        let v = self.post(
            "/dataCatalog/dataQueryPage",
            &json!({
                "current": current, "size": 100, "sourceType": "A", "dataDomain": "",
                "openType": "", "deptId": null, "sortPath": null, "dataName": "",
                "order": "", "orderFiled": ""
            }),
        )?;
        Ok(parse_catalog_page(&v))
    }

    /// 全量目录，按原顺序返回。
    pub fn catalog_all(&self, threads: usize) -> Result<Vec<CatalogRecord>> {
        let (total, pages, first) = self.catalog_page(1)?;
        let mut slots: Vec<Option<Vec<CatalogRecord>>> = vec![None; pages];
        if pages > 0 {
            slots[0] = Some(first);
        }
        let next = std::sync::atomic::AtomicUsize::new(2);
        let slots = std::sync::Mutex::new(slots);
        std::thread::scope(|s| {
            for _ in 0..threads.max(1) {
                s.spawn(|| loop {
                    let p = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if p > pages {
                        break;
                    }
                    if let Ok((_, _, recs)) = self.catalog_page(p) {
                        slots.lock().unwrap()[p - 1] = Some(recs);
                    }
                });
            }
        });
        let slots = slots.into_inner().unwrap();
        let mut out = Vec::with_capacity(total);
        for recs in slots.into_iter().flatten() {
            out.extend(recs);
        }
        Ok(out)
    }

    pub fn detail(&self, id: &str) -> Result<Value> {
        self.post(
            &format!("/dataCatalog/catalogDetail/{}", enc(id)),
            &json!({}),
        )
    }

    pub fn preview(&self, id: &str, size: usize) -> Result<Value> {
        self.post(
            &format!("/dataCatalog/catalogDataPreview/{}", enc(id)),
            &json!({"current": 1, "size": size}),
        )
    }
}

/// 目录页解析：平台响应的 `result` 里取分页与记录。
///
/// 抽成纯函数不是为了复用，是为了钉住两个口径：
/// - 分页字段缺失按 0 处理，不把「平台没给」变成 panic；
/// - 单条记录形状不对就跳过，不把整页 100 条一起丢掉（平台脏数据是常态）。
fn parse_catalog_page(v: &Value) -> (usize, usize, Vec<CatalogRecord>) {
    let total = v["result"]["total"].as_u64().unwrap_or(0) as usize;
    let pages = v["result"]["pages"].as_u64().unwrap_or(0) as usize;
    let recs: Vec<CatalogRecord> = v["result"]["records"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| serde_json::from_value(r.clone()).ok())
                .collect()
        })
        .unwrap_or_default();
    (total, pages, recs)
}

impl CatalogSource for Client {
    fn platform(&self) -> &'static str {
        "浙江省公共数据开放平台"
    }
    fn catalog_page(&self, current: usize) -> Result<(usize, usize, Vec<CatalogRecord>)> {
        Client::catalog_page(self, current)
    }
    fn catalog_all(&self, threads: usize) -> Result<Vec<CatalogRecord>> {
        Client::catalog_all(self, threads)
    }
    fn meta(&self, id: &str) -> Result<DatasetMeta> {
        Ok(meta_from_zj(&self.detail(id)?))
    }
    fn rows(&self, id: &str, size: usize) -> Result<Vec<Vec<String>>> {
        Ok(preview_rows(&self.preview(id, size)?))
    }
    fn rows_from_raw(&self, raw: &[u8]) -> Result<Vec<Vec<String>>> {
        let v: Value = serde_json::from_slice(raw)
            .map_err(|e| Error::Other(format!("浙江快照不是合法 JSON: {e}")))?;
        Ok(preview_rows(&v))
    }
    fn detail_raw(&self, id: &str) -> Result<Vec<u8>> {
        self.post_raw(
            &format!("/dataCatalog/catalogDetail/{}", enc(id)),
            &json!({}),
        )
    }
    fn preview_raw(&self, id: &str, size: usize) -> Result<Vec<u8>> {
        self.post_raw(
            &format!("/dataCatalog/catalogDataPreview/{}", enc(id)),
            &json!({"current": 1, "size": size}),
        )
    }
}

/// 浙江的详情响应 → 归一化元信息。
///
/// 字段名优先 `colNameCn`（中文名），没有才退到 `colName`；
/// 两个都没有就当成空列名——和旧产物的口径一致，不在这里静默补位。
fn meta_from_zj(detail: &Value) -> DatasetMeta {
    let r = &detail["result"];
    let columns: Vec<String> = r["columnList"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|c| {
                    c["colNameCn"]
                        .as_str()
                        .or_else(|| c["colName"].as_str())
                        .unwrap_or("")
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default();
    DatasetMeta {
        columns,
        has_preview: r["hasPreviewData"].as_i64().unwrap_or(0) == 1,
        description_len: r["dopSourceInfo"]["description"]
            .as_str()
            .unwrap_or("")
            .chars()
            .count(),
    }
}

/// 浙江的样例响应 → 表格（第一行是表头）。
///
/// 这是浙江的形状，不是通用形状：新平台在自己的 `rows()` 里解析自己的响应。
pub fn preview_rows(v: &Value) -> Vec<Vec<String>> {
    v["result"]["data"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|r| {
                    r.as_array()
                        .map(|cells| {
                            cells
                                .iter()
                                .map(|c| match c {
                                    Value::String(s) => s.clone(),
                                    Value::Null => String::new(),
                                    other => other.to_string(),
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 浙江的详情里字段名有两个：`colNameCn`（中文）和 `colName`（英文）。
    /// 旧产物按中文名对齐，这里必须保持一致，否则换平台前先自己把口径改坏了。
    #[test]
    fn meta_prefers_chinese_name_then_falls_back() {
        let d = json!({"result": {
            "columnList": [
                {"colNameCn": "编号", "colName": "id"},
                {"colName": "name"},
                {}
            ],
            "hasPreviewData": 1,
            "dopSourceInfo": {"description": "说明文字"}
        }});
        let m = meta_from_zj(&d);
        assert_eq!(m.columns, vec!["编号", "name", ""]);
        assert!(m.has_preview);
        assert_eq!(m.description_len, 4);
    }

    #[test]
    fn meta_without_preview_or_description() {
        let m = meta_from_zj(&json!({"result": {"hasPreviewData": 0}}));
        assert!(m.columns.is_empty());
        assert!(!m.has_preview);
        assert_eq!(m.description_len, 0);
    }

    #[test]
    fn preview_rows_parses_header_and_body() {
        let v = json!({"result": {"data": [["a", "b"], ["1", 2], [null]]}});
        let rows = preview_rows(&v);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["a".to_string(), "b".to_string()]);
        // 数字转字符串，null 变空串——和导出的表格口径一致
        assert_eq!(rows[1], vec!["1".to_string(), "2".to_string()]);
        assert_eq!(rows[2], vec![String::new()]);
    }

    /// 快照解析必须走适配器：坏字节要报错，不能悄悄返回空表
    /// （空表会被下游当成「平台没给样例」，把抓取失败伪装成平台缺陷）。
    #[test]
    fn rows_from_raw_rejects_non_json() {
        let c = Client::new();
        assert!(c.rows_from_raw(b"\x00\x01not json").is_err());
        let rows = c
            .rows_from_raw(br#"{"result":{"data":[["h"],["v"]]}}"#)
            .unwrap();
        assert_eq!(rows.len(), 2);
    }

    /// id 是 base64，`+ / =` 三个字符在 URL 路径里有特殊含义。
    /// 漏掉一个，平台会返回 200 + 空数据而不是报错——静默丢数据。
    #[test]
    fn enc_codes_all_reserved_chars_and_leaves_others() {
        assert_eq!(enc("ab+cd/ef=gh"), "ab%2Bcd%2Fef%3Dgh");
        assert_eq!(enc("a_b-c.d"), "a_b-c.d");
        assert_eq!(enc(""), "");
    }

    #[test]
    fn parse_catalog_page_skips_malformed_records() {
        let v = json!({"result": {
            "total": 24322,
            "pages": 244,
            "records": [
                {"id": "a", "sourceName": "甲"},
                {"id": 123, "sourceName": "类型错了"},
                {"id": "c", "updateTime": "2025-06-09 08:27:49"}
            ]
        }});
        let (total, pages, recs) = parse_catalog_page(&v);
        assert_eq!((total, pages), (24322, 244));
        assert_eq!(recs.len(), 2, "坏记录跳过，不能让整页消失");
        assert_eq!(recs[0].id, "a");
        assert_eq!(recs[1].update_time.as_deref(), Some("2025-06-09 08:27:49"));
    }

    /// 平台偶尔返回字符串数字或缺 result。解析要给出 0，不 panic。
    #[test]
    fn parse_catalog_page_defaults_to_zero() {
        let (total, pages, recs) = parse_catalog_page(&json!({}));
        assert_eq!((total, pages), (0, 0));
        assert!(recs.is_empty());
        let v = json!({"result": {"total": "24322", "pages": null, "records": {}}});
        let (total, pages, recs) = parse_catalog_page(&v);
        assert_eq!((total, pages), (0, 0));
        assert!(recs.is_empty());
    }

    #[test]
    fn preview_rows_stringifies_non_string_cells() {
        let v = json!({"result": {"data": [[true, 1.5, {"a": 1}, [1, 2]]]}});
        let rows = preview_rows(&v);
        assert_eq!(rows[0], vec!["true", "1.5", "{\"a\":1}", "[1,2]"]);
        assert!(preview_rows(&json!({})).is_empty());
    }
}
