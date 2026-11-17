//! 离线端到端测试：用一个内存里的假平台跑完 audit → aggregate。
//!
//! 意义：`CatalogSource` 这一层抽象真的能换掉平台；CI 不需要访问政府网站，
//! 也能验证「取数 → 分类 → 聚合」整条链路的数字。

use dataproof_core::api::{preview_rows, CatalogSource};
use dataproof_core::audit::{audit, AuditOptions};
use dataproof_core::{aggregate, model::CatalogRecord};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

struct FixtureSource {
    catalog: Vec<CatalogRecord>,
    detail: HashMap<String, Value>,
    preview: HashMap<String, Value>,
}

impl CatalogSource for FixtureSource {
    fn platform(&self) -> &'static str {
        "测试平台"
    }
    fn catalog_page(
        &self,
        current: usize,
    ) -> dataproof_core::Result<(usize, usize, Vec<CatalogRecord>)> {
        let page = if current == 1 {
            self.catalog.clone()
        } else {
            Vec::new()
        };
        Ok((self.catalog.len(), 1, page))
    }
    fn catalog_all(&self, _threads: usize) -> dataproof_core::Result<Vec<CatalogRecord>> {
        Ok(self.catalog.clone())
    }
    fn detail(&self, id: &str) -> dataproof_core::Result<Value> {
        Ok(self.detail[id].clone())
    }
    fn preview(&self, id: &str, _size: usize) -> dataproof_core::Result<Value> {
        Ok(self.preview[id].clone())
    }
    fn detail_raw(&self, id: &str) -> dataproof_core::Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.detail[id])?)
    }
    fn preview_raw(&self, id: &str, _size: usize) -> dataproof_core::Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.preview[id])?)
    }
}

fn rec(id: &str, name: &str) -> CatalogRecord {
    CatalogRecord {
        id: id.into(),
        source_name: name.into(),
        private_dept: Some("测试局/科室".into()),
        update_cycle: Some("30".into()),
        update_time: Some("2026-01-01 00:00:00".into()),
        ..Default::default()
    }
}

fn detail(cols: &[&str], desc: &str) -> Value {
    json!({"result": {
        "columnList": cols.iter().map(|c| json!({"colNameCn": c})).collect::<Vec<_>>(),
        "hasPreviewData": 1,
        "dopSourceInfo": {"description": desc}
    }})
}

fn preview(header: &[&str], rows: Vec<Vec<&str>>) -> Value {
    let mut data: Vec<Value> = vec![json!(header)];
    for r in rows {
        data.push(json!(r));
    }
    json!({"result": {"data": data}})
}

fn fixture() -> FixtureSource {
    // 1) 完全一致；第 3 列两行都空 → 空值率 2/6，整列全空 1
    // 2) 声明 4 列、样例 3 列 → 列数不一致，缺「备注」
    // 3) 声明名带部门前缀 → 仅命名不一致（不算结构问题）
    let mut detail_map = HashMap::new();
    let mut preview_map = HashMap::new();
    detail_map.insert(
        "a".to_string(),
        detail(&["编号", "名称", "更新时间"], "示例说明文字足够长"),
    );
    preview_map.insert(
        "a".to_string(),
        preview(
            &["编号", "名称", "更新时间"],
            vec![vec!["1", "甲", ""], vec!["2", "乙", ""]],
        ),
    );
    detail_map.insert(
        "b".to_string(),
        detail(&["编号", "名称", "更新时间", "备注"], "说明"),
    );
    preview_map.insert(
        "b".to_string(),
        preview(
            &["编号", "名称", "更新时间"],
            vec![vec!["1", "丙", "2026-01-01"]],
        ),
    );
    detail_map.insert(
        "c".to_string(),
        detail(&["鹿西乡民宿房间数"], "说明文字也不短"),
    );
    preview_map.insert("c".to_string(), preview(&["房间数"], vec![vec!["12"]]));

    FixtureSource {
        catalog: vec![
            rec("a", "一致的数据集"),
            rec("b", "少一列的数据集"),
            rec("c", "带前缀的数据集"),
        ],
        detail: detail_map,
        preview: preview_map,
    }
}

#[test]
fn audit_and_report_run_offline() {
    let dir: PathBuf = std::env::temp_dir().join(format!("dataproof-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.to_string_lossy().to_string();

    let src = fixture();
    let outcome = audit(
        &AuditOptions {
            threads: 2,
            limit: 0,
            out_dir: out.clone(),
        },
        &src,
    )
    .unwrap();
    assert_eq!(outcome.audited, 3);
    assert_eq!(outcome.failed, 0);

    let items = dataproof_core::audit::read_items(&format!("{out}/items.jsonl")).unwrap();
    assert_eq!(items.len(), 3);
    let a = items.iter().find(|d| d.id == "a").unwrap();
    assert_eq!(a.declared_cols, 3);
    assert_eq!(a.sample_cols, 3);
    assert_eq!(a.sample_rows, 2);
    assert_eq!(a.empty_rate, 0.3333);
    assert_eq!(a.all_empty_cols, 1);
    assert_eq!(a.desc_len, "示例说明文字足够长".chars().count());

    let b = items.iter().find(|d| d.id == "b").unwrap();
    assert_eq!(b.declared_cols, 4);
    assert_eq!(b.sample_cols, 3);
    assert_eq!(b.missing_fields, vec!["备注".to_string()]);

    let c = items.iter().find(|d| d.id == "c").unwrap();
    assert_eq!(
        dataproof_core::scoring::classify(c),
        dataproof_core::scoring::SchemaKind::NamingOnly
    );

    let summary = aggregate::aggregate(
        &format!("{out}/items.jsonl"),
        &format!("{out}/catalog_snapshot.json"),
        &out,
    )
    .unwrap();
    assert_eq!(summary["audited"], 3);
    assert_eq!(summary["withPreview"], 3);
    // 注意：schemaMismatch 统计的是「声明与样例对不上」的三类之和，
    // 包含仅命名不一致（历史口径，和冻结产物对齐）。结构性问题看 mismatchKinds。
    assert_eq!(summary["schemaMismatch"], 2);
    assert_eq!(summary["mismatchKinds"]["列数不一致"], 1);
    assert_eq!(summary["mismatchKinds"]["仅命名不一致"], 1);

    // 报告：与证明同一选择口径，只覆盖有样例的数据集；文件名走 safe_id
    let dest = format!("{out}/reports");
    let explanations = HashMap::new();
    let rendered =
        dataproof_core::single::render_top(&items, 10, &out, &dest, &explanations).unwrap();
    assert_eq!(rendered.written, 3);
    let top = items
        .iter()
        .max_by(|a, b| {
            dataproof_core::scoring::score(a)
                .partial_cmp(&dataproof_core::scoring::score(b))
                .unwrap()
        })
        .unwrap();
    let md = std::fs::read_to_string(format!(
        "{dest}/{}.md",
        dataproof_core::util::safe_id(&top.id)
    ))
    .unwrap();
    assert!(md.contains(&top.name));
    assert!(md.contains("## 体检结论"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn preview_rows_parses_header_and_body() {
    let v = preview(&["a", "b"], vec![vec!["1", "2"]]);
    let rows = preview_rows(&v);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], vec!["a".to_string(), "b".to_string()]);
}
