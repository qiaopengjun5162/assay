#!/usr/bin/env python3
"""把体检结果导出成前端可直接读的静态 JSON（demo 用静态站点，不做后端）。

用法: python3 export_web.py --items ../out-full/items.jsonl --summary ../out-full/summary.json \
        --proofs ../out-full/proofs.json --out ../web/data.json --limit 300
"""
import argparse, json, os
from scoring import score

ap = argparse.ArgumentParser()
ap.add_argument("--items", required=True)
ap.add_argument("--summary", required=True)
ap.add_argument("--proofs", default="")
ap.add_argument("--out", required=True)
ap.add_argument("--limit", type=int, default=300)
a = ap.parse_args()

STOPPED = {"-1", "999", "不更新"}


def score(i):
    s = 0.0
    if i.get("missingFields"): s += 3
    if i.get("extraFields"): s += 2
    if i.get("allEmptyCols", 0) > 0: s += 2
    s += min(3, (i.get("emptyRate") or 0) * 6)
    if str(i.get("updateCycle")) in STOPPED: s += 1
    if i.get("descLen", 0) < 10: s += 1
    if not i.get("hasPreview"): s += 1
    return round(s, 2)


items = []
with open(a.items, encoding="utf-8") as f:
    for line in f:
        try:
            items.append(json.loads(line))
        except json.JSONDecodeError:
            pass

summary = json.load(open(a.summary, encoding="utf-8"))
proofs = json.load(open(a.proofs, encoding="utf-8")) if a.proofs and os.path.exists(a.proofs) else []
proof_by_id = {p.get("id"): p for p in proofs if "sha256" in p}

rows = []
for i in items:
    rows.append({
        "id": i.get("id"),
        "name": i.get("name"),
        "dept": (i.get("dept") or "").split("/")[-1],
        "openType": i.get("openType"),
        "updateCycle": i.get("updateCycle"),
        "updateDetail": i.get("updateCycleDetail"),
        "declaredCols": i.get("declaredCols"),
        "sampleCols": i.get("sampleCols"),
        "sampleRows": i.get("sampleRows"),
        "emptyRate": i.get("emptyRate"),
        "allEmptyCols": i.get("allEmptyCols"),
        "missingFields": i.get("missingFields") or [],
        "extraFields": i.get("extraFields") or [],
        "descLen": i.get("descLen"),
        "hasPreview": i.get("hasPreview"),
        "score": score(i),
        "proof": proof_by_id.get(i.get("id")),
    })

rows.sort(key=lambda r: r["score"], reverse=True)
out = {
    "generatedAt": "2026-11-14",
    "source": "浙江省公共数据开放平台（data.zjzwfw.gov.cn）",
    "api": "/dopServer/dataCatalog/{dataQueryPage,catalogDetail,catalogDataPreview}",
    "summary": summary,
    "datasets": rows[: a.limit],
    "total": len(rows),
}
os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
json.dump(out, open(a.out, "w"), ensure_ascii=False, separators=(",", ":"))
print(f"导出 {len(out['datasets'])} 条（共 {len(rows)} 条）→ {a.out} "
      f"({os.path.getsize(a.out)/1024:.0f} KB)")
