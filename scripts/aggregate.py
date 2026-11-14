#!/usr/bin/env python3
"""把 items.jsonl 聚合成体检结论：summary.json + leaderboard.md + top20.lean

用法:
  python3 aggregate.py --items ../out-full/items.jsonl --catalog /tmp/zjopen/catalog.json \
      --out ../out-full
"""
import argparse, json, os, statistics, hashlib
from collections import Counter, defaultdict
from scoring import classify, score

ap = argparse.ArgumentParser()
ap.add_argument("--items", required=True)
ap.add_argument("--catalog", default="", help="目录全量（补 updateTime）")
ap.add_argument("--out", required=True)
a = ap.parse_args()

items = []
with open(a.items, encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if line:
            try:
                items.append(json.loads(line))
            except json.JSONDecodeError:
                pass

# 目录补字段（updateTime / modifyDate）
extra = {}
if a.catalog and os.path.exists(a.catalog):
    for r in json.load(open(a.catalog, encoding="utf-8")):
        extra[r["id"]] = {"updateTime": r.get("updateTime"), "modifyDate": r.get("modifyDate"),
                          "publishDate": r.get("publishDate"), "dataSize": r.get("dataSize")}
for it in items:
    it.update({k: v for k, v in extra.get(it["id"], {}).items() if v})

# 时效性：最后更新时间 vs 今天，以及「声称更新频率」是否兑现
from datetime import datetime
NOW = datetime(2026, 11, 14)
BAD_CYCLE = {"1": 7, "7": 10, "30": 45, "365": 400, "每年": 400}
lags, overdue = [], []
for it in items:
    ut = it.get("updateTime")
    if not ut:
        continue
    try:
        dt = datetime.strptime(ut[:19], "%Y-%m-%d %H:%M:%S")
    except ValueError:
        continue
    d = (NOW - dt).days
    lags.append(max(0, d))
    cyc = str(it.get("updateCycle"))
    if cyc in BAD_CYCLE and d > BAD_CYCLE[cyc]:
        overdue.append(it)

n = len(items)
with_preview = [i for i in items if i.get("sampleRows")]
rates = [i["emptyRate"] for i in with_preview if i.get("emptyRate") is not None]
all_empty = [i for i in with_preview if i.get("allEmptyCols", 0) > 0]
miss_kind = Counter(classify(i) for i in items)
mismatch = [i for i in items if classify(i) in ("列数不一致", "字段集合不一致", "仅命名不一致")]
no_desc = [i for i in items if i.get("descLen", 0) < 10]
no_cols = [i for i in items if not i.get("declaredCols")]
cycle = Counter(str(i.get("updateCycle")) for i in items)
STOPPED = {"-1", "999", "不更新"}
stopped = [i for i in items if str(i.get("updateCycle")) in STOPPED]
dept = defaultdict(lambda: {"n": 0, "preview": 0, "empty_sum": 0.0, "empty_n": 0,
                            "all_empty": 0, "mismatch": 0, "no_desc": 0, "stopped": 0})
for i in items:
    d = (i.get("dept") or "未知").split("/")[-1]
    s = dept[d]
    s["n"] += 1
    if i.get("sampleRows"):
        s["preview"] += 1
    if i.get("emptyRate") is not None and i.get("sampleRows"):
        s["empty_sum"] += i["emptyRate"]; s["empty_n"] += 1
    if i.get("allEmptyCols", 0) > 0: s["all_empty"] += 1
    if classify(i) in ("列数不一致", "字段集合不一致", "仅命名不一致"): s["mismatch"] += 1
    if i.get("descLen", 0) < 10: s["no_desc"] += 1
    if str(i.get("updateCycle")) in STOPPED: s["stopped"] += 1

summary = {
    "audited": n,
    "withPreview": len(with_preview),
    "emptyRateMean": round(sum(rates) / len(rates), 4) if rates else None,
    "emptyRateMedian": round(statistics.median(rates), 4) if rates else None,
    "emptyRateP90": round(sorted(rates)[int(len(rates) * 0.9)], 4) if rates else None,
    "emptyRateOver30": sum(1 for r in rates if r > 0.3),
    "allEmptyColumnDatasets": len(all_empty),
    "schemaMismatch": len(mismatch),
    "mismatchKinds": dict(miss_kind),
    "noDescription": len(no_desc),
    "noColumnList": len(no_cols),
    "stoppedUpdate": len(stopped),
    "lagMedianDays": int(statistics.median(lags)) if lags else None,
    "lagOver90Days": sum(1 for d in lags if d > 90),
    "lagOver365Days": sum(1 for d in lags if d > 365),
    "claimedCycleOverdue": len(overdue),
    "updateCycle": dict(cycle.most_common(8)),
    "byDept": {k: {**v, "empty_avg": round(v["empty_sum"] / v["empty_n"], 4) if v["empty_n"] else None}
               for k, v in sorted(dept.items(), key=lambda kv: -kv[1]["n"])},
}
json.dump(summary, open(f"{a.out}/summary.json", "w"), ensure_ascii=False, indent=1)

# ---- 榜单：问题最多的数据集（评分逻辑见 scoring.py） ----

ranked = sorted(items, key=score, reverse=True)[:50]
lines = ["# 浙江开放数据健康榜（问题 Top 50）", "",
         f"体检范围：{n} 个数据集；有样例：{len(with_preview)}；"
         f"样例级平均空值率：{(summary['emptyRateMean'] or 0) * 100:.1f}%；"
         f"字段不一致：{len(mismatch)}；停更标注：{len(stopped)}", "",
         "| # | 数据集 | 部门 | 声明列 | 样例列 | 类型 | 空值率 | 全空列 | 停更 |",
         "|---|---|---|---|---|---|---|---|---|"]
for k, i in enumerate(ranked, 1):
    lines.append("| {} | {} | {} | {} | {} | {} | {:.1f}% | {} | {} |".format(
        k, i.get("name", "")[:28], (i.get("dept") or "").split("/")[-1],
        i.get("declaredCols", 0), i.get("sampleCols", 0), classify(i),
        (i.get("emptyRate") or 0) * 100, i.get("allEmptyCols", 0),
        "是" if str(i.get("updateCycle")) in STOPPED else ""))
open(f"{a.out}/leaderboard.md", "w").write("\n".join(lines) + "\n")

# ---- 部门榜 ----
bl = ["# 部门榜（按数据集数）", "", "| 部门 | 数据集 | 有样例 | 平均空值率 | 全空列 | 字段不一致 | 说明不足 | 停更 |",
      "|---|---|---|---|---|---|---|---|"]
for d, s in list(summary["byDept"].items())[:25]:
    bl.append("| {} | {} | {} | {} | {} | {} | {} | {} |".format(
        d, s["n"], s["preview"],
        f"{s['empty_avg']*100:.1f}%" if s["empty_avg"] is not None else "—",
        s["all_empty"], s["mismatch"], s["no_desc"], s["stopped"]))
open(f"{a.out}/leaderboard_dept.md", "w").write("\n".join(bl) + "\n")

print(json.dumps({k: v for k, v in summary.items() if k != "byDept"}, ensure_ascii=False, indent=1))
print("\n问题 Top 10:")
for i in ranked[:10]:
    print(f"  {score(i):>5} | {i.get('name','')[:34]} | 空值{(i.get('emptyRate') or 0)*100:.0f}% "
          f"| 缺{len(i.get('missingFields') or [])}列 | {i.get('dept','').split('/')[-1]}")
print(f"\n输出: {a.out}/summary.json, leaderboard.md, leaderboard_dept.md")
