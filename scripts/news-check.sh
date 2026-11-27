#!/usr/bin/env bash
# 公众号日报体检：把 qunmind 导出的条目当成一个「数据源」体检一遍，出人话摘要。
#
# 与 scripts/reuse-news.sh 的分工：
# - reuse-news.sh 是演示 + 断言：固定 fixture、必须查出问题，查不出就算失败
# - 本脚本是生产路径：读真实导出，**查不出问题也算成功**（退出码只看跑没跑通）
#
# 用法：
#   bash scripts/news-check.sh                 # 读 out-news/items.json
#   bash scripts/news-check.sh path/items.json # 读指定导出
#   bash scripts/news-check.sh --export        # 先调 qunmind export-items 再体检
#   bash scripts/news-check.sh --gate          # 体检完顺手判「今天能不能发报」
#   bash scripts/news-check.sh -o out-news2    # 换产物目录
#
# 环境变量：
#   ASSAY_BIN     指定 assay 可执行文件（默认 target/release/assay，缺了先 cargo build）
#   QUNMIND_BIN   指定 qunmind 可执行文件（--export 时用，默认查 PATH）
#   QUNMIND_CONFIG 指定 qunmind 配置文件（--export 时用；不设则按 qunmind 默认读 CWD）
#
# 退出码：不带 --gate 时 0 = 跑通 / 1 = 跑不通 / 2 = 用法或环境不对；
#         带 --gate 时透传 news-gate.sh 的三档：0 可发 / 1 拦截 / 2 跑批不可信。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT="out-news"
EXPORT=0
GATE=0
ITEMS=""
# 用法从文件头的注释里抠出来，避免改了说明忘了改行号。
usage() {
  awk 'NR>1 && /^#/ {sub(/^# ?/, ""); print; next} NR>1 {exit}' "${BASH_SOURCE[0]}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -o|--out)    OUT="${2:-}"; shift 2 ;;
    -e|--export) EXPORT=1; shift ;;
    -g|--gate)   GATE=1; shift ;;
    -h|--help)   usage; exit 0 ;;
    -*)          echo "不认识的参数：$1" >&2; usage >&2; exit 2 ;;
    *)           ITEMS="$1"; shift ;;
  esac
done

mkdir -p "$OUT"

# ── 1. 备好二进制与输入 ───────────────────────────────────
BIN="${ASSAY_BIN:-$ROOT/target/release/assay}"
if [[ ! -x "$BIN" ]]; then
  echo "没找到 assay 可执行文件，先编一份（cargo build --release -p assay）…" >&2
  cargo build --release -q -p assay || { echo "编译失败" >&2; exit 1; }
  BIN="$ROOT/target/release/assay"
fi

if [[ "$EXPORT" -eq 1 ]]; then
  QM="${QUNMIND_BIN:-$(command -v qunmind || true)}"
  if [[ -z "$QM" || ! -x "$QM" ]]; then
    echo "要 --export 但找不到 qunmind：设 QUNMIND_BIN 或装到 PATH；" >&2
    echo "也可以先在 qunmind 仓库跑 qunmind export-items --output items.json，再把路径传进来。" >&2
    exit 2
  fi
  ITEMS="$OUT/items.json"
  QM_ARGS=(export-items --output "$ITEMS")
  if [[ -n "${QUNMIND_CONFIG:-}" ]]; then
    QM_ARGS+=(--config "$QUNMIND_CONFIG")
  elif [[ ! -f config.toml ]]; then
    # qunmind 的 --config 默认是 CWD/config.toml。本脚本开头 cd 到 assay 根目录，
    # 不显式传路径就会去读 assay/config.toml（不存在），报出来的错还很像「没启用来源」。
    echo "提示：当前目录没有 config.toml，qunmind 会按默认路径读 CWD/config.toml。" >&2
    echo "设 QUNMIND_CONFIG=/path/to/qunmind/config.toml 再跑 --export。" >&2
  fi
  echo "导出条目：$QM ${QM_ARGS[*]}"
  # qunmind 在没有启用任何公共来源时报错——这是刻意的，空数组会被读成「今天没新闻」。
  if ! "$QM" "${QM_ARGS[@]}"; then
    # 按实际踩过的顺序排：export-items 是 2026-11-26 才进 qunmind origin/main 的，
    # 旧二进制（含 tag v0.1.0）报「未知子命令」，很容易被误读成配置问题。
    echo "qunmind 导出失败。按可能性排序排查：" >&2
    echo "  1) 二进制太旧：'$QM --help | grep export-items' 没输出 —— 从 qunmind origin/main 重新构建" >&2
    echo "  2) 配置路径不对（见上面的 QUNMIND_CONFIG 提示）" >&2
    echo "  3) 配置里没有启用任何 public_sources" >&2
    exit 1
  fi
