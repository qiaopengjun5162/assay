#!/usr/bin/env bash
# 安装仓库内 git 钩子（不覆盖已有的非本项目钩子）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

git config core.hooksPath .githooks
chmod +x .githooks/*

echo "已启用 .githooks（core.hooksPath）。"
echo "取消：git config --unset core.hooksPath"
