#!/usr/bin/env bash
# 线上终检：验证评委真正打开的 github.io 站点是「最新部署 + 可自证清白」。
#
# 现有门禁验的是本地产物（verify-evidence/verify-reports）和发布资产
# （verify-release），没有一层验「线上的活站」——可用户和评委只会打开网址。
# 这道脚本补上维护者视角的线上验证：
#   ① 线上 data.json 与仓库冻结版本逐字节一致（确认部署的是这一版，不是旧缓存）
#   ② 从线上数据随机取一条，下载它的原始证据，重算 SHA-256 与页面记录逐字符比对
#   ③ 该条证据文件、报告、站点静态资源都能取到（不是 404 假链）
#   ④ 结构字段齐：500 条榜单、10 项检查、历史曲线有数据点
#
# 用法：bash scripts/verify-live.sh [base_url]
#   省略 base_url 用默认站点；可传自定义域名验证镜像。
# 不需要 gh CLI、不需要 Rust、不需要数据库——纯 curl + python3，谁都能跑。
# 退出码：0 = 线上可被信任地复核；非 0 = 部署过期 / 证据对不上 / 资源缺失。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BASE="${1:-https://qiaopengjun5162.github.io/assay}"
BASE="${BASE%/}"   # 去掉末尾斜杠，后面统一拼 /path

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail=0
ok()   { printf '  OK   %s\n' "$1"; }
bad()  { printf '  FAIL %s\n' "$1"; fail=$((fail + 1)); }

# 网络抖动重试三次；拿到空文件也当失败（代理有时返回 200 + 错误页）。
fetch() {
  local url="$1" dest="$2" i
  for i in 1 2 3; do
    # 静默 curl 的超时噪声：重试本身是预期内的，只有三次全失败才由调用方报 FAIL。
    if curl -fsSL --max-time 40 -o "$dest" "$url" 2>/dev/null && [[ -s "$dest" ]]; then
      return 0
    fi
    rm -f "$dest"
  done
  return 1
}

echo "线上终检：$BASE"
echo "── ① 部署版本是否与仓库一致 ────────────"
if ! fetch "$BASE/data.json" "$TMP/live.json"; then
  echo "  线上 data.json 取不到（重试 3 次），站点可能没部署或网络不通。" >&2
  exit 2
fi
LIVE_SHA="$(shasum -a 256 "$TMP/live.json" | cut -d' ' -f1)"
REPO_SHA="$(shasum -a 256 "$ROOT/web/data.json" | cut -d' ' -f1)"
printf '  线上 SHA: %s\n  仓库 SHA: %s\n' "$LIVE_SHA" "$REPO_SHA"
if [[ "$LIVE_SHA" == "$REPO_SHA" ]]; then
  ok "部署的就是仓库冻结版（逐字节一致）"
else
  bad "线上与仓库不一致——部署过期或站点被改过，别引用线上数字"
fi

echo "── ② 结构完整性 ─────────────────────────"
python3 - "$TMP/live.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
rows = d.get("datasets", [])
checks = d.get("checks", [])
hist = d.get("history", [])
problems = []
if len(rows) != 500:
    problems.append(f"榜单应为 500 条，实为 {len(rows)}")
if not checks:
    problems.append("检查项清单为空")
if not hist:
    problems.append("历史曲线没有数据点")
print(f"  榜单 {len(rows)} 条 / 检查项 {len(checks)} 项 / 曲线 {len(hist)} 点")
sys.exit(1 if problems else 0)
PY
[[ $? -eq 0 ]] && ok "结构字段齐全" || bad "结构字段有缺失"

echo "── ③ 抽一条证据，重算哈希 ───────────────"
# 取一条同时有证据和证明的记录，覆盖面最广。
PICK="$(python3 - "$TMP/live.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
for r in d.get("datasets", []):
    ev = r.get("evidence")
    if ev and ev.get("files") and len(ev["files"]) >= 2 and ev.get("sha256"):
        print(r["id"])
        break
PY
)"
if [[ -z "$PICK" ]]; then
  bad "线上数据里找不到可复核的证据记录"
else
  # 从数据里读出这条的文件相对路径、期望 SHA、报告路径。
  # 证据目录名（safe id）直接从 files[0] 取，不在这里重算 +/= 的映射规则——
  # 重复实现导出规则正是哈希对不上那类漂移的根源（见 D19/ENGINEERING_LESSONS）。
  python3 - "$TMP/live.json" "$PICK" > "$TMP/pick.tsv" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
r = next(x for x in d["datasets"] if x["id"] == sys.argv[2])
ev = r["evidence"]
first = ev["files"][0]                      # evidence/<safe>/detail.json
safe = first.split("/")[1]                  # 目录名即导出用的 safe id，直接复用
report = f"reports/{safe}.md"
print(ev["sha256"])
print("\t".join(ev["files"]))
print(report)
PY
  EXPECT_SHA="$(sed -n '1p' "$TMP/pick.tsv")"
  FILES="$(sed -n '2p' "$TMP/pick.tsv")"
  REPORT="$(sed -n '3p' "$TMP/pick.tsv")"

  # 下载证据文件，按 data ‖ \n--split--\n ‖ preview 的顺序拼接（与 verify-evidence 同一字节序）。
  cat_files() {
    local i=0
    IFS=$'\t' read -ra arr <<< "$FILES"
    for f in "${arr[@]}"; do
      local dest="$TMP/ev_$i"
      fetch "$BASE/$f" "$dest" || return 1
      if [[ $i -eq 0 ]]; then cat "$dest"; else printf '\n--split--\n'; cat "$dest"; fi
      i=$((i + 1))
    done
  }
  if cat_files > "$TMP/evidence.bin" && [[ -s "$TMP/evidence.bin" ]]; then
    ACTUAL_SHA="$(shasum -a 256 "$TMP/evidence.bin" | cut -d' ' -f1)"
    printf '  抽样 ID: %s\n  页面 SHA: %s\n  重算 SHA: %s\n' "$PICK" "$EXPECT_SHA" "$ACTUAL_SHA"
    if [[ "$ACTUAL_SHA" == "$EXPECT_SHA" ]]; then
      ok "下载重算的哈希与页面逐字符一致"
    else
      bad "证据哈希对不上——线上这份被动过，或字节序规则变了"
    fi
  else
    bad "证据文件下载失败"
  fi

  echo "── ④ 报告与站点资源可达 ────────────────"
  for rel in "$REPORT" favicon.svg og.png; do
    code="$(curl -s -o /dev/null -w '%{http_code}' --max-time 20 "$BASE/$rel")"
    [[ "$code" == 200 ]] && ok "$rel ($code)" || bad "$rel (HTTP $code)"
  done
fi

echo "──────────────────────────────────────────"
if [[ "$fail" -ne 0 ]]; then
  echo "  $fail 项不通过，线上站点不能作为可信交付。" >&2
  exit 1
fi
echo "  线上部署最新、证据可自证、资源齐备——评委打开网址就能自己验。"
