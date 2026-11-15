#!/usr/bin/env python3
"""解释层：让模型写人话，但每个数字都要过护栏。

流程：事实 → 模型写 3 句 → 数字护栏核对 → 不合格重试一次 → 仍不合格就标记拒绝。
护栏只比对数字集合，不做语义判断；它的作用是「不让模型引入事实之外的数字」。
"""
import argparse, json, os, re, sys, time, urllib.request
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone, timedelta

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from guard import check, extract  # noqa: E402
from scoring import score  # noqa: E402

CFG = os.path.expanduser("~/.hermes/profiles/ai/config.yaml")
TZ = timezone(timedelta(hours=8))
MODEL = "deepseek-flash"

SYSTEM = (
    "你是数据质量报告的解释员，写给要用这份数据的人看。规则："
    "1) 只能依据给定事实，不得引入外部知识，不推测原因；"
    "2) 不得出现事实之外的数字；全文最多引用 3 个数字，能不用就不用；"
    "3) 不要罗列事实，不要复述字段清单；"
    "4) 固定三句：第一句说最要紧的问题，第二句说这对使用者意味着什么，第三句给一个可执行的建议；"
    "5) 不用 markdown，不用『我』，不超过 120 字。"
)


def load_key() -> str:
    lines = open(CFG, encoding="utf-8").read().splitlines()
    for i, ln in enumerate(lines):
        if "api.deepseek.com" in ln and "/anthropic" not in ln:
            for j in range(max(0, i - 4), min(len(lines), i + 5)):
                s = lines[j].strip()
                if s.startswith("api_key:"):
                    return s.split(":", 1)[1].strip().strip('"\'')
    raise SystemExit("找不到 DeepSeek 官方 key")


KEY = load_key()


def call(facts: dict, hint: str = "") -> str:
    body = {
        "model": MODEL,
        "max_tokens": 900,
        "system": SYSTEM,
        "messages": [{"role": "user", "content": [{"type": "text", "text":
            "事实：\n" + json.dumps(facts, ensure_ascii=False, indent=1) + "\n\n"
            + (hint or "请写 3 句解释。")}]}],
    }
    req = urllib.request.Request(
        "https://api.deepseek.com/anthropic/v1/messages",
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json", "x-api-key": KEY},
    )
    with urllib.request.urlopen(req, timeout=90) as r:
        d = json.loads(r.read())
    return "".join(b.get("text", "") for b in d.get("content", []) if b.get("type") == "text").strip()


CYCLE = {"1": "每日", "7": "每周", "30": "每月", "365": "每年",
         "-1": "停更", "999": "不更新", "不更新": "不更新"}


def facts_of(it, lag):
    rate = it.get("emptyRate")
    f = {
        "数据集": it.get("name"),
        "部门": (it.get("dept") or "").split("/")[-1],
        "声明列数": it.get("declaredCols"),
        "实际样例列数": it.get("sampleCols"),
        "样例行数": it.get("sampleRows"),
        "样例空值率": f"{round(rate * 100, 2)}%" if isinstance(rate, (int, float)) else None,
        "整列为空的列数": it.get("allEmptyCols"),
        "更新频率": CYCLE.get(str(it.get("updateCycle")), it.get("updateCycle")),
        "字段一致性": it.get("schemaKind"),
        "字段说明字数": it.get("descLen"),
    }
    if lag is not None:
        f["距上次更新天数"] = lag
    if it.get("missingFields"):
        f["声明有但样例没有的字段"] = it["missingFields"][:6]
    if it.get("extraFields"):
        f["样例有但声明没有的字段"] = it["extraFields"][:6]
    return {k: v for k, v in f.items() if v is not None}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--items", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--catalog", default="", help="目录快照，用来给出滞后天数")
    ap.add_argument("--top", type=int, default=30)
    ap.add_argument("--threads", type=int, default=3)
    a = ap.parse_args()

    items = [json.loads(l) for l in open(a.items, encoding="utf-8") if l.strip()]
    items = [i for i in items if i.get("hasPreview") == 1]
    items.sort(key=score, reverse=True)
    picked = items[: a.top]

    lag_by_id = {}
    if a.catalog and os.path.exists(a.catalog):
        now = datetime(2026, 11, 14)
        for r in json.load(open(a.catalog, encoding="utf-8")):
            ut = (r.get("updateTime") or "")[:19]
            try:
                lag_by_id[r.get("id")] = (now - datetime.strptime(ut, "%Y-%m-%d %H:%M:%S")).days
            except ValueError:
                pass

    def work(it):
        facts = facts_of(it, lag_by_id.get(it.get("id")))
        name = it.get("name", "")
        for attempt in (1, 2):
            hint = "" if attempt == 1 else (
                "上一版不合格（数字越界、引用过多或被截断），请重写：只写三句完整的中文，不超过 120 字，数字最多 3 个，结尾必须是句号。")
            try:
                text = call(facts, hint)
            except Exception as e:
                return {"id": it["id"], "name": name, "status": "error",
                        "error": str(e)[:160]}
            if not text.strip().endswith(("。", "！", "？", "”")):
                print(f"  CUT   {name[:24]} 结尾不完整 (第 {attempt} 次)")
                if attempt == 2:
                    return {"id": it["id"], "name": name, "status": "truncated",
                            "text": text, "attempts": attempt}
                continue
            if len(text.strip()) < 15:
                print(f"  EMPTY {name[:24]} (第 {attempt} 次)")
                if attempt == 2:
                    return {"id": it["id"], "name": name, "status": "empty",
                            "attempts": attempt}
                continue
            ok, bad = check(text, facts)
            if ok and len(extract(text)) > 3:
                ok, bad = False, sorted(extract(text), key=len)[3:]
            if ok:
                print(f"  PASS {name[:24]}  {len(text)} 字")
                return {"id": it["id"], "name": name, "text": text,
                        "status": "pass", "attempts": attempt,
                        "numbers": sorted(extract(text), key=len)}
            print(f"  FAIL {name[:24]}  越界数字 {bad} (第 {attempt} 次)")
        return {"id": it["id"], "name": name, "text": text, "status": "rejected",
                "attempts": 2, "illegal": bad}

    t0 = time.time()
    with ThreadPoolExecutor(max_workers=a.threads) as ex:
        recs = list(ex.map(work, picked))

    passed = [r for r in recs if r.get("status") == "pass"]
    out = {
        "generatedAt": datetime.now(TZ).isoformat(timespec="seconds"),
        "model": MODEL,
        "guard": "输出中的每个数字必须出现在事实里；不合格重试一次，仍不合格标记 rejected",
        "total": len(recs),
        "passed": len(passed),
        "elapsedSec": round(time.time() - t0, 1),
        "items": recs,
    }
    json.dump(out, open(os.path.join(a.out, "explanations.json"), "w"),
              ensure_ascii=False, indent=1)
    print(f"\n解释层: {len(passed)}/{len(recs)} 条通过数字护栏，"
          f"耗时 {out['elapsedSec']}s → {a.out}/explanations.json")


if __name__ == "__main__":
    main()
