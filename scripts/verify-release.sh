#!/usr/bin/env bash
# 发布终检：把一个 tag 的全部资产下载回来验签、解压、并跑本机二进制。
#
# RELEASE_CHECKLIST 第 4 节要求「必须是 10 个资产」，但旧命令只验 macos-arm64 一个，
# 其余靠 workflow 自检那句话带过。这道脚本补上维护者视角的完整终检：
# 5 平台 × (包 + .sha256) 一个不漏，且验的是 GitHub 上用户真能下载到的字节。
#
# 用法：bash scripts/verify-release.sh [tag]    省略 tag 则取 latest
# 需要 gh CLI 已登录；国内网络若下载失败，先 export HTTPS_PROXY=http://127.0.0.1:7890
# 退出码：0 = 10 资产全对且包内二进制跑得过门禁；非 0 = 有缺失/哈希不符/二进制跑不动。
# 自 0.2.6 起多一道 ⑤：用包内二进制跑 --source news:，验「这一版新增的功能」没坏。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="qiaopengjun5162/assay"
PLATFORMS=(linux-amd64 linux-arm64 macos-amd64 macos-arm64 windows-amd64)
EXPECTED=$(( ${#PLATFORMS[@]} * 2 ))

TAG="${1:-$(gh release view --json tagName --jq .tagName)}"
if [[ -z "$TAG" ]]; then
  echo "拿不到 tag，检查 gh 登录状态" >&2
  exit 2
fi

# gh api 也会抖（实测返回 EOF），和下面的下载一样要重试；否则脚本在
# 网络最不稳的那一刻直接退出，反而让人以为 release 有问题。
META=""
for i in 1 2 3; do
  if META="$(gh api "repos/$REPO/releases/tags/$TAG" 2>/dev/null)" && [[ -n "$META" ]]; then
    break
  fi
  META=""
  sleep 2
done
if [[ -z "$META" ]]; then
  echo "读不到 release 元数据（重试 3 次仍失败），检查网络与 gh 登录" >&2
  exit 2
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
cd "$TMP"

echo "发布终检：$TAG"
echo "── ① 资产数量与状态 ─────────────────────"
READY="$(printf '%s' "$META" | jq -r '.draft == false and .prerelease == false')"
N="$(printf '%s' "$META" | jq -r '.assets | length')"
printf '  draft/prerelease 已发布: %s\n' "$READY"
printf '  资产数: %s（期望 %s）\n' "$N" "$EXPECTED"
if [[ "$READY" != "true" || "$N" -ne "$EXPECTED" ]]; then
  echo "资产不齐或还是草稿，终止。" >&2
  exit 1
fi
for a in "${PLATFORMS[@]}"; do
  ext=tar.gz
  [[ "$a" == windows-* ]] && ext=zip
  printf '%s' "$META" | jq -e --arg n "assay-$a.$ext" '.assets | any(.name == $n)' >/dev/null \
    || { echo "  缺 assay-$a.$ext" >&2; exit 1; }
  printf '%s' "$META" | jq -e --arg n "assay-$a.$ext.sha256" '.assets | any(.name == $n)' >/dev/null \
    || { echo "  缺 assay-$a.$ext.sha256" >&2; exit 1; }
done
echo "  10 个都在。"

echo "── ② 下载 + 逐份验签 ────────────────────"
# 下载有两条通道，**curl CDN 优先、gh api 兜底**：
#   github.com/<repo>/releases/download/... 走 release CDN，快（26 秒/包）但某些代理节点
#     会间歇返回 502；实测多数包能下全，重试几次即可。
#   api.github.com 的 asset 通道绕开 CDN 的 502，但下大文件(~1.6M)会静默截断，
#     必须靠下面的 size 校验识别，且套 timeout 防止卡死。
# 两条通道都做「字节数 == 元数据 size」校验，任何截断/半包都会被判失败并重试。
# 串行 + 重试三次。**别给 curl 加 --http1.1**：实测 HTTP/2 26 秒下完、HTTP/1.1 60 秒才一半。
asset_field() {
  printf '%s' "$META" | jq -r --arg n "$1" --arg f "$2" '.assets[] | select(.name == $n)[$f]'
}
# 完整性校验必须按元数据里的 size 比字节数：gh api 下大文件（~1.6M）实测会
# 中途静默截断成 893K 却仍返回 0，单看「文件非空」会把它当成成功，后面才报哈希不符。
complete() {
  local dest="$1" want="$2" got
  [[ -s "$dest" ]] || return 1
  got="$(wc -c < "$dest" | tr -d ' ')"
  [[ "$got" == "$want" ]]
}
download() {
  local name="$1" dest="$2" id url want i
  id="$(asset_field "$name" id)"
  want="$(asset_field "$name" size)"
  # 通道一：curl release CDN（快，但代理可能间歇 502）
  url="https://github.com/$REPO/releases/download/$TAG/$name"
  for i in 1 2 3; do
    curl -fsSL --max-time 60 -o "$dest" "$url" 2>/dev/null
    if complete "$dest" "$want"; then
      return 0
    fi
    rm -f "$dest"
  done
  # 通道二：gh api asset（绕开 CDN 的 502；下大文件会静默截断，靠 size 校验拦，
  # 且套 90 秒硬超时防止像实测那样卡在 893K 耗 8 分钟）
  if [[ -n "$id" && "$id" != "null" ]]; then
    for i in 1 2 3; do
      timeout 90 gh api -H "Accept: application/octet-stream" "repos/$REPO/releases/assets/$id" \
        >"$dest" 2>/dev/null
      if complete "$dest" "$want"; then
        return 0
      fi
      rm -f "$dest"
      sleep 2
    done
  fi
  return 1
}
fail=0
for a in "${PLATFORMS[@]}"; do
  ext=tar.gz
  [[ "$a" == windows-* ]] && ext=zip
  base="assay-$a.$ext"
  for f in "$base" "$base.sha256"; do
    if ! download "$f" "$f"; then
      printf '  FAIL %-42s 下载失败/字节数不符（两通道各重试 3 次）\n' "$f"
      fail=$((fail + 1))
    fi
  done
done
for a in "${PLATFORMS[@]}"; do
  ext=tar.gz
  [[ "$a" == windows-* ]] && ext=zip
  f="assay-$a.$ext.sha256"
  [[ -s "$f" ]] || { printf '  FAIL %-42s 文件缺失\n' "$f"; fail=$((fail + 1)); continue; }
  # Windows 曾因 PowerShell Out-File 写出 CRLF 让 shasum -c 报 FAILED open or read；
  # 单独查一次行尾，免得哈希对了却栽在不可见字符上。
  if python3 -c "import sys;sys.exit(b'\r' in open(sys.argv[1],'rb').read())" "$f"; then
    printf '  OK   %-42s LF 行尾\n' "$f"
  else
    printf '  FAIL %-42s CRLF 行尾（shasum -c 会误报）\n' "$f"
    fail=$((fail + 1))
  fi
  if shasum -a 256 -c "$f" >/dev/null 2>&1; then
    printf '  OK   %-42s 哈希一致\n' "$f"
  else
    printf '  FAIL %-42s 哈希不符\n' "$f"
    fail=$((fail + 1))
  fi
done

echo "── ③ 解压：每个包应有且只有一个二进制 ────"
for a in "${PLATFORMS[@]}"; do
  ext=tar.gz
  bin=assay
  [[ "$a" == windows-* ]] && { ext=zip; bin=assay.exe; }
  if [[ ! -s "assay-$a.$ext" ]]; then
    printf '  FAIL %-16s 包未下载，跳过解压\n' "$a"
    fail=$((fail + 1))
    continue
  fi
  mkdir -p "$a"
  if [[ "$ext" == zip ]]; then
    python3 -c "import zipfile,sys;zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" "assay-$a.zip" "$a"
  else
    tar xzf "assay-$a.tar.gz" -C "$a"
  fi
  # .exe 是 PE 文件，在非 Windows 主机上没有 Unix 执行位；只要求存在且非空。
  if [[ -s "$a/$bin" ]] && { [[ "$a" == windows-* ]] || [[ -x "$a/$bin" ]]; }; then
    printf '  OK   %-16s %s (%s 字节)\n' "$a" "$bin" "$(wc -c < "$a/$bin" | tr -d ' ')"
  else
    printf '  FAIL %-16s 包内没有可执行的 %s\n' "$a" "$bin"
    fail=$((fail + 1))
  fi
done

echo "── ④ 本机二进制实跑复核 ──────────────────"
os="$(uname -s)"
arch="$(uname -m)"
case "$os:$arch" in
  Darwin:arm64)   local_platform=macos-arm64 ;;
  Darwin:x86_64)  local_platform=macos-amd64 ;;
  Linux:x86_64)   local_platform=linux-amd64 ;;
  Linux:aarch64)  local_platform=linux-arm64 ;;
  *)              local_platform="" ;;
