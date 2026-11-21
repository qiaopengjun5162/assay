#!/usr/bin/env bash
# 发布终检：把一个 tag 的全部资产下载回来验签、解压、并跑本机二进制。
#
# RELEASE_CHECKLIST 第 4 节要求「必须是 10 个资产」，但旧命令只验 macos-arm64 一个，
# 其余靠 workflow 自检那句话带过。这道脚本补上维护者视角的完整终检：
# 5 平台 × (包 + .sha256) 一个不漏，且验的是 GitHub 上用户真能下载到的字节。
#
# 用法：bash scripts/verify-release.sh [tag]    省略 tag 则取 latest
# 需要 gh CLI 已登录；国内网络若下载失败，先 export HTTPS_PROXY=http://127.0.0.1:7890
# 退出码：0 = 10 资产全对；非 0 = 有缺失/哈希不符/二进制跑不动。
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

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
cd "$TMP"

echo "发布终检：$TAG"
echo "── ① 资产数量与状态 ─────────────────────"
META="$(gh api "repos/$REPO/releases/tags/$TAG")"
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
# 10 个并发 curl 在本机代理下会撞 HTTP/2 framing 错误、静默丢文件；
# 串行 + 强制 HTTP/1.1 + 重试三次，慢几秒但结果可信。
download() {
  local url="$1" dest="$2" i
  for i in 1 2 3; do
    if curl -fsSL --http1.1 -o "$dest" "$url"; then
      [[ -s "$dest" ]] && return 0
    fi
    rm -f "$dest"
  done
  return 1
}
fail=0
for a in "${PLATFORMS[@]}"; do
  ext=tar.gz
  [[ "$a" == windows-* ]] && ext=zip
  base="assay-$a.$ext"
  for f in "$base" "$base.sha256"; do
    if ! download "https://github.com/$REPO/releases/download/$TAG/$f" "$f"; then
      printf '  FAIL %-42s 下载失败（重试 3 次仍无文件）\n' "$f"
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

echo "──────────────────────────────────────────"
if [[ "$fail" -ne 0 ]]; then
  echo "  $fail 项不通过，别把这个 release 发出去。" >&2
  exit 1
fi
echo "  10 资产齐、哈希全对、包内二进制可用。"
