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
//! 这条测试只做一件事：**入库的 `byDept` 键名必须等于 `dept_object` 现在
//! 输出的键名**。产物漂移时它会红，逼你重跑 `just export`，而不是加一层 alias
//! 把差异藏起来。

use assay_core::model::DeptStat;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/core
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("仓库根目录")
}

/// 当前代码给部门对象输出的键集合。
fn expected_keys() -> BTreeSet<String> {
    let v = DeptStat::default();
    let obj = assay_core::aggregate::dept_object(&v).expect("部门统计可序列化");
    obj.as_object()
        .expect("部门统计必须是对象")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn shipped_data_json_by_dept_matches_current_key_style() {
    let path = repo_root().join("web/data.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        // 没 checkout 数据（比如只跑 core 单测的窄环境）就跳过，不制造假失败。
        eprintln!("跳过：{} 不存在", path.display());
        return;
    };
    let data: serde_json::Value =
        serde_json::from_str(&raw).expect("web/data.json 必须是合法 JSON");
    let by_dept = data
        .get("summary")
        .and_then(|s| s.get("byDept"))
        .and_then(|d| d.as_object())
        .expect("summary.byDept 必须是对象");

    let want = expected_keys();
    let mut bad: Vec<(String, Vec<String>)> = Vec::new();
    for (dept, obj) in by_dept {
        let got: BTreeSet<String> = obj
            .as_object()
            .expect("部门值必须是对象")
            .keys()
            .cloned()
            .collect();
        if got != want {
            let missing: Vec<String> = want.difference(&got).cloned().collect();
            let extra: Vec<String> = got.difference(&want).cloned().collect();
            bad.push((dept.clone(), [missing, extra].concat()));
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
