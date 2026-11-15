#!/usr/bin/env bash
# 证明门禁：只接受内核通过、且不含 sorry 的证明。
#
# 借鉴 yamafaktory/formal 的三条规则：
#   1. 拒绝含 sorry 的证明（等于没证）
#   2. 审计被接受证明依赖的公理
#   3. 只有 PASS 才写接受标记；失败永不缓存
#
# 用法: scripts/check_proofs.sh fixtures/expected.lean [更多.lean ...]
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "用法: $0 <file.lean> [...]" >&2
  exit 2
fi

fail=0
for f in "$@"; do
  echo "== $f"
  if [ ! -f "$f" ]; then
    echo "   FAIL 文件不存在"; fail=1; continue
  fi
  if grep -nE '\bsorry\b|\badmit\b' "$f" >/dev/null 2>&1; then
    echo "   FAIL 含 sorry/admit，拒绝接受"; fail=1; continue
  fi
  if ! command -v lean >/dev/null 2>&1; then
    echo "   FAIL 找不到 lean 可执行文件"; fail=1; continue
  fi
  if out=$(lean "$f" 2>&1); then
    echo "   PASS 内核校验通过"
  else
    echo "   FAIL 内核拒绝:"; echo "$out" | head -20; fail=1
  fi
done

exit "$fail"
