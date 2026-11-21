#!/usr/bin/env bash
# 深圳市政府数据开放平台探针：只读、不需要账号、不打印任何密钥。
#
# 为什么要这个脚本：`docs/audit/SHENZHEN.md` 里写了一串「实测」，包括
# 「目录能匿名枚举 2,791 条」「163 条已过期」「没 key 报 10009」「错 key 报 10001」。
# 这类断言最怕变成传说，所以留一条不依赖本项目的命令，谁都能当场复核。
# 第 1b 节直接解析已下载的两页目录重算统计数——它们以前只能人工复核。
#
# 用法：
#   ./scripts/sz-probe.sh              # 只验「不需要 key」的部分
#   ./scripts/sz-probe.sh <appKey>     # 顺带验一次真实调用（key 不会出现在输出里）
#
# 只发 GET/POST，不写任何本地文件（临时文件走 mktemp 并自动清理）。
set -euo pipefail

BASE="https://opendata.sz.gov.cn"
# 注意目录里 resId 是「29200/00000001」（带斜杠），而调用地址用下划线：
# 写成斜杠会得到 HTTP 200 + 平台首页 HTML，不是错误码——静默的坑。
RES="29200_00000001"
KEY="${1:-}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

say() { printf '%s\n' "$*"; }

say "== 1. 目录能不能匿名枚举（POST /data/api/findPageByParams）"
# 参数名照抄页面上的 findPage()：pageNo/pageSize 之外那几个留空即可
for p in 1 2; do
  curl -sS --max-time 60 -X POST "$BASE/data/api/findPageByParams" \
    -H "X-Requested-With: XMLHttpRequest" \
    -d "pageNo=$p&pageSize=2000&sortType=&domainId=&deptCode=&classifyType=&classifyId=" \
    -o "$TMP/p$p.json"
  # 返回体里 apiList 是一段**被转义的 JSON 字符串**（`\"resTitle\"`），
  # 所以按裸词计数，别按带引号的字面量匹配——那个永远匹配不上。
  n=$(grep -o 'resTitle' "$TMP/p$p.json" | wc -l | tr -d ' ')
  say "   第 $p 页返回 $n 条"
done
# 分层计数是接口自己给的，不用我们算
count() { grep -o "$1[^0-9]*[0-9]\+" "$2" | head -1 | grep -o '[0-9]\+$' || true; }
say "   市级 $(count cityCount "$TMP/p1.json") / 区级 $(count countyCount "$TMP/p1.json")"

say ""
say "== 1b. 匿名目录当场能算出的结论（不需要 appKey）=="
# 早期笔记只写了「163 条过期 / 2,563 条无条件」两个数，没有复核命令——
# 与「别让实测变成传说」的初衷矛盾。目录两页已经下载好了，直接算。
python3 - "$TMP/p1.json" "$TMP/p2.json" <<'PY'
import json
import sys
from collections import Counter
from datetime import date, datetime

today = date.today()
rows = []
for path in sys.argv[1:]:
    with open(path, encoding="utf-8") as f:
        rows += json.loads(json.load(f)["apiList"])["list"]

level = Counter(r.get("openLevelName") for r in rows)
print(f"   共 {len(rows):,} 条；无条件开放 {level['无条件开放']:,} / 有条件开放 {level['有条件开放']:,}")

# 深圳的 updateCycle 是中文，assay 的 BAD_CYCLE 表用数字编码（1/30/365）；
# 这里按 docs/audit/SHENZHEN.md 的映射换算成同等阈值 7/45/400 天。
threshold = {"按日": 7, "按月": 45, "按年": 400}
unknown = Counter()
stale = []
for r in rows:
    cycle = r.get("updateCycle")
    if cycle not in threshold:
        unknown[cycle] += 1
        continue
    try:
        lag = (today - datetime.strptime(str(r.get("dataUpdateTime"))[:10], "%Y-%m-%d").date()).days
    except (ValueError, TypeError):
        unknown["日期不可解析"] += 1
        continue
    if lag > threshold[cycle]:
        stale.append((lag, r.get("resTitle"), cycle))

judgeable = len(rows) - sum(unknown.values())
print(f"   可判定 {judgeable:,} 条；判不了 {sum(unknown.values()):,} 条：" +
      " / ".join(f"{k} {v:,}" for k, v in unknown.most_common()))
print(f"   按 assay 阈值已过期：{len(stale):,} 条")
if stale:
    lag, title, cycle = max(stale)
    print(f"   最久未更新：{lag:,} 天（{title}，声称{cycle}）")
PY

say ""
say "== 2. 不发 key 调数据（GET /api/$RES/1/service.xhtml）"
curl -sS --max-time 30 "$BASE/api/$RES/1/service.xhtml?page=1&rows=2" -o "$TMP/nokey.json"
say "   $(head -c 200 "$TMP/nokey.json")"

say ""
say "== 3. 发一个假 key 调数据"
curl -sS --max-time 30 \
  "$BASE/api/$RES/1/service.xhtml?page=1&rows=2&appKey=00000000000000000000000000000000" \
  -o "$TMP/badkey.json"
say "   $(head -c 200 "$TMP/badkey.json")"

if [ -n "$KEY" ]; then
  say ""
  say "== 4. 用真 key 调一次（前 400 字符）"
  # 用 --data-urlencode 拼 query，避免 key 出现在 ps / 历史记录里
  curl -sS --max-time 30 -G "$BASE/api/$RES/1/service.xhtml" \
    --data-urlencode "page=1" --data-urlencode "rows=2" --data-urlencode "appKey=$KEY" \
    -o "$TMP/ok.json"
  say "   $(head -c 400 "$TMP/ok.json")"
else
  say ""
  say "== 4. 跳过真实调用（没传 appKey）。拿到 key 后：./scripts/sz-probe.sh <appKey>"
fi

say ""
say "结论口径：第 1 节能通说明目录开放；第 2/3 节的 errorCode 说明接口活着、只差授权。"
