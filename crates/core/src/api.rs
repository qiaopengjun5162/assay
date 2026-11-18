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

/// 平台适配层：换一个开放数据平台，只实现这个 trait，其余流程不用动。
///
/// 约定：`detail_raw` / `preview_raw` 返回**原始响应字节**，
/// 证据包要对它算哈希，不能经过二次序列化。
pub trait CatalogSource: Send + Sync {
    /// 展示用平台名（写进产物与页面）
    fn platform(&self) -> &'static str;
    fn catalog_page(&self, current: usize) -> Result<(usize, usize, Vec<CatalogRecord>)>;
    fn catalog_all(&self, threads: usize) -> Result<Vec<CatalogRecord>>;
    fn detail(&self, id: &str) -> Result<Value>;
    fn preview(&self, id: &str, size: usize) -> Result<Value>;
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
        Ok((total, pages, recs))
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
    fn detail(&self, id: &str) -> Result<Value> {
        Client::detail(self, id)
    }
    fn preview(&self, id: &str, size: usize) -> Result<Value> {
        Client::preview(self, id, size)
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

/// 从样例响应里取出行（第一行是表头）。
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
