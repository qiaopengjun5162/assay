#!/usr/bin/env bash
# 校验一段提交范围内的 Conventional Commits。
#
# 为什么 CI 里还要跑一遍：本地 .githooks/commit-msg 能被 `--no-verify` 绕过，
# 也能因为新人忘了 `just hooks` 而根本没装上。钩子图方便，CI 才有约束力。
#
# 用法：./scripts/check-commits.sh [base]
#   base 省略时用 origin/main。base 不是合法 revision（例如新分支的 before=000…）
#   时回退到 origin/main，避免把「没得比」误判成失败。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

BASE="${1:-origin/main}"
if ! git rev-parse --verify --quiet "${BASE}^{commit}" >/dev/null; then
  BASE="origin/main"
fi
if ! git rev-parse --verify --quiet "${BASE}^{commit}" >/dev/null; then
  echo "无法解析基准分支 ${BASE}（浅克隆？需要 fetch-depth: 0）" >&2
  exit 2
fi

ALLOWED_TYPES="build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test"
PATTERN="^(${ALLOWED_TYPES})(\(.+\))?!?: .+"

COMMITS=$(git rev-list --no-merges "${BASE}..HEAD")
if [[ -z "$COMMITS" ]]; then
  echo "相对 ${BASE} 没有待检查的提交。"
  exit 0
fi

INVALID=0
COUNT=0
for commit in $COMMITS; do
  COUNT=$((COUNT + 1))
  MSG=$(git log -1 --format=%s "$commit")
  if [[ ! "$MSG" =~ $PATTERN ]]; then
    echo "不合规 ${commit:0:8}：$MSG" >&2
    INVALID=1
  fi
done

if [[ "$INVALID" -ne 0 ]]; then
  echo "" >&2
  echo "提交信息必须遵循 Conventional Commits。允许的类型：${ALLOWED_TYPES//|/ }" >&2
  echo "例：feat(cli): add drift subcommand" >&2
  exit 1
fi

echo "提交信息合规（${COUNT} 条）。"
