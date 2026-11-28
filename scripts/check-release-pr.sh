#!/usr/bin/env bash
# 核对一个 release-please 发版 PR 是否「可以合并」。
#
# 为什么需要它：发版 PR 的 ci 天然跑不起来——release-please 用 GITHUB_TOKEN 建 PR，
# GitHub 对这类 PR 的 workflow 要求人工批准；没批准就合并，run 会在分支被删时取消，
# 显示成 failure 且**没有任何 job**（看着吓人，其实什么都没跑）。所以「这版是真的吗、
# 版本号对得上吗、有没有顺手带进代码」长期靠人眼看 diff。
#
# 而人眼判据反复出错：#71 只增 1 行 CHANGELOG，按「新增行数少=真发版」判会当成真的，
# 实际那条 #56 早已在 v0.2.4 发布过——本质判据是「内容是否重复」，不是行数。
# 这个脚本把 docs/RELEASE_CHECKLIST.md 第 2 节的人肉步骤变成可执行断言。
#
# 用法：./scripts/check-release-pr.sh <pr-number>
# 退出码：0 = 通过（可以合并）；1 = 有阻塞问题；2 = 参数或环境问题
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PR="${1:-}"
if [[ -z "$PR" ]]; then
  echo "用法：$0 <pr-number>" >&2
  exit 2
fi
if ! command -v gh >/dev/null 2>&1; then
  echo "需要 gh CLI（https://cli.github.com）" >&2
  exit 2
fi

# 发版 PR 只应改动这 6 个文件：三个版本号载体 + CHANGELOG + 两个 extra-files。
# 多一个都说明这不是纯发版 PR，或者 version-file 配错导致整文件被替换。
ALLOWED_FILES=(
  ".release-please-manifest.json"
  ".release-please-version"
  "CHANGELOG.md"
  "Cargo.toml"
  "Cargo.lock"
  "ROADMAP.md"
)
# 锁定版本号必须同步的四个 package（见 .release-please-config.json 的 extra-files）。
LOCK_PACKAGES=("assay" "assay-core" "assay-api" "assay-web")

BLOCKERS=0
WARNINGS=0

fail() { printf '  ✗ %s\n' "$*" >&2; BLOCKERS=$((BLOCKERS + 1)); }
warn() { printf '  ! %s\n' "$*"; WARNINGS=$((WARNINGS + 1)); }
ok()   { printf '  ✓ %s\n' "$*"; }

# 带重试的 gh 调用。RELEASE_CHECKLIST 第 174 行记过这个坑：`gh api` 实测返回过
# `Post "https://api.github.com/graphql": EOF`，不重试会让门禁在网络最抖的那一刻
# 把「查不到」误报成「release 有问题」。宁可重试，也不把网络故障当结论。
ghx() {
  local i out
  for i in 1 2 3; do
    if out="$(gh "$@" 2>/dev/null)"; then
      printf '%s' "$out"
      return 0
    fi
    sleep "$i"
  done
  return 1
}

# 关键元数据拿不到就无法核对，必须明确退出 2（环境问题），不能给出错误结论。
die_net() { echo "网络/接口不稳定，重试 3 次仍失败，无法核对：$*" >&2; exit 2; }

