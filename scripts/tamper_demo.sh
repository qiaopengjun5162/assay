#!/usr/bin/env bash
# 篡改演示：改掉一条定理里的空值计数，让 Lean 内核当场拒绝。
# 用法: ./scripts/tamper_demo.sh [证明文件，默认 out-full/TopProofs.lean]
# 退出码: 0 = 内核确实拒绝了（符合预期）；1 = 篡改后仍然通过（说明证明是假的）
set -euo pipefail

src="${1:-out-full/TopProofs.lean}"
[ -f "$src" ] || { echo "找不到证明文件: $src" >&2; exit 2; }

tmp="$(mktemp -d "${TMPDIR:-$HOME/.hermes/profiles/ai/cache/scratch}/dataproof-tamper.XXXXXX")"
cp "$src" "$tmp/Tampered.lean"

python3 - "$tmp/Tampered.lean" <<'PY'
import re, sys

path = sys.argv[1]
text = open(path, encoding="utf-8").read()
pat = re.compile(r"^(theorem empty_(\d+)_(\d+) : countEmpty rows_\2 \3 = )(\d+)( := by decide)$", re.M)
m = pat.search(text)
if not m:
    sys.exit("没找到可篡改的定理")

old = int(m.group(4))
new = old + 1
text = text[: m.start(4)] + str(new) + text[m.end(4):]
open(path, "w", encoding="utf-8").write(text)
print(f"篡改: {m.group(2)} 号数据集第 {m.group(3)} 列，空值数 {old} → {new}")
PY

echo "运行: lean $(basename "$tmp")/Tampered.lean"
set +e
out="$(lean "$tmp/Tampered.lean" 2>&1)"
code=$?
set -e
echo "$out" | head -14

if [ "$code" -ne 0 ]; then
  echo
  echo "内核拒绝（预期）—— 退出码 $code"
  rm -rf "$tmp"
  exit 0
fi
echo
echo "警告：篡改后仍然通过，这份证明没有约束力" >&2
rm -rf "$tmp"
exit 1
