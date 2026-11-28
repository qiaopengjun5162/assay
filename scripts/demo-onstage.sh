#!/usr/bin/env bash
# 路演现场：把观众随手给的一份 JSON 当场体检，输出一眼看懂的问题清单。
#
# 和 reuse-demo.sh 的区别：那个证明「能换源」，是开发者视角、原始输出；
# 这个面向观众——一条命令吃任意 JSON，把 10 条检查的命中翻成「几条有什么毛病」，
# 用来现场回答「给我一份你的数据，我现在就能查」。
#
# 用法：
#   bash scripts/demo-onstage.sh                      # 用内置演示数据
#   bash scripts/demo-onstage.sh /path/to/mydata.json # 用观众自带的数据
#
# 退出码：0 = 跑出了检查结论（哪怕命中 0）；非 0 = 数据源或链路坏了。
# 不碰 web/ 等冻结产物：所有产物进临时目录，退出即删。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SRC="${1:-$ROOT/fixtures/daily-news.json}"
if [[ ! -f "$SRC" ]]; then
    echo "找不到数据文件：$SRC" >&2
    echo "用法：bash scripts/demo-onstage.sh [你的 JSON 路径]" >&2
    exit 1
fi

# 前置字段体检：适配器认 title / publishedAt，不是 name / updatedAt。
# 现场用观众数据最容易在这里翻车——名字空白、超期全 0，还以为是工具坏了。
python3 - "$SRC" <<'PY'
import json, sys
p = sys.argv[1]
try:
    d = json.load(open(p, encoding="utf-8"))
except Exception as e:
    print(f"JSON 解析失败：{e}", file=sys.stderr); raise SystemExit(1)
for key in ("platform", "declaredColumns", "items"):
    if key not in d:
        print(f"缺顶层字段 {key}（格式见 fixtures/daily-news.json）", file=sys.stderr)
        raise SystemExit(1)
items = d["items"] or []
if not items:
    print("items 是空的，没有可体检的条目", file=sys.stderr); raise SystemExit(1)
if any(not str(x.get("id", "")).strip() for x in items):
    print("有条目缺 id；id 必须唯一且非空", file=sys.stderr); raise SystemExit(1)
bad = [k for k in ("title", "publishedAt") if any(k in x for x in items)]
if not bad and any(("name" in x or "updatedAt" in x) for x in items):
    print("提示：条目用了 name / updatedAt，适配器认的是 title / publishedAt；", file=sys.stderr)
    print("      超期与条目名会算不出来（不是工具坏了）。", file=sys.stderr)
PY

cargo build --release -q -p assay
BIN="$ROOT/target/release/assay"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

echo "══ 当场体检 ══════════════════════════════════════════"
echo "数据源：$SRC"
echo "────────────────────────────────────────────────────────"
echo "① 用同一套内核体检（audit）"
"$BIN" --out "$OUT" --source "$SRC" audit --threads 2 | tail -2

echo "② 聚合命中数（report）"
"$BIN" --out "$OUT" report | tail -4

echo "③ 把命中翻成人话（观众看的这一屏）"
python3 - "$OUT/items.jsonl" "$OUT/summary.json" <<'PY'
import json, sys

items_path, summary_path = sys.argv[1], sys.argv[2]
rows = [json.loads(l) for l in open(items_path, encoding="utf-8") if l.strip()]
s = json.load(open(summary_path, encoding="utf-8"))

# 数字全部取 summary 的权威字段，这里不重算任何口径——
# 自己现算容易和 checks.rs 的判定分叉（历史上 schemaMismatch 就出过两个数）。
bad_fields = [r for r in rows if r.get("missingFields")]

print()
print(f"  这次查了 {s.get('audited', len(rows))} 条数据，结果：")
print(f"  • 声明有、实际给的字段对不上：{s.get('schemaMismatch', '?')} 条")
for r in bad_fields[:3]:
    miss = "、".join(r["missingFields"][:4])
    print(f"      「{r.get('name', '(无名)')[:24]}」缺：{miss}")
print(f"  • 超过 90 天没更新：{s.get('lagOver90Days', '?')} 条")
print(f"  • 字段说明不足：{s.get('noDescription', '?')} 条")
print(f"  • 整列全空：{s.get('allEmptyColumnDatasets', '?')} 条")
print()
print("  这些数字不是我说的——是 10 条公开规则算出来的，改阈值就是改结论。")
PY

echo "════════════════════════════════════════════════════════"
echo "  同一套尺子，换你的数据也一样查。用你自己的 JSON 重跑："
echo "  bash scripts/demo-onstage.sh /path/to/your.json"
