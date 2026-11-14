#!/usr/bin/env python3
"""DeepSeek 全家桶 API 能力实测。

密钥只从本地 ~/.hermes/profiles/ai/config.yaml 读取，不落盘、不打印。
输出: probe_result.json
"""
import json, os, time, wave, io, sys, urllib.request, urllib.error, struct

CFG = os.path.expanduser("~/.hermes/profiles/ai/config.yaml")
ORIGIN = "https://api.deepseek.com"


def load_key() -> str:
    """从 YAML 里定向抽取 DeepSeek 官方 provider 的 key（不引入 yaml 依赖、不打印）。"""
    lines = open(CFG, encoding="utf-8").read().splitlines()
    for i, ln in enumerate(lines):
        if "api.deepseek.com" in ln and "/anthropic" not in ln:
            for j in range(max(0, i - 4), min(len(lines), i + 5)):
                s = lines[j].strip()
                if s.startswith("api_key:"):
                    return s.split(":", 1)[1].strip().strip('"\'')
    raise SystemExit("找不到 DeepSeek 官方 key")


KEY = load_key()


def req(path, body, beta=False, raw=False, timeout=180):
    data = json.dumps(body).encode() if body is not None else None
    h = {"Content-Type": "application/json", "x-api-key": KEY}
    if beta:
        h["anthropic-beta"] = "files-api-2025-04-14"
    r = urllib.request.Request(ORIGIN + path, data=data, headers=h, method="POST" if data else "GET")
    t0 = time.time()
    try:
        with urllib.request.urlopen(r, timeout=timeout) as resp:
            payload = resp.read()
            return {"status": resp.status, "elapsed": round(time.time() - t0, 2),
                    "body": payload if raw else json.loads(payload)}
    except urllib.error.HTTPError as e:
        return {"status": e.code, "elapsed": round(time.time() - t0, 2),
                "error": e.read().decode()[:800]}
    except Exception as e:
        return {"status": None, "elapsed": round(time.time() - t0, 2), "error": str(e)[:300]}


out = {}

# ---- 0. 计费模型清单（免费） ----
out["models"] = req("/models", None)

# ---- 1. deepseek_read_url：让它读一篇真实链接 ----
out["read_url"] = req("/anthropic/v1/messages", {
    "model": "deepseek-flash",
    "max_tokens": 512,
    "tools": [{"type": "deepseek_read_url", "name": "deepseek_read_url"}],
    "messages": [{"role": "user", "content": [{
        "type": "text",
        "text": "读取这个链接 https://mp.weixin.qq.com/s/78KBHc4-4N5b2vfL9SkutQ "
                "用两句话概括它发布了什么，并列出文中出现的一个具体接口名。"}]}],
})

# ---- 2. deepseek_web_search：需要时效性的问题 ----
out["web_search"] = req("/anthropic/v1/messages", {
    "model": "deepseek-flash",
    "max_tokens": 512,
    "tools": [
        {"type": "deepseek_read_url", "name": "deepseek_read_url"},
        {"type": "deepseek_web_search", "name": "deepseek_web_search"},
    ],
    "messages": [{"role": "user", "content": [{
        "type": "text",
        "text": "现在 DeepSeek API 官方最新模型叫什么？请联网确认并给出你查到的日期。"}]}],
})

# ---- 3. TTS -> WAV -> ASR 闭环 ----
tts = req("/v1/tools/tts", {
    "text": "数据体检，结论可验证，改一个数字证明就报错。",
    "voice_id": "stella", "format": "pcm", "filter_markdown": True}, raw=True, timeout=120)
out["tts"] = {"status": tts["status"], "elapsed": tts["elapsed"],
              "bytes": len(tts.get("body") or b""), "error": tts.get("error")}

if tts["status"] == 200 and len(tts.get("body") or b"") > 1000:
    pcm = tts["body"]
    buf = io.BytesIO()
    with wave.open(buf, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(24000)
        w.writeframes(pcm)
    wav = buf.getvalue()
    boundary = "----probe" + str(int(time.time()))
    parts = (
        f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"probe.wav\"\r\n"
        f"Content-Type: audio/wav\r\n\r\n").encode() + wav + f"\r\n--{boundary}--\r\n".encode()
    r = urllib.request.Request(ORIGIN + "/v1/tools/asr", data=parts, method="POST",
                              headers={"x-api-key": KEY,
                                       "Content-Type": f"multipart/form-data; boundary={boundary}"})
    t0 = time.time()
    try:
        with urllib.request.urlopen(r, timeout=120) as resp:
            out["asr"] = {"status": resp.status, "elapsed": round(time.time() - t0, 2),
                          "body": json.loads(resp.read())}
    except urllib.error.HTTPError as e:
        out["asr"] = {"status": e.code, "error": e.read().decode()[:500]}

json.dump(out, open("probe_result.json", "w"), ensure_ascii=False, indent=1)

# ---- 打印摘要（不含任何密钥） ----
def blocks(r):
    try:
        return [b.get("type") for b in r["body"]["content"]]
    except Exception:
        return None

print("models:", out["models"].get("status"),
      (out["models"].get("body") or {}).get("data") if isinstance(out["models"].get("body"), dict) else out["models"].get("error"))
for k in ("read_url", "web_search"):
    r = out[k]
    print(f"\n{k}: status={r['status']} {r['elapsed']}s blocks={blocks(r)}")
    if r.get("error"):
        print("  error:", r["error"][:300])
    else:
        txt = "".join(b.get("text", "") for b in r["body"].get("content", []) if b.get("type") == "text")
        print("  text:", txt[:300].replace("\n", " "))
        for b in r["body"].get("content", []):
            if b.get("type") not in ("text",):
                print("  block:", b.get("type"), json.dumps(b, ensure_ascii=False)[:240])
print("\ntts:", {k: v for k, v in out["tts"].items()})
print("asr:", json.dumps(out.get("asr", {}), ensure_ascii=False)[:400])
