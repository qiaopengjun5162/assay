//! 离线端到端测试：用一个内存里的假平台跑完 audit → aggregate。
//!
//! 意义：`CatalogSource` 这一层抽象真的能换掉平台；CI 不需要访问政府网站，
//! 也能验证「取数 → 分类 → 聚合」整条链路的数字。

use assay_core::api::{CatalogSource, DatasetMeta};
use assay_core::audit::{audit, AuditOptions};
use assay_core::{aggregate, model::CatalogRecord};
use std::collections::HashMap;
use std::path::PathBuf;

/// 假平台：直接返回**归一化**数据，不模拟任何一家平台的 JSON。
///
/// 这一点本身就是被测的东西——如果下游还在按浙江的 `result.columnList` 取值，
/// 这个只有字段名和行的 fixture 就编译不过、跑不通。
struct FixtureSource {
    catalog: Vec<CatalogRecord>,
    meta: HashMap<String, DatasetMeta>,
    rows: HashMap<String, Vec<Vec<String>>>,
}

impl CatalogSource for FixtureSource {
    fn platform(&self) -> &'static str {
        "测试平台"
    }
    fn catalog_page(
        &self,
        current: usize,
    ) -> assay_core::Result<(usize, usize, Vec<CatalogRecord>)> {
        let page = if current == 1 {
            self.catalog.clone()
        } else {
            Vec::new()
        };
        Ok((self.catalog.len(), 1, page))
    }
    fn catalog_all(&self, _threads: usize) -> assay_core::Result<Vec<CatalogRecord>> {
        Ok(self.catalog.clone())
    }
    fn meta(&self, id: &str) -> assay_core::Result<DatasetMeta> {
        Ok(self.meta[id].clone())
    }
    fn rows(&self, id: &str, _size: usize) -> assay_core::Result<Vec<Vec<String>>> {
        Ok(self.rows[id].clone())
    }
    fn rows_from_raw(&self, raw: &[u8]) -> assay_core::Result<Vec<Vec<String>>> {
        // 假平台的快照格式就是它自己：JSON 数组，第一行表头。
        Ok(serde_json::from_slice(raw)?)
    }
    fn detail_raw(&self, id: &str) -> assay_core::Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.meta[id])?)
    }
    fn preview_raw(&self, id: &str, _size: usize) -> assay_core::Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.rows[id])?)
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

fn meta(cols: &[&str], desc: &str) -> DatasetMeta {
    DatasetMeta {
        columns: cols.iter().map(|c| c.to_string()).collect(),
        has_preview: true,
        description_len: desc.chars().count(),
    }
}

/// 表头 + 数据行，和 trait 约定的形状一致。
fn table(header: &[&str], rows: Vec<Vec<&str>>) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = vec![header.iter().map(|s| s.to_string()).collect()];
    for r in rows {
        out.push(r.iter().map(|s| s.to_string()).collect());
    }
    out
}

fn fixture() -> FixtureSource {
    // 1) 完全一致；第 3 列两行都空 → 空值率 2/6，整列全空 1
    // 2) 声明 4 列、样例 3 列 → 列数不一致，缺「备注」
    // 3) 声明名带部门前缀 → 仅命名不一致（不算结构问题）
    let mut meta_map = HashMap::new();
    let mut rows_map = HashMap::new();
    meta_map.insert(
        "a".to_string(),
        meta(&["编号", "名称", "更新时间"], "示例说明文字足够长"),
    );
    rows_map.insert(
        "a".to_string(),
        table(
            &["编号", "名称", "更新时间"],
            vec![vec!["1", "甲", ""], vec!["2", "乙", ""]],
        ),
    );
    meta_map.insert(
        "b".to_string(),
        meta(&["编号", "名称", "更新时间", "备注"], "说明"),
    );
    rows_map.insert(
        "b".to_string(),
        table(
            &["编号", "名称", "更新时间"],
            vec![vec!["1", "丙", "2026-01-01"]],
        ),
    );
    meta_map.insert(
        "c".to_string(),
        meta(&["鹿西乡民宿房间数"], "说明文字也不短"),
    );
    rows_map.insert("c".to_string(), table(&["房间数"], vec![vec!["12"]]));

    FixtureSource {
        catalog: vec![
            rec("a", "一致的数据集"),
            rec("b", "少一列的数据集"),
            rec("c", "带前缀的数据集"),
        ],
        meta: meta_map,
        rows: rows_map,
    }
}

#[test]
fn audit_and_report_run_offline() {
    let dir: PathBuf = std::env::temp_dir().join(format!("assay-e2e-{}", std::process::id()));
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

    let items = assay_core::audit::read_items(&format!("{out}/items.jsonl")).unwrap();
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
        assay_core::scoring::classify(c),
        assay_core::scoring::SchemaKind::NamingOnly
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
    let rendered = assay_core::single::render_top(&items, 10, &out, &dest, &explanations).unwrap();
    assert_eq!(rendered.written, 3);
    let top = items
        .iter()
        .max_by(|a, b| {
            assay_core::scoring::score(a)
                .partial_cmp(&assay_core::scoring::score(b))
                .unwrap()
        })
        .unwrap();
    let md = std::fs::read_to_string(format!("{dest}/{}.md", assay_core::util::safe_id(&top.id)))
        .unwrap();
    assert!(md.contains(&top.name));
    assert!(md.contains("## 体检结论"));

    let _ = std::fs::remove_dir_all(&dir);
}
