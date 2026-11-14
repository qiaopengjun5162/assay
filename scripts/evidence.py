#!/usr/bin/env python3
"""证据包：把平台接口的原始响应落盘并算哈希（确定性抓取，不经过任何模型）。

为什么不用 LLM 抓页面（2026-11-14 实测结论）：
  DeepSeek read_url 读平台详情页时，返回的文档 source.data 长度为 0（页面是客户端渲染的），
  但模型仍在 thinking 里「原样抄出」了「数据条数：382」这类页面上不存在的数字。
  空文档 + 模型补全 = 伪造证据。所以存证通道必须是确定性抓取。

用法:
  python3 evidence.py --items ../out-full/items.jsonl --out ../out-full --top 20
"""
import argparse, hashlib, json, os, re, time, urllib.request
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone, timedelta

BASE = "https://data.zjzwfw.gov.cn/dopServer"
TZ = timezone(timedelta(hours=8))


def post(path, body, timeout=40):
    req = urllib.request.Request(
        BASE + path,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json", "Referer": BASE + "/",
                 "User-Agent": "Mozilla/5.0 dataproof/0.1"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


def safe_id(i: str) -> str:
    return re.sub(r"[^A-Za-z0-9_-]", "_", i)[:64]


def enc(i: str) -> str:
    return i.replace("+", "%2B").replace("/", "%2F").replace("=", "%3D")


def main() -> None:
    import sys
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from scoring import score

    ap = argparse.ArgumentParser()
    ap.add_argument("--items", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--top", type=int, default=20)
    ap.add_argument("--threads", type=int, default=4)
    a = ap.parse_args()

    items = []
    for line in open(a.items, encoding="utf-8"):
        line = line.strip()
        if line:
            try:
                items.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    items = [i for i in items if i.get("hasPreview") == 1]
    items.sort(key=score, reverse=True)
    picked = items[: a.top]

    ev_dir = os.path.join(a.out, "evidence")
    os.makedirs(ev_dir, exist_ok=True)

    def work(it):
        did = it["id"]
        d = safe_id(did)
        folder = os.path.join(ev_dir, d)
        os.makedirs(folder, exist_ok=True)
        try:
            raw_detail = post(f"/dataCatalog/catalogDetail/{enc(did)}", {})
            raw_preview = post(f"/dataCatalog/catalogDataPreview/{enc(did)}", {"current": 1, "size": 10})
            open(os.path.join(folder, "detail.json"), "wb").write(raw_detail)
            open(os.path.join(folder, "preview.json"), "wb").write(raw_preview)
            h = hashlib.sha256()
            h.update(raw_detail)
            h.update(b"\n--split--\n")
            h.update(raw_preview)
            rec = {
                "id": did,
                "name": it["name"],
                "fetchedAt": datetime.now(TZ).isoformat(timespec="seconds"),
                "detailBytes": len(raw_detail),
                "previewBytes": len(raw_preview),
                "sha256": h.hexdigest(),
                "files": [f"evidence/{d}/detail.json", f"evidence/{d}/preview.json"],
            }
            print(f"  OK  {it['name'][:26]}  {rec['detailBytes']:>6} + {rec['previewBytes']:>6} 字节")
            return rec
        except Exception as e:
            print(f"  ERR {it['name'][:26]} {str(e)[:80]}")
            return {"id": did, "name": it["name"], "error": str(e)[:200]}

    t0 = time.time()
    with ThreadPoolExecutor(max_workers=a.threads) as ex:
        records = list(ex.map(work, picked))

    ok = [r for r in records if "sha256" in r]
    idx = {
        "capturedAt": datetime.now(TZ).isoformat(timespec="seconds"),
        "source": "浙江省公共数据开放平台接口原始响应（确定性抓取，不经模型）",
        "api": ["/dataCatalog/catalogDetail/{id}", "/dataCatalog/catalogDataPreview/{id}"],
        "total": len(records),
        "captured": len(ok),
        "elapsedSec": round(time.time() - t0, 1),
        "items": records,
    }
    json.dump(idx, open(os.path.join(a.out, "evidence.json"), "w"), ensure_ascii=False, indent=1)
    print(f"\n证据包: {len(ok)}/{len(records)} 条，耗时 {idx['elapsedSec']}s → {a.out}/evidence.json")


if __name__ == "__main__":
    main()
