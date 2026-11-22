#!/usr/bin/env bash
# 查询后端真库端到端：起独立 Postgres → 灌库 → 起服务 → 验端点 → 清理。
#
# 为什么需要：crates/api 的 14 个测试全是纯函数（参数归一化、错误映射），
# handler 走真 SQL 的部分从没被验证过。这套脚本补上「只读服务接真库能用」这一半。
#
# 安全：起的是临时实例（独立 data 目录 + 随机高位端口），不碰本机已有的 5432；
# 退出时无论成功失败都会停服务、停库、删临时目录。
#
# 用法：bash scripts/api-e2e.sh
#   PGBIN=/path/to/pg/bin bash scripts/api-e2e.sh   # 覆盖 postgres 工具路径
# 退出码：0 = 全部端点符合预期；非 0 = 有断言失败或缺依赖。
set -uo pipefail

# tracing 的日志带 ANSI 颜色码会把 `datasets=24322` 拆成 `datasets\x1b[3m=\x1b[0m...`，
# 让字符串断言失败——关掉颜色，断言才可预期。
export NO_COLOR=1

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/assay-api"
OUT="$ROOT/out-full"

# 找 postgres 工具：优先 PGBIN，其次 PATH，最后常见 Homebrew 路径。
if [[ -n "${PGBIN:-}" ]]; then
  :
elif command -v pg_ctl >/dev/null 2>&1; then
  PGBIN="$(dirname "$(command -v pg_ctl)")"
