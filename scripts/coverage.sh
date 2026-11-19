#!/usr/bin/env bash
# 行覆盖率测量 + 门禁。
#
# 用法：
#   ./scripts/coverage.sh              # 测量、报告、校验下限
#   ./scripts/coverage.sh --report     # 复用上次结果，不重跑测试
#   ./scripts/coverage.sh --html       # 额外生成 target/llvm-cov/html
#
# 这个下限存在的意义：让「能自动验证的那部分代码」不会悄悄烂掉。
# 它刻意低于实测值——覆盖涨了就往上抬，绝不要为了把红灯变绿而往下调。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# 实测 76.81% 行覆盖（2026-11-19，101 测试，core+cli）。
# macOS aarch64 与 Ubuntu runner 实测完全一致（纯 Rust，无 cfg(target_os) 分歧），
# 地板仍留 ~2.7pp 余量，吸收工具链小版本差异。
# 只升不降：new code 没测试就该让这个门禁红。历史上从 52 → 66 → 74。
FLOOR="${COVERAGE_FLOOR:-74}"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "需要 cargo-llvm-cov：cargo install cargo-llvm-cov --locked" >&2
  exit 1
fi

mode="run"
want_html=0
for arg in "$@"; do
  case "$arg" in
    --report) mode="report" ;;
    --html) want_html=1 ;;
    *) echo "未知参数：$arg" >&2; exit 2 ;;
  esac
done

if [[ "$mode" == "run" ]]; then
  # 清掉上一轮的 profraw。cargo llvm-cov 的 report 会把目录下所有 profraw
  # 合并进来，历史残留（比如改名前的旧二进制）会把路径和数字同时搞脏。
  cargo llvm-cov clean --workspace
  if command -v cargo-nextest >/dev/null 2>&1; then
    cargo llvm-cov nextest -p assay-core -p assay --all-features --no-report
  else
    cargo llvm-cov --no-report --no-fail-fast -p assay-core -p assay
  fi
fi

echo
echo "== 逐文件行覆盖 =="
cargo llvm-cov report --summary-only

if [[ "$want_html" == "1" ]]; then
  cargo llvm-cov report --html
  echo
  echo "HTML 报告：target/llvm-cov/html/index.html"
fi

echo
cargo llvm-cov report --summary-only --fail-under-lines "$FLOOR" >/dev/null
echo "覆盖率门禁通过（行覆盖 >= ${FLOOR}%）。"
