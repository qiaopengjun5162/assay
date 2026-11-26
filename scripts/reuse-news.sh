#!/usr/bin/env bash
# 复用演示（新闻源）：直接读 qunmind 的 PublicNewsItem 形状，证明「上游不用改结构」。
#
# 与 reuse-demo.sh 的分工：
# - reuse-demo.sh 用 assay 自有 fixture 格式（camelCase、必填 id、带 declaredColumns 包装）
# - 本脚本用**上游原样数组**（snake_case、无 id、Option 字段），id 由 assay 派生
#
# 全链路 audit → report → checks，产物写临时目录，不碰 web/ 冻结快照。
#
# 用法：bash scripts/reuse-news.sh
# 退出码：0 = 跑通且确实查出问题；非 0 = 没跑通，或演示数据没造出问题。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --release -q -p assay
BIN="$ROOT/target/release/assay"
SRC="$ROOT/fixtures/qunmind-items.json"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

echo "复用演示（新闻源）：数据源 = fixtures/qunmind-items.json"
echo "形状 = qunmind::source::PublicNewsItem（snake_case / 无 id / Option 字段）"
echo "临时产物目录：$OUT"
echo "── ① 体检（--source news:<path>，内核零改动）────────────"
"$BIN" --out "$OUT" --source "news:$SRC" audit --threads 2

echo "── ② 聚合 ────────────────────────────────────────────"
"$BIN" --out "$OUT" report

echo "── ③ 检查项命中 ──────────────────────────────────────"
"$BIN" checks --items "$OUT/items.jsonl" --catalog "$OUT/catalog_snapshot.json"

echo "── ④ 逐条看缺了什么字段 ──────────────────────────────"
# missingFields 是 audit 阶段就有的；不用在这里另算，口径必须与检查项同源。
python3 - "$OUT/items.jsonl" <<'PY'
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8") if l.strip()]
bad = [r for r in rows if r.get("missingFields")]
if not bad:
    print("  没有字段对不上的条目（演示数据没造出问题，检查失败）")
    raise SystemExit(1)
for r in bad:
    print(f"  {r['name']}: 声明有但实际缺 {r['missingFields']}")
print(f"  共 {len(rows)} 条，{len(bad)} 条字段对不上")
PY

echo "──────────────────────────────────────────────────────"
echo "  上游一行没改，同一套内核照样查出问题——接真实源只剩一个导出命令。"
