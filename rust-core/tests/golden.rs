//! 黄金文件测试：固定快照生成的 Lean 必须与仓库里的 expected.lean 完全一致。
//!
//! 这一层防的是「改规则时悄悄改了证明的语义」。证明一旦变了，
//! 报告里的数字和讲稿里的例子就不可信了。

use std::process::Command;

#[test]
fn fixture_generates_expected_lean() {
    let bin = env!("CARGO_BIN_EXE_lean_from_fixture");
    let out = Command::new(bin)
        .arg("fixtures/snapshot.json")
        .output()
        .expect("跑不动 lean_from_fixture");
    assert!(
        out.status.success(),
        "生成失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let got = String::from_utf8(out.stdout).expect("输出不是 UTF-8");
    let want =
        std::fs::read_to_string("fixtures/expected.lean").expect("缺 fixtures/expected.lean");
    assert_eq!(
        got, want,
        "生成的 Lean 与黄金文件不一致。若这是有意的规则变更，请重新生成 fixtures/expected.lean 并说明原因。"
    );
}

#[test]
fn expected_lean_has_no_sorry() {
    let text = std::fs::read_to_string("fixtures/expected.lean").expect("缺黄金文件");
    assert!(
        !text.contains("sorry"),
        "证明里出现 sorry：等于没证明。CI 必须拒绝。"
    );
}