fi

ITEMS="${ITEMS:-$OUT/items.json}"
if [[ ! -s "$ITEMS" ]]; then
  echo "读不到条目文件：$ITEMS" >&2
  echo "先在 qunmind 仓库跑：qunmind export-items --output items.json，再传路径给本脚本。" >&2
  exit 2
fi

# ── 2. 体检 → 聚合 ────────────────────────────────────────
# audit 只抓详情、report 才落 checks.txt / summary.json / 榜单。跑完的目录是自洽的：
# items.jsonl 是逐条证据，summary.json 是汇总，run.json 记了这次的口径（含 platform）。
echo "体检：$("$BIN" --version)"
if ! "$BIN" --out "$OUT" --source "news:$ITEMS" audit --threads 4; then
  echo "体检阶段失败，看上面的报错。" >&2
  exit 1
fi
if ! "$BIN" --out "$OUT" report; then
  echo "聚合阶段失败，看上面的报错。" >&2
  exit 1
fi
# report 只落 items.jsonl / summary.json / 榜单，检查项明细只打到 stdout，
# 想留证据得自己接出去；接的是同一份 items.jsonl 与 catalog_snapshot.json。
if ! "$BIN" checks --items "$OUT/items.jsonl" --catalog "$OUT/catalog_snapshot.json" \
    >"$OUT/checks.txt"; then
  echo "检查项阶段失败，看上面的报错。" >&2
  exit 1
fi

# ── 3. 人话摘要 ───────────────────────────────────────────
python3 - "$OUT" "$ITEMS" <<'PY'
import json, os, sys
from collections import Counter

out, items_path = sys.argv[1], sys.argv[2]
j = lambda n: json.load(open(os.path.join(out, n), encoding="utf-8"))
rows = [json.loads(l) for l in open(os.path.join(out, "items.jsonl"), encoding="utf-8") if l.strip()]
summary, run = j("summary.json"), j("run.json")
srcs = Counter(r.get("dept") or "（未标来源）" for r in rows)

def line(s=""):
    print(s)

line()
line(f"公众号日报体检 · {run.get('platform') or '未知来源'}")
line(f"条目 {len(rows)} 条 / 来源 {len(srcs)} 个："
     + "、".join(f"{k}({v})" for k, v in srcs.most_common(5))
     + ("…" if len(srcs) > 5 else ""))
line("─" * 52)
line("① 三个数")
line(f"   说明不足：{summary.get('noDescription', 0)} 条（口径 = checks 的 desc-too-short）")
line(f"   超 90 天未更新：{summary.get('lagOver90Days', 0)} 条")
line(f"   一致性错位：{summary.get('schemaMismatch', 0)} 条"
     + ("（声明列与实际列对得上）" if not summary.get("schemaMismatch") else "（上游声明与实际输出不一致）"))

# 这里的 10 与 crates/core/src/checks.rs 的 DESC_MIN 是同一个口径：改了那边要同步改这里。
# 万一忘了同步，下面的 noDescription 对不上会打一行「口径可能变过」，不会静默矛盾。
short = sorted((r for r in rows if (r.get("descLen") or 0) < 10), key=lambda r: r.get("descLen") or 0)
line("② 最该补说明的 5 条")
if short:
    for r in short[:5]:
        line(f"   {r.get('dept') or '未标来源'} · {(r.get('name') or '')[:34]}（{r.get('descLen', 0)} 字）")
    if len(short) > 5:
        line(f"   …… 还有 {len(short) - 5} 条，见 {out}/checks.txt")
elif summary.get("noDescription"):
    line(f"   摘要说有 {summary['noDescription']} 条说明不足，但按 10 字没筛出来——")
    line("   多半是 checks::DESC_MIN 改过而本脚本没跟上，去对一下阈值。")
else:
    line("   没有——这批条目的说明都够长")

line("③ 产物")
for f in ("items.jsonl", "summary.json", "checks.txt", "leaderboard.md"):
    p = os.path.join(out, f)
    line(f"   {p}{'' if os.path.exists(p) else '（缺）'}")
line("─" * 52)
line("口径：描述缺口按每条条目的 summary/摘要长度算；本脚本只体检，不生成日报正文。")
PY

echo

if [[ "$GATE" -eq 1 ]]; then
  echo "── 发报门禁 ──────────────────────────────────────────"
  bash "$ROOT/scripts/news-gate.sh" "$OUT/summary.json"
  exit $?
fi

echo "退出码 0：体检跑完了。有问题的条目看上面的摘要与 $OUT/checks.txt。"
