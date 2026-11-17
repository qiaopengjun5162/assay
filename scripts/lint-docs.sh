#!/usr/bin/env bash
# Markdown lint。默认全量；`--changed` 只查相对 origin/main 的改动。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MDLINT="markdownlint-cli2"
if ! command -v "$MDLINT" >/dev/null 2>&1; then
  MDLINT="npx markdownlint-cli2"
fi

if [[ "${1:-}" == "--changed" ]]; then
  BASE="${BASE_BRANCH:-origin/main}"
  # core.quotepath=false：中文文件名会被转义，行尾不再是 .md，grep 会漏掉
  CHANGED=$(
    {
      git -c core.quotepath=false diff --name-only --diff-filter=AM "$BASE...HEAD"
      git -c core.quotepath=false diff --name-only --diff-filter=AM HEAD
    } | grep '\.md$' | sort -u || true
  )
  if [[ -z "$CHANGED" ]]; then
    echo "相对 $BASE 没有改动的 Markdown。"
    exit 0
  fi
  echo "$CHANGED" | xargs -r $MDLINT --no-globs
else
  $MDLINT
fi
