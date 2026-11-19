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
#
# ── 为什么有两个数 ─────────────────────────────────────────────
# 以前只量 core + cli，报一个 76.88%；`crates/api` 从没被量过（它自己只有 34%）。
# 一个数字代表两种范围，正是「同一套材料三个覆盖率」那类错误的同一个来源。
# 现在一次测量、两个数、两个地板：
#
#   逻辑主体（core + cli）  实测 76.88%（2026-11-19，112 测试）  地板 74
#   全量（含 api crate）    实测 73.81%（同一轮，126 测试）      地板 72
#
# 全量低是因为 api 的 handler 要 Postgres：没有数据库就测不了 SQL 那一段，
# 而本项目不打算为了刷覆盖率在 CI 里起一个库。这不丢人，藏着才丢人。
# macOS aarch64 与 Ubuntu runner 实测一致（纯 Rust，无 cfg(target_os) 分歧）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FLOOR_CORE_CLI="${COVERAGE_FLOOR:-74}"
FLOOR_ALL="${COVERAGE_FLOOR_ALL:-72}"

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
    cargo llvm-cov nextest -p assay-core -p assay -p assay-api --all-features --no-report
  else
    cargo llvm-cov --no-report --no-fail-fast -p assay-core -p assay -p assay-api
  fi
fi

SUMMARY="$(mktemp)"
trap 'rm -f "$SUMMARY"' EXIT

echo
echo "== 逐文件行覆盖 =="
cargo llvm-cov report --summary-only | tee "$SUMMARY"

if [[ "$want_html" == "1" ]]; then
  cargo llvm-cov report --html
  echo
  echo "HTML 报告：target/llvm-cov/html/index.html"
fi

# 从同一张逐文件表里算行覆盖，输出 `pct lines missed`。
# 按文件名前缀筛，比再跑一遍 llvm-cov 快，也保证两个数出自同一次测量。
# 列序：Filename Regions MissReg RegCov Funcs MissFn FnCov Lines MissLines LineCov …
scope() {
  awk -v pat="$1" '
    $1 ~ pat && $8 ~ /^[0-9]+$/ { lines += $8; missed += $9 }
    END {
      if (lines == 0) { print "0.00 0 0"; exit }
      printf "%.2f %d %d\n", (lines - missed) * 100 / lines, lines, missed
    }
  ' "$SUMMARY"
}

gate() { # pct floor label
  awk -v p="$1" -v f="$2" -v l="$3" 'BEGIN {
    if (p + 0 < f + 0) { printf "覆盖率门禁失败：%s %.2f%% < %.0f%%\n", l, p, f; exit 1 }
  }'
}

read -r CC_PCT CC_LINES CC_MISSED <<<"$(scope '^(core|cli)/')"
read -r ALL_PCT ALL_LINES ALL_MISSED <<<"$(
  awk '$1 == "TOTAL" && $8 ~ /^[0-9]+$/ {
          printf "%.2f %d %d\n", ($8 - $9) * 100 / $8, $8, $9
        }' "$SUMMARY"
)"

echo
printf '逻辑主体（core+cli）：%.2f%%（%d 行，未覆盖 %d）地板 %s%%\n' \
  "$CC_PCT" "$CC_LINES" "$CC_MISSED" "$FLOOR_CORE_CLI"
printf '全量（含 api）：      %.2f%%（%d 行，未覆盖 %d）地板 %s%%\n' \
  "$ALL_PCT" "$ALL_LINES" "$ALL_MISSED" "$FLOOR_ALL"

gate "$CC_PCT" "$FLOOR_CORE_CLI" "逻辑主体"
gate "$ALL_PCT" "$FLOOR_ALL" "全量"
echo "覆盖率门禁通过。"