# semver 比较：$1 > $2 时返回 0。用 sort -V 而不是算术拆段，
# 免得把 0.2.9 和 0.2.11 按字典序比错。
semver_gt() {
  [[ "$1" != "$2" ]] && [[ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" == "$1" ]]
}

REPO="$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null)" || die_net "gh repo view"

echo "== 发版 PR #${PR} 核对（${REPO}） =="

META="$(ghx pr view "$PR" --json number,title,state,headRefName,baseRefName,headRefOid \
  --jq '"\(.number)\t\(.title)\t\(.state)\t\(.headRefName)\t\(.baseRefName)\t\(.headRefOid)"')" || die_net "gh pr view $PR"
IFS=$'\t' read -r PR_NUM TITLE STATE BRANCH BASE HEAD_SHA <<<"$META"
echo "  标题：${TITLE}"
echo "  状态：${STATE}    分支：${BRANCH} → ${BASE}"
echo

# ---------- 1. 版本号能不能从标题里读出来 ----------
echo "1) 版本号"
VERSION="$(printf '%s' "$TITLE" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)"
if [[ -z "$VERSION" ]]; then
  fail "标题里没有形如 x.y.z 的版本号，无法核对"
  echo
  echo "结论：无法继续，先修正 PR 标题或确认这不是 release-please 开的 PR。"
  exit 1
fi
ok "标题版本 = ${VERSION}"

# ---------- 2. 改动文件白名单 ----------
echo "2) 改动文件白名单"
FILES="$(ghx pr diff "$PR" --name-only)" || die_net "gh pr diff --name-only $PR"
UNEXPECTED=()
while IFS= read -r f; do
  [[ -z "$f" ]] && continue
  hit=0
  for a in "${ALLOWED_FILES[@]}"; do
    [[ "$f" == "$a" ]] && { hit=1; break; }
  done
  ((hit == 0)) && UNEXPECTED+=("$f")
done <<<"$FILES"

if ((${#UNEXPECTED[@]})); then
  for f in "${UNEXPECTED[@]}"; do
    fail "不该出现：${f}"
  done
  echo "    发版 PR 只允许改：${ALLOWED_FILES[*]}"
else
  ok "只改了白名单中的文件（$(printf '%s\n' "$FILES" | grep -c . || true) 个）"
fi

# ---------- 3. Cargo.toml 只能动版本那一行 ----------
# 整文件替换（`@@ -1,32 +1 @@`）是 version-file 配成 Cargo.toml 的典型症状，
# 曾差点把整个 workspace 清单清空（PR #37）。所以这里数加减行，不数 hunk。
echo "3) Cargo.toml 改动粒度"
PATCH="$(ghx pr diff "$PR")" || die_net "gh pr diff $PR"
TOML_PLUS=0
TOML_MINUS=0
if printf '%s\n' "$PATCH" | grep -q '^diff --git .*Cargo\.toml'; then
  TOML_PLUS="$(printf '%s\n' "$PATCH" | awk '
    /^diff --git / { on = ($0 ~ /\/Cargo\.toml$/) ; next }
    on && /^\+/ && !/^\+\+\+/ { n++ } END { print n+0 }')"
  TOML_MINUS="$(printf '%s\n' "$PATCH" | awk '
    /^diff --git / { on = ($0 ~ /\/Cargo\.toml$/) ; next }
    on && /^-/ && !/^---/ { n++ } END { print n+0 }')"
  if ((TOML_PLUS > 1 || TOML_MINUS > 1)); then
    fail "Cargo.toml 增 ${TOML_PLUS} 行 / 删 ${TOML_MINUS} 行，超过「只改版本一行」"
    echo "    出现整文件替换说明 version-file 配错，立刻关 PR（见 RELEASE_CHECKLIST 第 2 节）"
  else
    ok "Cargo.toml 增 ${TOML_PLUS} 行 / 删 ${TOML_MINUS} 行"
    # 加减行内容必须就是版本行，防止它改的是别的一行。
    TOML_DIFF="$(printf '%s\n' "$PATCH" | awk '
      /^diff --git / { on = ($0 ~ /\/Cargo\.toml$/) ; next }
      on && /^[+-]/ && !/^(\+\+\+|---)/ { print }')"
    if [[ -n "$TOML_DIFF" ]] && ! printf '%s' "$TOML_DIFF" | grep -qvE '^[+-]version = "'; then
      ok "改的确实是 version 行"
    elif [[ -n "$TOML_DIFF" ]]; then
      fail "Cargo.toml 改的不是 version 行："
      printf '%s\n' "$TOML_DIFF" | sed 's/^/      /' >&2
    fi
  fi
else
  fail "Cargo.toml 没改，但发版必须更新版本号"
fi

# ---------- 4. 版本号在四处必须一致 ----------
# .release-please-version / manifest / Cargo.toml / Cargo.lock(4 包)。
# 漏一处不会立刻报错，只会让 `cargo --locked` 或下载到的二进制版本对不上。
echo "4) 版本号一致性"
# 用 raw Accept 直接取该 PR 头部提交的文件内容，不切工作区。
# 用 head SHA 而不是分支名：合并时 `--delete-branch` 会删掉分支，
# 分支名查 contents 会 404（PR #104 实测踩到），SHA 在对象库里仍然有效。
raw_file() {
  ghx api -H "Accept: application/vnd.github.raw" "repos/${REPO}/contents/$1?ref=${HEAD_SHA}" || true
}

V_FILE="$(raw_file ".release-please-version" | tr -d '[:space:]')"
V_MANIFEST="$(raw_file ".release-please-manifest.json" | tr -d '\n' | grep -oE '"\."[[:space:]]*:[[:space:]]*"[0-9.]+"' | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || true)"
V_TOML="$(raw_file "Cargo.toml" | grep -E '^version = "[0-9.]+"' | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || true)"
V_ROADMAP="$(raw_file "ROADMAP.md" | grep -E 'x-release-please-version' | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || true)"

check_version() { # name actual expected
  if [[ "$2" == "$3" ]]; then
    ok "$1 = $2"
  else
    fail "$1 = ${2:-<读不到>}，应为 $3"
  fi
}
check_version ".release-please-version" "$V_FILE" "$VERSION"
check_version ".release-please-manifest.json" "$V_MANIFEST" "$VERSION"
check_version "Cargo.toml [workspace.package]" "$V_TOML" "$VERSION"
check_version "ROADMAP.md" "$V_ROADMAP" "$VERSION"

LOCK_CONTENT="$(raw_file "Cargo.lock")"
LOCK_VERSIONS="$(printf '%s\n' "$LOCK_CONTENT" | awk -v want="$(printf '%s ' "${LOCK_PACKAGES[@]}")" '
  BEGIN { nw = split(want, arr, " "); for (i = 1; i <= nw; i++) if (arr[i] != "") want_set[arr[i]] = 1 }
  /^\[\[package\]\]/ { if (n in want_set) print n "=" v; n = ""; v = ""; next }
  /^name = /    { n = $3; gsub(/"/, "", n); next }
  /^version = / { v = $3; gsub(/"/, "", v); next }
  END { if (n in want_set) print n "=" v }
')"
for pkg in "${LOCK_PACKAGES[@]}"; do
  got="$(printf '%s\n' "$LOCK_VERSIONS" | grep "^${pkg}=" | cut -d= -f2 || true)"
  check_version "Cargo.lock ${pkg}" "$got" "$VERSION"
done

# ---------- 5. 假发版的硬判据：版本号必须往前走 ----------
# 版本号不可重用，所以「算出来的版本 <= 已发布的最高版本」一定是假发版（或已发过）。
# 这条比 CHANGELOG 内容比对更无歧义，作为主判据。
echo "5) 版本号是否推进"
LATEST_TAG="$(ghx api "repos/${REPO}/releases/latest" --jq .tag_name || true)"
LATEST="${LATEST_TAG#v}"
if [[ -z "$LATEST" ]]; then
  warn "读不到 latest release，跳过推进检查"
elif semver_gt "$VERSION" "$LATEST"; then
  ok "${VERSION} > latest ${LATEST}"
elif [[ "$STATE" == "MERGED" ]]; then
  # 已合并的历史发版，版本低于当前 latest 是正常事实（后面又发了新版本）。
  # 这条判据的用途是拦「还没合并且算出旧版本号」的假发版，不该反过来判历史。
  ok "已合并的历史发版（${VERSION} ≤ latest ${LATEST}），不做拦截"
elif [[ "$VERSION" == "$LATEST" ]]; then
  fail "${VERSION} 正是当前 latest —— 这一版已经发过，版本号不能重用"
else
  fail "${VERSION} 不高于 latest ${LATEST} —— 版本号不能重用，这是假发版或重复 PR"
fi

# ---------- 6. 假发版的本质判据：CHANGELOG 条目是否已经发布过 ----------
# 只增 1 行也可能是假发版：#71 的新增条目 #56 早已在 v0.2.4 出现过。
# 所以逐个比对新增条目里的 PR 号，看它在 main 的 CHANGELOG 里是否已有归属版本。
echo "6) CHANGELOG 新增条目是否重复"
NEW_PRS="$(printf '%s\n' "$PATCH" | awk '
  /^diff --git / { on = ($0 ~ /\/CHANGELOG\.md$/) ; next }
  on && /^\+/ && !/^\+\+\+/ { print }
' | grep -oE '#[0-9]+' | tr -d '#' | sort -un || true)"

if [[ -z "$NEW_PRS" ]]; then
  warn "没有从新增 CHANGELOG 行里解析出 PR 号（可能只有 ### 标题或版本标题）"
else
  if ! git rev-parse --verify --quiet origin/main >/dev/null; then
    git fetch --quiet origin main || true
  fi
  # 把 main 上的每个 PR 号映射到它所在的版本段落，命中时能直接给证据。
  # 注意标题行长这样：`## [0.2.11](https://github.com/.../compare/v0.2.10...v0.2.11) (2026-11-28)`
  # 直接取 $2 会把 `0.2.11](https://...)` 整串吃进去，必须先砍掉 `]` 之后的部分。
  MAIN_PRS="$(git show origin/main:CHANGELOG.md 2>/dev/null | awk '
    /^## \[/ {
      ver = $2
      sub(/\].*$/, "", ver)   # 只保留 [ ] 之间的版本号
      gsub(/^\[/, "", ver)
    }
    {
      rest = $0
      while (match(rest, /#[0-9]+/)) {
        print substr(rest, RSTART + 1, RLENGTH - 1) "\t" ver
        rest = substr(rest, RSTART + RLENGTH)
      }
    }' || true)"

  DUP=0
  DUP_SAMPLES=""
  while IFS= read -r p; do
    [[ -z "$p" ]] && continue
    where="$(printf '%s\n' "$MAIN_PRS" | awk -F'\t' -v p="$p" '$1 == p { print $2; exit }')"
    # 已合并的 PR 会让 main 的 CHANGELOG 里出现「本次目标版本」段落，
    # 条目归属该版本是**正常**的（说明这次发版已经合进去了），不算重复。
    if [[ -n "$where" && "$where" != "$VERSION" ]]; then
      DUP=$((DUP + 1))
      # 只留 5 个样本：假发版常有十几条重复，全打出来会淹掉结论本身。
      if ((DUP <= 5)); then
        DUP_SAMPLES="${DUP_SAMPLES} #${p}→${where}"
      fi
    fi
  done <<<"$NEW_PRS"
  if ((DUP > 0)); then
    fail "${DUP} 条新增条目已在历史版本发布过，又被算进 ${VERSION} —— 内容重复，是假发版"
    echo "    例：${DUP_SAMPLES# }"
    echo "    逐条核实：git show origin/main:CHANGELOG.md | grep -n '#<号>'"
  else
    ok "新增 $(printf '%s\n' "$NEW_PRS" | grep -c . || true) 个 PR 号，均未在历史版本出现过"
  fi
fi

# CHANGELOG 的新段落版本必须等于标题版本，防止「标题写 0.2.11、内容写 0.2.10」。
NEW_HEADING="$(printf '%s\n' "$PATCH" | awk '
  /^diff --git / { on = ($0 ~ /\/CHANGELOG\.md$/) ; next }
  on && /^\+## \[/ { print }
' | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)"
if [[ -z "$NEW_HEADING" ]]; then
  fail "新增 CHANGELOG 里没有形如 ## [x.y.z] 的版本标题"
elif [[ "$NEW_HEADING" == "$VERSION" ]]; then
  ok "CHANGELOG 新段落版本 = ${NEW_HEADING}"
else
  fail "CHANGELOG 新段落写的是 ${NEW_HEADING}，标题是 ${VERSION}"
fi

# ---------- 汇总 ----------
echo
if ((BLOCKERS > 0)); then
  echo "结论：不可合并（${BLOCKERS} 个阻塞问题，${WARNINGS} 个提示）。"
  echo "  处置参照 docs/RELEASE_CHECKLIST.md 第 2 节；假发版直接关掉，不要合并。"
  exit 1
fi
echo "结论：通过（${WARNINGS} 个提示）。发版 PR 只动版本号与 CHANGELOG，可以合并。"
echo "  合并后别忘了：① 确认 draft release 已生成 ② 点火 ③ verify-release.sh 终检。"
