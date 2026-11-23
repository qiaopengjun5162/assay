#!/usr/bin/env bash
# verify-live 的负向自测：证明「线上终检」这道门禁真的会红，不是永远盖章的橡皮图章。
#
# gate-negative.sh 证明了本地产物门禁有约束力，但 verify-live 是新加的一层，
# 它验的是「线上活站」。若 it 永远返回 0，那句「线上可自证」就是纸糊的。
# 本脚本在临时目录搭一个假站点，用本地 HTTP 服务喂给 verify-live.sh，
# 先确认真站点能通过，再逐项破坏，确认每处篡改都让它退出非 0。
#
# 不碰 web/ 冻结产物、不访问外网、不需要 gh/Rust。
# 退出码：0 = 每处破坏都被抓住；非 0 = 有破坏没触发门禁（形同虚设）。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE="$ROOT/scripts/verify-live.sh"
DATA="$ROOT/web/data.json"

need() { command -v "$1" >/dev/null 2>&1 || { echo "缺少 $1" >&2; exit 2; }; }
need python3
need curl

TMP="$(mktemp -d)"
SITE="$TMP/site"
PID=""
cleanup() {
  [[ -n "$PID" ]] && kill "$PID" 2>/dev/null
  wait "$PID" 2>/dev/null
  rm -rf "$TMP"
}
trap cleanup EXIT

# 随机独立端口，避免撞车（api-e2e 踩过端口复用的坑）。
PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"
BASE="http://127.0.0.1:$PORT"
LOG="$TMP/server.log"

pass=0
fail=0
row() { printf '  %-26s %-18s %s\n' "$1" "$2" "$3"; }
ok() { row "$1" "$2" "PASS"; pass=$((pass + 1)); }
bad() {
  row "$1" "$2" "FAIL"
  sed -n '1,8p' "$TMP/$1.log" 2>/dev/null | sed 's/^/      /'
  fail=$((fail + 1))
}

# run_gate 名字 期望退出码 输出须含(-- 表示不检查)
run_gate() {
  local name="$1" want="$2" needle="$3" rc
  bash "$GATE" "$BASE" >"$TMP/$name.log" 2>&1
  rc=$?
  # 期望 0 时要求「线上部署最新」收尾语；期望非 0 时要求有 FAIL 或明确的不通过提示。
  if [[ "$want" -eq 0 ]]; then
    if [[ "$rc" -eq 0 ]] && { [[ "$needle" == "--" ]] || grep -qF -- "$needle" "$TMP/$name.log"; }; then
      ok "$name" "rc=0"
    else
      bad "$name" "rc=$rc, 期望 0"
    fi
  else
    if [[ "$rc" -ne 0 ]] && grep -qE "FAIL|不通过" "$TMP/$name.log"; then
      ok "$name" "rc=$rc(非0)"
    else
      bad "$name" "rc=$rc, 期望非0且有 FAIL"
    fi
  fi
}

# 搭一个「真」站点：data.json 必须是仓库原版（①要比对线上==仓库），
# 再从 data.json 读出抽样记录的证据/报告路径，把真实字节放进去。
build_site() {
  # 目录本身保留（http.server 的 CWD 指向它），只清内容——删掉再建会让
  # 运行中的服务仍盯着旧 inode，后续 build_site 的改动它看不见。
  mkdir -p "$SITE"
  find "$SITE" -mindepth 1 -delete >/dev/null 2>&1
  cp "$DATA" "$SITE/data.json"
  printf 'fake favicon' > "$SITE/favicon.svg"
  printf 'fake og' > "$SITE/og.png"
  python3 - "$DATA" "$SITE" <<'PY'
import json, os, sys
data_path, site = sys.argv[1], sys.argv[2]
d = json.load(open(data_path, encoding="utf-8"))
ev = d["datasets"][0]["evidence"]
for f in ev["files"]:                       # evidence/<safe>/detail.json
    dst = os.path.join(site, f)
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    # 内容随意：verify-live 只重算哈希与 data.json 里的 sha256 比对。
    # 但要与 data.json 记录的哈希一致，才过正向——所以从仓库真证据复制。
    src = os.path.join(os.path.dirname(data_path), f)
    with open(src, "rb") as s, open(dst, "wb") as t:
        t.write(s.read())
safe = ev["files"][0].split("/")[1]
os.makedirs(os.path.join(site, "reports"), exist_ok=True)
with open(os.path.join(site, "reports", safe + ".md"), "w", encoding="utf-8") as t:
    t.write("fake report")
PY
}

