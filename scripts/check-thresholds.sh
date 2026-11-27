#!/usr/bin/env bash
# 比对 core 与前端的判定阈值，防止「改一处忘一处」。
#
# 为什么需要这道门禁：web crate 刻意不依赖 core（core 带 ureq 等网络依赖，
# 拉进 wasm 会撑大产物），所以两边没法共享一个 const。数值漂移不会编译失败，
# 只会让页面和报告给出两个互相打脸的数字——项目已经吃过这个亏（见 checks.rs
# 模块文档与 D30）。这道门禁把「两边相等」变成 CI 里可判定的事实。
#
# 用法：bash scripts/check-thresholds.sh
# 退出码：0 = 两边一致；1 = 有漂移；2 = 解析不到常量（源码格式变了）。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 - <<'PY'
import re
import sys
from pathlib import Path

CORE_CHECKS = Path("crates/core/src/checks.rs")
CORE_SCORING = Path("crates/core/src/scoring.rs")
WEB_CONST = Path("crates/web/src/constants.rs")


def const_from(text: str, name: str, pattern: str) -> str:
    """从源码文本里取一个常量右值，去掉行尾注释与空白。"""
    m = re.search(pattern.format(NAME=re.escape(name)), text)
    if not m:
        raise ValueError(f"找不到常量 {name}（源码格式可能变了）")
    return m.group(1).strip()


def normalize_num(s: str) -> str:
    """数值归一：0.30 与 0.3 视为相等，避免纯文本比对误报。"""
    try:
        return repr(float(s))
    except ValueError:
        return s.rstrip(";").strip()


try:
    checks = CORE_CHECKS.read_text(encoding="utf-8")
    scoring = CORE_SCORING.read_text(encoding="utf-8")
    web = WEB_CONST.read_text(encoding="utf-8")
except OSError as e:
    print(f"读源码失败：{e}", file=sys.stderr)
    sys.exit(2)

NUM_PAT = r"pub const {NAME}[^=]*=\s*([0-9.]+)\s*;"
LIST_PAT = r"pub const {NAME}[^=]*=\s*(\[[^]]*\])\s*;"

# 每一项：(人话名字, core 值, 前端值, 是否按数值比对)
pairs = []
for name in ("DESC_MIN", "EMPTY_RATE_HIGH"):
    try:
        c = const_from(checks, name, NUM_PAT)
        w = const_from(web, name, NUM_PAT)
        pairs.append((name, normalize_num(c), normalize_num(w), True))
    except ValueError as e:
        print(f"解析失败：{e}", file=sys.stderr)
        sys.exit(2)

try:
    c_stop = const_from(scoring, "STOPPED_CYCLES", LIST_PAT)
    w_stop = const_from(web, "STOPPED_CYCLES", LIST_PAT)
    # 列表按内容归一：空白与尾逗号不影响语义
    norm_list = lambda s: re.findall(r'"([^"]*)"', s)
    pairs.append(("STOPPED_CYCLES", norm_list(c_stop), norm_list(w_stop), False))
except ValueError as e:
    print(f"解析失败：{e}", file=sys.stderr)
    sys.exit(2)

bad = 0
print("阈值一致性：core ↔ 前端")
for name, c, w, is_num in pairs:
    ok = c == w
    print(f"  {'OK  ' if ok else 'DIFF'} {name:18} core={c}  web={w}")
    if not ok:
        bad += 1

if bad:
    print(
        f"\n{bad} 项不一致。改 `crates/core/src/checks.rs`（或 scoring.rs）时，"
        "必须同步 `crates/web/src/constants.rs`，反之亦然。",
        file=sys.stderr,
    )
    sys.exit(1)

print("  全部一致。")
PY
