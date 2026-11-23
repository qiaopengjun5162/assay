#!/usr/bin/env bash
# 复用演示：证明 assay 的内核能查「非浙江」的数据——同一套体检、同一套检查项。
#
# 这次用公众号日报的新闻条目形状（fixtures/daily-news.json）当数据源，
# 走 audit → report → checks 全链路，产物写到临时目录，不碰 web/ 等冻结产物。
# 赛前用途：路演现场一句话的实证——「换数据源只改一处，内核零改动」。
# 赛后用途：把 fixtures/daily-news.json 换成真实日报拉取，其余命令原样复用。
#
# 用法：bash scripts/reuse-demo.sh
# 退出码：0 = 内核确实对这份非浙江数据跑出了检查结论；非 0 = 没跑通。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --release -q -p assay
BIN="$ROOT/target/release/assay"
FIXTURE="$ROOT/fixtures/daily-news.json"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

echo "复用演示：数据源 = fixtures/daily-news.json（公众号日报条目形状）"
echo "临时产物目录：$OUT"
echo "── ① 体检（同一套 audit 内核，--source 指向本地 fixture）────"
"$BIN" --out "$OUT" --source "$FIXTURE" audit --threads 2

echo "── ② 聚合（同一套 report 内核）───────────────────────────"
"$BIN" --out "$OUT" report

echo "── ③ 检查项命中（同一套 checks 口径）──────────────────────"
"$BIN" checks --items "$OUT/items.jsonl" --catalog "$OUT/catalog_snapshot.json"

echo "── ④ 抽看字段对不上的条目（超期数看②的 lagOver90Days）────"
# 注意：items.jsonl 里没有 lagDays 字段——它在 export 阶段才派生，
# 超期条数要用 report 输出的 lagOver90Days，别在这里现算（口径不一致会露怯）。
python3 - "$OUT/items.jsonl" <<'PY'
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8") if l.strip()]
bad = [r for r in rows if r.get("missingFields")]
if not bad:
    print("  没有字段对不上的条目（演示数据没造出问题，检查失败）")
    raise SystemExit(1)
for r in bad[:3]:
    print(f"  {r['name']}: 声明有但实际缺 {r['missingFields']}")
PY

echo "──────────────────────────────────────────────────────────"
echo "  同一套内核，换一份非浙江数据，照样查出问题——可复用成立。"