else
  for p in /opt/homebrew/opt/postgresql@17/bin /usr/local/opt/postgresql@17/bin /usr/lib/postgresql/*/bin; do
    [[ -x "$p/pg_ctl" ]] && { PGBIN="$p"; break; }
  done
fi

need() { command -v "$1" >/dev/null 2>&1 || { echo "缺少命令 $1" >&2; exit 2; }; }
need python3
need curl
if [[ ! -x "$BIN" ]]; then
  echo "缺少 $BIN，先跑：cargo build --release -p assay-api" >&2
  exit 2
fi
for f in "$OUT/items.jsonl" "$OUT/summary.json"; do
  [[ -f "$f" ]] || { echo "缺少灌库输入 $f" >&2; exit 2; }
done
for t in pg_ctl initdb createdb psql pg_isready; do
  [[ -x "$PGBIN/$t" ]] || { echo "缺少 postgres 工具 $PGBIN/$t，用 PGBIN= 指定" >&2; exit 2; }
done

# 数据库和服务各占一个独立随机端口，不能共用——共用会让服务绑定失败，
# 表现为 /health 一直空、后面所有端点断言连锁失败（排查时先查 svc.log）。
PORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"
APIPORT="$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')"
DBURL="postgres://assay@127.0.0.1:$PORT/assay"
TMP="$(mktemp -d)"
SVC_PID=""

cleanup() {
  [[ -n "$SVC_PID" ]] && kill "$SVC_PID" 2>/dev/null
  "$PGBIN/pg_ctl" -D "$TMP/data" stop -m immediate >/dev/null 2>&1
  rm -rf "$TMP"
}
trap cleanup EXIT

pass=0
fail=0
row() { printf '  %-28s %s\n' "$1" "$2"; }
ok() { row "$1" "PASS"; pass=$((pass + 1)); }
bad() { row "$1" "FAIL"; fail=$((fail + 1)); }
check_eq() { # 名字 期望 实际
  if [[ "$2" == "$3" ]]; then ok "$1"; else bad "$1 (期望=$2 实际=$3)"; fi
}

echo "── 起独立 Postgres（端口 $PORT，不动 5432）─────────"
"$PGBIN/initdb" -D "$TMP/data" -U assay --auth=trust >"$TMP/initdb.log" 2>&1 || {
  echo "initdb 失败" >&2; tail -10 "$TMP/initdb.log" >&2; exit 2; }
"$PGBIN/pg_ctl" -D "$TMP/data" -o "-p $PORT -h 127.0.0.1" -l "$TMP/pg.log" start >/dev/null 2>&1 || {
  echo "pg_ctl 起库失败" >&2; tail -10 "$TMP/pg.log" >&2; exit 2; }
"$PGBIN/createdb" -h 127.0.0.1 -p "$PORT" -U assay assay || exit 2

echo "── ① 灌库（幂等）───────────────────────────────"
OUT1="$(DATABASE_URL="$DBURL" "$BIN" ingest --out "$OUT" --captured-at 2026-11-18 --commit e2e-test 2>&1)"
RC1=$?
OUT2="$(DATABASE_URL="$DBURL" "$BIN" ingest --out "$OUT" --captured-at 2026-11-18 --commit e2e-test 2>&1)"
RC2=$?
if [[ $RC1 -eq 0 && "$OUT1" == *"datasets=24322"* ]]; then ok "灌入 24322 条"; else bad "灌入条数(rc=$RC1): $OUT1"; fi
if [[ $RC2 -eq 0 && "$OUT2" == *"run_id=1"* && "$OUT2" == *"datasets=24322"* ]]; then
  ok "同日重灌幂等（仍 run #1）"
else
  bad "重灌不幂等(rc=$RC2): $OUT2"
fi

echo "── ② 起服务（端口 $APIPORT）────────────────────"
DATABASE_URL="$DBURL" PORT="$APIPORT" RUST_LOG=warn "$BIN" serve >"$TMP/svc.log" 2>&1 &
SVC_PID=$!
B="http://127.0.0.1:$APIPORT"
for _ in $(seq 1 40); do
  [[ "$(curl -s -o /dev/null -w '%{http_code}' "$B/health" 2>/dev/null)" == "200" ]] && break
  sleep 0.25
done
HEALTH="$(curl -s "$B/health" 2>/dev/null)"
if [[ "$HEALTH" == *'"ok":true'* ]]; then
  ok "/health 200 ok"
else
  bad "/health: ${HEALTH:-空}（服务日志：$(tail -3 "$TMP/svc.log" | tr '\n' ' ')）"
fi

echo "── ③ 端点断言 ──────────────────────────────────"
check_eq "/v1/runs 条数" "1" "$(curl -s "$B/v1/runs" | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))')"
check_eq "/v1/runs 体检数" "24322" "$(curl -s "$B/v1/runs" | python3 -c 'import sys,json;print(json.load(sys.stdin)[0]["audited"])')"

# 票务是讲稿点名的那条：7→92 列、35 天、9.0 分。
TICKET="$(curl -s "$B/v1/datasets?run=1&q=%E7%A5%A8%E5%8A%A1&size=5" | python3 -c '
import sys,json
rows=json.load(sys.stdin)["items"]
r=next((x for x in rows if x["name"]=="票务线上交易明细信息"),None)
if r:
    print("|".join(str(r[k]) for k in ("declaredCols","sampleCols","lagDays","score")))
else:
    print("MISSING")')"
check_eq "票务行与静态榜一致" "7|92|35|9.0" "$TICKET"

check_eq "mismatch 过滤 = 全量口径" "1284" "$(curl -s "$B/v1/datasets?run=1&mismatch=true&size=1" | python3 -c 'import sys,json;print(json.load(sys.stdin)["total"])')"
check_eq "minEmpty=0.8 比率语义" "441" "$(curl -s "$B/v1/datasets?run=1&minEmpty=0.8&size=1" | python3 -c 'import sys,json;print(json.load(sys.stdin)["total"])')"

HIST="$(curl -s "$B/v1/datasets/ati4nLKVPhIJLxa3CeV5Ng%3D%3D/history")"
[[ "$HIST" == *'2026-11-18'* ]] && ok "history 返回该数据集" || bad "history: $HIST"

NOTFOUND="$(curl -s -w '|%{http_code}' "$B/v1/datasets?run=999&size=1")"
if [[ "$NOTFOUND" == *'没有 id=999'* && "$NOTFOUND" == *'404'* ]]; then
  ok "不存在 run → 404 带原因"
else
  bad "404: $NOTFOUND"
fi

EVIL="$(curl -s -o /dev/null -w '%{http_code}' "$B/v1/datasets?run=1&sort=evil%27%3BDROP")"
check_eq "非法 sort → 400（白名单）" "400" "$EVIL"

echo "── ④ 最新 run 按日期而非 max(id) ───────────────"
DATABASE_URL="$DBURL" "$BIN" ingest --out "$OUT" --captured-at 2026-11-19 >/dev/null 2>&1
LATEST="$(curl -s "$B/v1/datasets?size=1" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d["capturedAt"])')"
check_eq "默认取较新的 captured_at" "2026-11-19" "$LATEST"

echo
echo "结果：$pass PASS / $fail FAIL"
[[ $fail -eq 0 ]]
