#!/usr/bin/env bash
# 门禁负向自测：正向基线必须先过，再故意破坏产物，确认每道门禁真的会红。
#
# 常规测试证明的是「正常产物能通过」；这脚本证明的是「坏产物过不了」。
# 项目对外承诺「每条结论你都能自己验」——如果验的工具本身没有约束力，
# 那句承诺就是纸糊的。全部在临时副本上做，不碰 web/ 等入库的冻结产物。
#
# 退出码：0 = 每道门禁都对破坏有反应；非 0 = 有门禁形同虚设。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/assay"
DATA="$ROOT/web/data.json"
EVIDENCE="$ROOT/web/evidence"
REPORTS="$ROOT/web/reports"
PROOFS="$ROOT/fixtures/expected.lean"
BASELINE="$ROOT/baseline/summary.json"

if [[ ! -x "$BIN" ]]; then
  echo "缺少 $BIN，先跑：cargo build --release -p assay" >&2
  exit 2
fi
for f in "$DATA" "$EVIDENCE" "$REPORTS" "$PROOFS" "$BASELINE"; do
  if [[ ! -e "$f" ]]; then
    echo "缺少输入 $f" >&2
    exit 2
  fi
done

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

pass=0
fail=0

row() { printf '  %-24s %-20s %s\n' "$1" "$2" "$3"; }
ok() { row "$1" "$2" "PASS"; pass=$((pass + 1)); }
bad() {
  row "$1" "$2" "FAIL"
  sed -n '1,6p' "$TMP/$1.log" | sed 's/^/      /'
  fail=$((fail + 1))
}

# run_expect 名字 期望退出码 输出须含的字串(-- 表示不检查) -- 命令...
run_expect() {
  local name="$1" want="$2" needle="$3"
  shift 4
  local rc
  "$@" >"$TMP/$name.log" 2>&1
  rc=$?
  if [[ "$rc" -eq "$want" ]] && { [[ "$needle" == "--" ]] || grep -qF -- "$needle" "$TMP/$name.log"; }; then
    ok "$name" "rc=$want"
  else
    bad "$name" "rc=$rc, 期望 $want"
  fi
}

# 复制一份可随意破坏的证据 + 数据；参数为目标子目录名。
clone_evidence() {
  local d="$TMP/$1"
  mkdir -p "$d"
  cp "$DATA" "$d/data.json"
  cp -R "$EVIDENCE" "$d/evidence"
}

echo "门禁负向自测（临时目录 $TMP）"
echo "── 正向基线 ──────────────────────────────"

run_expect evidence-baseline 0 "500 / 500 份一致" -- \
  "$BIN" verify-evidence "$DATA" "$EVIDENCE"
run_expect reports-baseline 0 "500 / 500 份一致" -- \
  "$BIN" verify-reports "$DATA" "$REPORTS" "$EVIDENCE"
run_expect proofs-baseline 0 "PASS 内核校验通过" -- \
  "$BIN" check-proofs "$PROOFS"

echo "── 故意破坏：必须变红 ────────────────────"

# ① 证据文件改一个字节
clone_evidence evidence-byte
VICTIM="$(find "$TMP/evidence-byte/evidence" -type f | head -1)"
printf 'X' | dd of="$VICTIM" bs=1 seek=0 conv=notrunc status=none
run_expect evidence-byte 1 "FAIL" -- \
  "$BIN" verify-evidence "$TMP/evidence-byte/data.json" "$TMP/evidence-byte/evidence"

# ② 整份证据消失
clone_evidence evidence-missing
MISSING="$(find "$TMP/evidence-missing/evidence" -type f | head -1)"
rm -rf "$(dirname "$MISSING")"
run_expect evidence-missing 1 "FAIL" -- \
  "$BIN" verify-evidence "$TMP/evidence-missing/data.json" "$TMP/evidence-missing/evidence"

# ③ 报告里印的 SHA-256 被改（证据本身没动）
clone_evidence report-sha
cp -R "$REPORTS" "$TMP/report-sha/reports"
REPORT="$(find "$TMP/report-sha/reports" -type f | head -1)"
python3 - "$REPORT" <<'PY'
import re
import sys

path = sys.argv[1]
text = open(path, encoding="utf-8").read()
changed = re.sub(r"(SHA-256：`)[0-9a-f]{64}", lambda m: m.group(1) + "0" * 64, text, count=1)
if changed == text:
    raise SystemExit("报告里没找到 SHA-256，样本不成立")
open(path, "w", encoding="utf-8").write(changed)
PY
run_expect report-sha 1 "FAIL" -- \
  "$BIN" verify-reports "$TMP/report-sha/data.json" "$TMP/report-sha/reports" "$TMP/report-sha/evidence"

# ④ 证明里塞 sorry
cp "$PROOFS" "$TMP/proofs-sorry.lean"
printf '\nsorry\n' >> "$TMP/proofs-sorry.lean"
run_expect proofs-sorry 1 "sorry" -- \
  "$BIN" check-proofs "$TMP/proofs-sorry.lean"

echo "── 退出码语义 ────────────────────────────"

# tamper 是演示：Lean 内核拒绝（码 1），但命令本身成功（码 0）。
# 这条断言同时钉住口径，避免讲稿把它写成「命令退出码 1」。
# 必须显式给 --file：默认路径是 $out/TopProofs.lean，而 out-full/ 不入库，
# CI checkout 后根本不存在——本地绿、CI 红就栽在这里。
run_expect tamper-demo 0 "内核拒绝（预期）—— lean 退出码 1" -- \
  "$BIN" tamper --file "$PROOFS"

# 漂移三态：0 正常 / 1 该开 PR / 2 跑批不可信（处置相反）。
run_expect drift-healthy 0 "--" -- \
  "$BIN" drift --baseline "$BASELINE" --current "$BASELINE"
python3 - "$BASELINE" "$TMP/drift-drifted.json" "$TMP/drift-suspect.json" <<'PY'
import json
import sys

base = json.load(open(sys.argv[1], encoding="utf-8"))
drifted = dict(base)
drifted["schemaMismatch"] = base.get("schemaMismatch", 0) + 9000
drifted["lagMedianDays"] = 999
json.dump(drifted, open(sys.argv[2], "w", encoding="utf-8"))
suspect = dict(base)
suspect["audited"] = 100
json.dump(suspect, open(sys.argv[3], "w", encoding="utf-8"))
PY
run_expect drift-drifted 1 "判定：漂移" -- \
  "$BIN" drift --baseline "$BASELINE" --current "$TMP/drift-drifted.json"
run_expect drift-suspect 2 "--" -- \
  "$BIN" drift --baseline "$BASELINE" --current "$TMP/drift-suspect.json"

echo "──────────────────────────────────────────"
echo "  $pass 通过 / $fail 失败"
if [[ "$fail" -ne 0 ]]; then
  echo "有门禁对破坏没有反应，别信它。" >&2
  exit 1
fi
echo "每道门禁都真的会红。"