esac
if [[ -z "$local_platform" ]]; then
  echo "  跳过（$os/$arch 不在自动实跑清单里，手动跑包内二进制）"
else
  BIN="$TMP/$local_platform/assay"
  ver="$("$BIN" --version 2>&1)"
  printf '  %s: %s\n' "$local_platform" "$ver"
  expected_ver="${TAG#v}"
  [[ "$ver" == *"$expected_ver"* ]] || { echo "  版本号与 tag 不符（期望 $expected_ver）"; fail=$((fail + 1)); }
  for gate in verify-evidence verify-reports; do
    if [[ "$gate" == verify-evidence ]]; then
      out="$("$BIN" "$gate" "$ROOT/web/data.json" "$ROOT/web/evidence" 2>&1)"
    else
      out="$("$BIN" "$gate" "$ROOT/web/data.json" "$ROOT/web/reports" "$ROOT/web/evidence" 2>&1)"
    fi
    echo "  $gate: $(printf '%s' "$out" | tail -1)"
    printf '%s' "$out" | grep -q "500 / 500 份一致" || { echo "  门禁没过"; fail=$((fail + 1)); }
  done
fi

echo "── ⑤ 新功能实跑：--source news: ──────────"
# ④ 只验旧路径（浙江 500 份证据 / 报告），对「这一版新增了什么」是瞎的：#75 的
# news: 适配器自 0.2.6 才进包，用户下完最想跑的正是它。拿仓库里的
# fixtures/qunmind-items.json（上游原样数组：snake_case、无 id、Option 字段，4 条）
# 跑一遍 audit + checks，验三件事：
#   1. 认出来源 —— run.json.platform 含 qunmind（不强绑全串，见 #75 的教训）
#   2. 派生 id 没丢条 —— catalogTotal 与 items.jsonl 行数一致
#   3. 尺子没歪 —— 一致性三项 0 命中（#77 之前这里会 4/4 全中，是拿 Option 当承诺）
if [[ -n "$local_platform" ]]; then
  ver_ok="$(python3 - "${expected_ver:-}" <<'PY'
