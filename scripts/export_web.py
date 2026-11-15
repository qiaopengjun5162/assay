#!/usr/bin/env python3
"""把体检结果导出成前端可直接读的静态 JSON（demo 用静态站点，不做后端）。

用法: python3 export_web.py --items ../out-full/items.jsonl --summary ../out-full/summary.json \
        --proofs ../out-full/proofs.json --out ../web/data.json --limit 300
"""
import argparse, json, os
from datetime import datetime
from scoring import score, classify

NOW = datetime(2026, 11, 14)

ap = argparse.ArgumentParser()
ap.add_argument("--items", required=True)
ap.add_argument("--summary", required=True)
ap.add_argument("--proofs", default="")
ap.add_argument("--catalog", default="", help="目录快照，用来补 updateTime 与滞后天数")
ap.add_argument("--evidence", default="", help="证据包索引 evidence.json")
ap.add_argument("--explanations", default="", help="解释层 explanations.json")
ap.add_argument("--out", required=True)
ap.add_argument("--limit", type=int, default=300)
a = ap.parse_args()

STOPPED = {"-1", "999", "不更新"}


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

evidence = {}
if a.evidence and os.path.exists(a.evidence):
    for e in json.load(open(a.evidence, encoding="utf-8")).get("items", []):
        if e.get("sha256"):
            evidence[e["id"]] = {
                "sha256": e["sha256"], "fetchedAt": e.get("fetchedAt"),
                "detailBytes": e.get("detailBytes"), "previewBytes": e.get("previewBytes"),
                "files": e.get("files", []),
            }

catalog = {}
if a.catalog and os.path.exists(a.catalog):
    for r in json.load(open(a.catalog, encoding="utf-8")):
        catalog[r["id"]] = r

BAD_CYCLE = {"1": 7, "7": 10, "30": 45, "365": 400, "每年": 400}

def lag_of(item):
    ut = (catalog.get(item.get("id")) or {}).get("updateTime") or ""
    if len(ut) < 19:
        return None, None
    try:
        d = (NOW - datetime.strptime(ut[:19], "%Y-%m-%d %H:%M:%S")).days
    except ValueError:
        return None, ut
    cyc = str(item.get("updateCycle"))
    overdue = cyc in BAD_CYCLE and d > BAD_CYCLE[cyc]
    return max(0, d), ut

explanations = {}
if a.explanations and os.path.exists(a.explanations):
    for e in json.load(open(a.explanations, encoding="utf-8")).get("items", []):
        if e.get("text"):
            explanations[e["id"]] = {"text": e["text"], "status": e.get("status")}

rows = []
for i in items:
    lag, ut = lag_of(i)
    cyc = str(i.get("updateCycle"))
    rows.append({
        "id": i.get("id"),
        "name": i.get("name"),
        "dept": (i.get("dept") or "").split("/")[-1],
        "openType": i.get("openType"),
        "updateCycle": i.get("updateCycle"),
        "updateDetail": i.get("updateCycleDetail"),
        "updateTime": ut,
        "lagDays": lag,
        "overdue": bool(cyc in BAD_CYCLE and lag is not None and lag > BAD_CYCLE[cyc]),
        "declaredCols": i.get("declaredCols"),
        "sampleCols": i.get("sampleCols"),
        "sampleRows": i.get("sampleRows"),
        "emptyRate": i.get("emptyRate"),
        "allEmptyCols": i.get("allEmptyCols"),
        "missingFields": i.get("missingFields") or [],
        "extraFields": i.get("extraFields") or [],
        "schemaKind": classify(i),
        "descLen": i.get("descLen"),
        "hasPreview": i.get("hasPreview"),
        "score": score(i),
        "proof": proof_by_id.get(i.get("id")),
        "evidence": evidence.get(i.get("id")),
        "explain": explanations.get(i.get("id")),
    })

rows.sort(key=lambda r: r["score"], reverse=True)
out = {
    "generatedAt": "2026-11-14",
    "source": "浙江省公共数据开放平台（data.zjzwfw.gov.cn）",
    "api": "/dopServer/dataCatalog/{dataQueryPage,catalogDetail,catalogDataPreview}",
    "summary": dict(summary, proofCount=len(proof_by_id), evidenceCount=len(evidence), explainCount=len(explanations)),
    "datasets": rows[: a.limit],
    "total": len(rows),
}
os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
json.dump(out, open(a.out, "w"), ensure_ascii=False, separators=(",", ":"))
print(f"导出 {len(out['datasets'])} 条（共 {len(rows)} 条）→ {a.out} "
      f"({os.path.getsize(a.out)/1024:.0f} KB)")
