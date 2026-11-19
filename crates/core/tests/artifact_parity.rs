//! 入库产物与当前代码的**同形检查**。
//!
//! 为什么需要这一层：`web/data.json`、`web/reports/*.md` 是随仓库发布的快照，
//! 由 `assay export` 生成后提交。它们不在编译路径上，所以**代码改了字段名，
//! 产物不会跟着变，也不会有人发现**——直到某个读者按代码的字段名去取值，
//! 拿到 `null` 却以为是「这个部门没有样例」。
//!
//! 真实发生过：`dept_object` 曾把部门平均空值率输出成 `empty_avg`，后来改成
//! `emptyAvg` 并给 `DeptStat.empty_avg` 加了 `skip_serializing`，但入库的
//! `web/data.json` 没重跑。前端加了一行 `alias = "empty_avg"` 兜住，于是
//! 「发布的契约」与「代码的行为」长期不是一回事，而没有任何检查会红。
//!
//! 这里查三处：`summary.byDept`（每个部门）、`checks`（数组 + 顺序 + 分母）、
//! `history`（每个点）。产物漂移时它会红，逼你重跑 `just export`，
//! 而不是再加一层 alias 把差异藏起来。
//!
//! 没 checkout 数据（只跑窄范围单测）时整组跳过，不制造假失败。

use assay_core::model::DeptStat;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/core
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("仓库根目录")
}

/// 读入库的 `web/data.json`。不存在就返回 `None`（调用方跳过）。
fn read_shipped() -> Option<Value> {
    let path = repo_root().join("web/data.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        eprintln!("跳过：{} 不存在", path.display());
        return None;
    };
    Some(serde_json::from_str(&raw).expect("web/data.json 必须是合法 JSON"))
}

/// 取对象的一层键名。
fn keys_of(v: &Value) -> BTreeSet<String> {
    v.as_object().expect("必须是对象").keys().cloned().collect()
}

#[test]
fn shipped_data_json_by_dept_matches_current_key_style() {
    let Some(data) = read_shipped() else { return };
    let by_dept = data
        .get("summary")
        .and_then(|s| s.get("byDept"))
        .and_then(|d| d.as_object())
        .expect("summary.byDept 必须是对象");

    let sample = DeptStat::default();
    let want = keys_of(
        &serde_json::to_value(assay_core::aggregate::dept_object(&sample).expect("可序列化"))
            .expect("可转 JSON"),
    );

    let mut bad: Vec<(String, Vec<String>)> = Vec::new();
    for (dept, obj) in by_dept {
        let got = keys_of(obj);
        if got != want {
            let diff: Vec<String> = want.symmetric_difference(&got).cloned().collect();
            bad.push((dept.clone(), diff));
        }
    }

    assert!(
        bad.is_empty(),
        "入库的 web/data.json 有 {} / {} 个部门的字段名与当前代码不一致，\
         说明产物没重跑（`just report && just export 500 web/data.json`）。\n\
         期望键：{want:?}\n前 3 个不一致：{:#?}",
        bad.len(),
        by_dept.len(),
        &bad[..bad.len().min(3)]
    );
}

/// `checks` 的形状由 `checks::CheckRow` 决定，**顺序**由 `checks::ALL` 决定。
///
/// 这两件事原先都埋在 `export.rs` 的一个 `json!` 字面量里：`Check` 加字段时
/// 导出不会跟着变，产物悄悄少一个键，没有编译错误，读者拿到 `undefined`。
/// 顺序也是契约——页面按顺序印，讲稿会说「第几条」。
#[test]
fn shipped_data_json_checks_match_registry() {
    let Some(data) = read_shipped() else { return };
    let shipped = data
        .get("checks")
        .and_then(|c| c.as_array())
        .expect("checks 必须是数组");

    let want = keys_of(
        &serde_json::to_value(assay_core::checks::CheckRow::new(
            &assay_core::checks::ALL[0],
            0,
            0,
        ))
        .expect("CheckRow 可序列化"),
    );
    for (i, row) in shipped.iter().enumerate() {
        assert_eq!(
            keys_of(row),
            want,
            "checks[{i}] 的键与 CheckRow 不一致：产物没重跑，或有人又在 export 里手写 json!"
        );
    }

    let ids: Vec<&str> = shipped
        .iter()
        .map(|r| r.get("id").and_then(Value::as_str).unwrap_or("<缺 id>"))
        .collect();
    let want_ids: Vec<&str> = assay_core::checks::ALL.iter().map(|c| c.id).collect();
    assert_eq!(
        ids, want_ids,
        "checks 的 id 顺序必须与 checks::ALL 一致（页面按顺序印，讲稿会点「第几条」）"
    );

    // `of` 是本批体检总数，必须每行相同——不同说明有人按分片算了分母。
    let ofs: BTreeSet<u64> = shipped
        .iter()
        .filter_map(|r| r.get("of").and_then(Value::as_u64))
        .collect();
    assert_eq!(
        ofs.len(),
        1,
        "checks[].of 出现了 {} 种值 {ofs:?}：分母必须只有本批总数一个",
        ofs.len()
    );
}

/// `history` 每项的键由 `history::Point` 的 serde 派生决定。
///
/// 日志是**手工追加**的历史，键名漂移不会报错，只会让曲线画不出来
/// （前端反序列化失败 → 一个点都不显示 → 看着像「还没攒够数据」）。
#[test]
fn shipped_data_json_history_matches_point_shape() {
    let Some(data) = read_shipped() else { return };
    let Some(log) = data.get("history").and_then(|h| h.as_array()) else {
        panic!("data.json 必须有 history 数组（空数组也行，但不能缺）")
    };
    if log.is_empty() {
        eprintln!("跳过：history 为空，还没有任何历史点");
        return;
    }
    let sample = assay_core::history::Point {
        date: "1970-01-01".into(),
        audited: 0,
        with_preview: 0,
        lag_over90: 0,
        mismatch: 0,
        all_empty: 0,
        stopped: 0,
        empty_over30: 0,
    };
    let want = keys_of(&serde_json::to_value(&sample).expect("Point 可序列化"));
    for (i, p) in log.iter().enumerate() {
        assert_eq!(
            keys_of(p),
            want,
            "history[{i}] 的键与 history::Point 不一致：日志与代码的字段对不上了"
        );
    }
}