import sys
def parts(v):
    return tuple(int(x) for x in v.split("-")[0].split("."))
try:
    ok = parts(sys.argv[1]) >= (0, 2, 6)
except (ValueError, IndexError):
    ok = False
print("yes" if ok else "no")
PY
)"
  if [[ "$ver_ok" != "yes" ]]; then
    printf '  跳过（%s < 0.2.6，news: 适配器尚未入包）\n' "${expected_ver:-版本未知}"
  else
    NEWS_OUT="$TMP/news-smoke"
    mkdir -p "$NEWS_OUT"
    if "$BIN" --out "$NEWS_OUT" --source "news:$ROOT/fixtures/qunmind-items.json" \
        audit --threads 2 >"$NEWS_OUT/audit.log" 2>&1; then
      "$BIN" checks --items "$NEWS_OUT/items.jsonl" \
        --catalog "$NEWS_OUT/catalog_snapshot.json" >"$NEWS_OUT/checks.txt" 2>&1 || true
      if python3 - "$NEWS_OUT" <<'PY'
import json, os, sys
out = sys.argv[1]
run = json.load(open(os.path.join(out, "run.json"), encoding="utf-8"))
items = [l for l in open(os.path.join(out, "items.jsonl"), encoding="utf-8") if l.strip()]
ok = True
def check(cond, msg):
    global ok
    print(("  OK   " if cond else "  FAIL ") + msg)
    ok = ok and bool(cond)
plat = run.get("platform") or ""
check("qunmind" in plat, f"来源识别：platform={plat!r}")
check(run.get("catalogTotal") == 4 and len(items) == 4,
      f"条数：catalogTotal={run.get('catalogTotal')} items.jsonl={len(items)}")
hits = {}
for line in open(os.path.join(out, "checks.txt"), encoding="utf-8"):
    f = line.split()
    if len(f) >= 3 and "/" in f[2]:
        hits[f[0]] = f[2]
want = f"0/{len(items)}"
bad = {k: hits.get(k) for k in ("schema-count", "schema-field-set", "schema-naming-only")
       if hits.get(k) != want}
check(not bad, "一致性三项 0 命中" if not bad else f"一致性三项有命中：{bad}")
sys.exit(0 if ok else 1)
PY
      then
        :
      else
        echo "  新功能实跑没过（--source news:）"
        fail=$((fail + 1))
      fi
    else
      printf '  FAIL %-16s 跑不起来（--source news:）\n' "news:"
      sed -n '1,5p' "$NEWS_OUT/audit.log" | sed 's/^/       /'
      fail=$((fail + 1))
    fi
  fi
fi

echo "──────────────────────────────────────────"
if [[ "$fail" -ne 0 ]]; then
  echo "  $fail 项不通过，别把这个 release 发出去。" >&2
  exit 1
fi
echo "  10 资产齐、哈希全对、包内二进制可用。"
