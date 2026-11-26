#!/usr/bin/env bash
# 发报门禁：读体检产物，回答「今天这批素材能不能进日报」。
#
# 三档退出码（和周度巡检的口径一致：先判跑批可不可信，再判指标漂没漂）：
#   0 = 可发报
#   1 = 拦截：结构或时效漂了，要人确认后再发
#   2 = 跑批不可信：数据本身坏了 / 产物过期，别拿它做判断
#
# 用法：
#   bash scripts/news-gate.sh [summary.json，默认 out-news/summary.json]
#   just news-gate
#   bash scripts/news-check.sh --export --gate   # 导出 + 体检 + 门禁一条龙
#
# 环境变量（都是可覆盖的经验值，依据见 docs/SCHEDULING.md）：
#   GATE_MIN_ITEMS=10          条目数下限，低于此判「跑批不可信」
#   GATE_MIN_FETCH_RATE=0.9    抓取成功率下限，低于此判不可信
#   GATE_MAX_MISMATCH=0        一致性错位上限，超过即拦截
#   GATE_MAX_LAG_RATIO=0.2     超 90 天条目占比上限，超过即拦截
#   GATE_MAX_AGE_HOURS=26      产物最大小时数，防止上次跑的旧文件被当成今天的结果
#
# 注意：**「说明不足」不参与拦截**。真实数据里 49/59 条只有标题没摘要，
# 拿它当门禁等于天天不发报；它是给编辑的提醒，不是发报条件。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SUMMARY="${1:-$ROOT/out-news/summary.json}"
usage() {
  awk 'NR>1 && /^#/ {sub(/^# ?/, ""); print; next} NR>1 {exit}' "${BASH_SOURCE[0]}"
}
[[ "${1:-}" == "-h" || "${1:-}" == "--help" ]] && { usage; exit 0; }

python3 - "$SUMMARY" \
  "${GATE_MIN_ITEMS:-10}" \
  "${GATE_MIN_FETCH_RATE:-0.9}" \
  "${GATE_MAX_MISMATCH:-0}" \
  "${GATE_MAX_LAG_RATIO:-0.2}" \
  "${GATE_MAX_AGE_HOURS:-26}" <<'PY'
import json, os, sys
from datetime import datetime, timezone

summary_path, min_items, min_rate, max_mismatch, max_lag, max_age = sys.argv[1:]
min_items = int(min_items)
min_rate = float(min_rate)
max_mismatch = int(max_mismatch)
max_lag = float(max_lag)
max_age = float(max_age)

def report(code, marker, msg):
    print(f"GATE={marker}")
    print(msg)
    sys.exit(code)

folder = os.path.dirname(os.path.abspath(summary_path))
if not os.path.isfile(summary_path):
    report(2, "UNTRUSTED", f"读不到 {summary_path}；先跑 bash scripts/news-check.sh。")

try:
    s = json.load(open(summary_path, encoding="utf-8"))
    run = json.load(open(os.path.join(folder, "run.json"), encoding="utf-8"))
except (OSError, ValueError) as e:
    report(2, "UNTRUSTED", f"产物读不出来（文件坏了或没生成）：{e}")

audited = int(s.get("audited") or 0)
preview = int(s.get("withPreview") or 0)
mismatch = int(s.get("schemaMismatch") or 0)
lag = int(s.get("lagOver90Days") or 0)
no_desc = int(s.get("noDescription") or 0)

# ① 产物新鲜度：cron 里最阴的失败是「这轮导出挂了，但上轮的 out-news/ 还在」，
#    门禁读到旧 summary 照样放行。宁可误拦，不能拿着昨天的结论发今天的报。
captured = run.get("capturedAt")
if not captured:
    report(2, "UNTRUSTED", "run.json 缺 capturedAt，无法判断产物是哪一轮的。")
try:
    when = datetime.fromisoformat(captured)
except ValueError:
    report(2, "UNTRUSTED", f"capturedAt 不是 ISO 时间：{captured!r}")
if when.tzinfo is None:
    when = when.replace(tzinfo=timezone.utc)
age_hours = (datetime.now(timezone.utc) - when.astimezone(timezone.utc)).total_seconds() / 3600
if age_hours < -1:
    report(2, "UNTRUSTED", f"capturedAt 在未来（{captured}），时钟或文件有问题。")
if age_hours > max_age:
    report(2, "UNTRUSTED",
           f"产物已过期：{age_hours:.1f} 小时前生成（上限 {max_age:g} 小时，{captured}）。")

# ② 跑批健康：这两条不达标，指标再好看也不能信。
if audited < min_items:
    report(2, "UNTRUSTED",
           f"条目只有 {audited} 条（下限 {min_items}）。多半是上游没启用、抓取失败或导出空数组。")
rate = preview / audited
if rate < min_rate:
    report(2, "UNTRUSTED",
           f"抓取成功率 {rate:.1%}（{preview}/{audited}，下限 {min_rate:.0%}），这批数据不可信。")

# ③ 漂移：拦的是「结构变了 / 旧闻混进来」，这两件事会直接误导读者。
reasons = []
if mismatch > max_mismatch:
    reasons.append(f"一致性错位 {mismatch} 条（上限 {max_mismatch}）——上游声明与实际输出对不上")
lag_ratio = lag / audited
if lag_ratio > max_lag:
    reasons.append(f"超 90 天条目 {lag}/{audited} = {lag_ratio:.1%}（上限 {max_lag:.0%}）——旧闻混进今日素材")
if reasons:
    report(1, "BLOCK", "别发报，先确认：\n  - " + "\n  - ".join(reasons))

# ④ 放行，但把「说明不足」作为编辑提醒留着——不拦，是因为真实占比就是八成多。
print("GATE=PASS")
print(f"可发报：{audited} 条，抓取成功率 {rate:.1%}，一致性错位 0，超 90 天 {lag_ratio:.1%}。")
if no_desc:
    print(f"提醒（不拦截）：{no_desc} 条说明不足，编辑补摘要再发更好。")
PY