echo "verify-live 负向自测（临时站点 $BASE）"

# 先建好站点目录，否则 http.server 的 cd 会失败（且日志重定向因 && 短路不生效）。
build_site

# 起本地静态服务：用 python 的 SimpleHTTPRequestHandler，根目录=SITE。
( cd "$SITE" && python3 -m http.server "$PORT" --bind 127.0.0.1 >"$LOG" 2>&1 ) &
PID=$!
# 等服务起来（最多 5 秒）。
for i in $(seq 1 25); do
  [[ "$(curl -s -o /dev/null -w '%{http_code}' --max-time 1 "$BASE/data.json" 2>/dev/null)" == "200" ]] && break
  sleep 0.2
done
if [[ "$(curl -s -o /dev/null -w '%{http_code}' --max-time 2 "$BASE/data.json" 2>/dev/null)" != "200" ]]; then
  echo "本地测试服务没起来，看 $LOG" >&2
  sed -n '1,10p' "$LOG" >&2
  exit 2
fi

echo "── 正向基线 ──────────────────────────────"
run_gate baseline 0 "线上部署最新"

echo "── 故意破坏：必须变红 ────────────────────"

# ① 线上 data.json 被改一个字节（部署被改过 / 过期）
build_site
python3 - "$SITE/data.json" <<'PY'
import sys
p = sys.argv[1]
b = open(p, "rb").read()
open(p, "wb").write(b.replace(b'"total"', b'"totalX"', 1) if b'"total"' in b else b + b" ")
PY
run_gate data-tampered 1 "FAIL"

# ② 结构破坏：榜单砍到 1 条（同时会触发①的 SHA 不一致，属预期）
build_site
python3 - "$SITE/data.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p, encoding="utf-8"))
d["datasets"] = d["datasets"][:1]
json.dump(d, open(p, "w", encoding="utf-8"))
PY
run_gate structure 1 "FAIL"

# ③ 抽样的证据文件改一个字节（哈希必不符）
build_site
VICTIM="$(find "$SITE/evidence" -type f | head -1)"
printf 'X' | dd of="$VICTIM" bs=1 seek=0 conv=notrunc status=none
run_gate evidence-byte 1 "FAIL"

# ④ 证据文件整份消失（下载失败）
build_site
VICTIM="$(find "$SITE/evidence" -type f | head -1)"
rm -f "$VICTIM"
run_gate evidence-missing 1 "FAIL"

# ⑤ 报告 404（static resource 假链）
build_site
rm -f "$SITE"/reports/*.md
run_gate report-404 1 "FAIL"

echo "── 退出码语义 ────────────────────────────"
# ⑥ 站点不可达（重试 3 次后取不到 data.json）→ exit 2，与「校验失败 exit 1」区分。
kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null; PID=""
# 换一个没人监听的端口。
DEAD_PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"
bash "$GATE" "http://127.0.0.1:$DEAD_PORT" >"$TMP/unreachable.log" 2>&1
rc=$?
if [[ "$rc" -eq 2 ]] && grep -qF "取不到" "$TMP/unreachable.log"; then
  ok "unreachable" "rc=2"
else
  bad "unreachable" "rc=$rc, 期望 2"
fi

echo "──────────────────────────────────────────"
echo "  $pass 通过 / $fail 失败"
if [[ "$fail" -ne 0 ]]; then
  echo "有破坏没被线上终检抓住，别信它。" >&2
  exit 1
fi
echo "线上终检对每处破坏都真的会红。"
