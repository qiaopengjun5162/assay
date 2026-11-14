#!/usr/bin/env python3
"""给榜单前 N 名生成 Lean 证明并跑内核校验。

用法: python3 proof_top.py --items ../out-full/items.jsonl --out ../out-full --top 20
产物: TopProofs.lean / proofs.md / verify.txt
"""
import argparse, hashlib, json, os, subprocess, urllib.parse, urllib.request
from scoring import score

BASE = "https://data.zjzwfw.gov.cn/dopServer"


def post(path, body, timeout=40):
    req = urllib.request.Request(BASE + path, data=json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json",
                                          "Referer": BASE + "/", "User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.loads(r.read())


def enc(i):
    return i.replace("+", "%2B").replace("/", "%2F").replace("=", "%3D")


STOPPED = {"-1", "999", "不更新"}




ap = argparse.ArgumentParser()
ap.add_argument("--items", required=True)
ap.add_argument("--out", required=True)
ap.add_argument("--top", type=int, default=20)
ap.add_argument("--evidence", default="", help="evidence.json：优先用已存证的原始响应生成证明")
a = ap.parse_args()

items = [json.loads(l) for l in open(a.items, encoding="utf-8") if l.strip()]
items = [i for i in items if i.get("hasPreview") == 1]
items.sort(key=score, reverse=True)
picked = items[:a.top]

ev = {}
if a.evidence and os.path.exists(a.evidence):
    for e in json.load(open(a.evidence, encoding="utf-8")).get("items", []):
        if e.get("sha256") and e.get("files"):
            ev[e["id"]] = e
base_dir = os.path.dirname(os.path.abspath(a.evidence)) if a.evidence else ""


def load_preview(it):
    """优先读已存证的原始响应；没有才现场抓。保证「证的就是存下来的那份」。"""
    e = ev.get(it["id"])
    if e:
        path = os.path.join(base_dir, e["files"][1])
        if os.path.exists(path):
            return json.load(open(path, encoding="utf-8")), "snapshot"
    return post("/dataCatalog/catalogDataPreview/" + enc(it["id"]), {"current": 1, "size": 10}), "live"


blocks, theorems, meta = [], [], []
for k, it in enumerate(picked):
    try:
        pv, src = load_preview(it)
        rows = pv["result"]["data"]
    except Exception as e:
        meta.append({"name": it.get("name"), "status": f"抓取失败: {e}"})
        continue
    if not rows:
        continue
    header, body = rows[0], rows[1:]
    if not body:
        continue
    ncols = len(header)
    matrix = [[1 if str(r[c]).strip() == "" else 0 for c in range(ncols) if c < len(r)]
              for r in body]
    matrix = [m + [0] * (ncols - len(m)) for m in matrix]
    per_col = [sum(r[c] for r in matrix) for c in range(ncols)]
    snap = json.dumps(rows, ensure_ascii=False).encode()
    h = hashlib.sha256(snap).hexdigest()
    blocks.append("-- {}：{} | 样例 {}x{} | 空值率 {:.1f}%".format(
        k, it.get("name"), len(body), ncols, (it.get("emptyRate") or 0) * 100))
    blocks.append("def rows_{} : List (List Nat) := [\n{}\n]".format(
        k, ",\n".join("  [" + ", ".join(map(str, r)) + "]" for r in matrix)))
    for c in range(ncols):
        theorems.append(f"theorem empty_{k}_{c} : countEmpty rows_{k} {c} = {per_col[c]} := by decide")
    meta.append({"index": k, "name": it.get("name"), "id": it["id"], "dept": it.get("dept"),
                 "rows": len(body), "cols": ncols, "emptyRate": it.get("emptyRate"),
                 "allEmptyCols": it.get("allEmptyCols"),
                 "missingFields": it.get("missingFields"), "sha256": h, "source": src})

head = ("/-\n  自动生成 by proof_top.py\n  平台: 浙江省公共数据开放平台\n"
        "  说明: 1 = 空值；每行对应一条真实样例记录。改动任何数字，内核报错。\n-/\n\n"
        "/-- 某列空值条数 -/\ndef countEmpty (rs : List (List Nat)) (col : Nat) : Nat :=\n"
        "  (rs.filter (fun r => r.getD col 0 == 1)).length\n\n")
lean = head + "\n\n".join(blocks) + "\n\n" + "\n".join(theorems) + "\n"
path = os.path.join(a.out, "TopProofs.lean")
open(path, "w").write(lean)

r = subprocess.run(["lean", path], capture_output=True, text=True, timeout=300)
ok = r.returncode == 0
open(os.path.join(a.out, "verify.txt"), "w").write("PASS\n" if ok else "FAIL\n" + r.stdout[-2000:])
json.dump(meta, open(os.path.join(a.out, "proofs.json"), "w"), ensure_ascii=False, indent=1)

md = ["# 榜单前 {} 名的机器可复核证明".format(len(meta)), "",
      "运行 `lean TopProofs.lean` 的结果：**{}**（退出码 {}）".format("PASS" if ok else "FAIL", r.returncode),
      "", "| # | 数据集 | 样例 | 空值率 | 定理数 | 快照 SHA-256 |", "|---|---|---|---|---|---|"]
for m in meta:
    if "sha256" not in m:
        md.append(f"| - | {m['name']} | {m['status']} | - | - | - |")
        continue
    md.append("| {} | {} | {}×{} | {:.1f}% | {} | `{}…` |".format(
        m["index"], m["name"][:26], m["rows"], m["cols"], (m.get("emptyRate") or 0) * 100,
        m["cols"], m["sha256"][:16]))
open(os.path.join(a.out, "proofs.md"), "w").write("\n".join(md) + "\n")

print(f"定理 {len(theorems)} 条，lean 校验: {'PASS' if ok else 'FAIL'}")
if not ok:
    print(r.stdout[-1200:])
print(f"产物: {path}, proofs.md, verify.txt")
