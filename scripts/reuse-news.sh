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

echo "── ④ 逐条看哪里不合格 ────────────────────────────────"
# 这里**不再**列 missingFields：qunmind 的条目没有「独立声明方」，声明列就是
# 实际给出的列，missingFields 恒为空（第一版把 Option 当承诺，59 条真实数据 100% 命中，
# 那种全中是没有信息量的）。这个源真正能查的是「说明太短」和「过期」。
python3 - "$OUT/items.jsonl" <<'PY'
import json, sys
rows = [json.loads(l) for l in open(sys.argv[1], encoding="utf-8") if l.strip()]
short = [r for r in rows if (r.get("descLen") or 0) < 10]
for r in short:
    print(f"  说明不足：{r['name'][:36]}（{r.get('descLen', 0)} 字）")
if not short:
    print("  没有说明不足的条目（演示数据没造出问题，检查失败）")
    raise SystemExit(1)
print(f"  共 {len(rows)} 条，{len(short)} 条说明不足")
PY

echo "──────────────────────────────────────────────────────"
echo "  上游一行没改，同一套内核照样查出问题。"
echo "  接真实源：qunmind export-items --output items.json，再 --source news:items.json"
