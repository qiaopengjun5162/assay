#!/usr/bin/env python3
"""往 build log 追加一条带时间戳的实操记录。

用法：
  python3 log_step.py --phase "阶段 6" --title "全量体检" \
      --cmd "cargo run --release -- --limit 24322" \
      --out "..." --conclusion "24,322 个全部拿到，耗时 11 分钟"

  # 输出较长时从 stdin 读
  ./target/release/dataproof-core --limit 5 | python3 log_step.py \
      --phase "阶段 6" --title "首次运行" --cmd "./dataproof-core --limit 5" --out - \
      --conclusion "报错，原因是 X"

设计原则：
  - 只追加，不改历史
  - 输出按原样落盘（超长只截断尾部并标注）
  - 事实（命令/输出）与判断（结论）分字段，写文章时不会混
"""
import argparse
import datetime
import os
import subprocess
import sys

LOG = os.path.expanduser(
    "~/Code/personal-learning/notes/tech/ai/2026-11-14-dataproof-build-log.md"
)


def now() -> str:
    return datetime.datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S %Z")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--phase", required=True, help="所属阶段，如「阶段 6：全量体检」")
    ap.add_argument("--title", required=True, help="一句话说明这一步在做什么")
    ap.add_argument("--cmd", default="", help="实际执行的命令（原样记录）")
    ap.add_argument("--out", default="", help="真实输出；传 - 表示从 stdin 读")
    ap.add_argument("--why", default="", help="为什么做这一步")
    ap.add_argument("--conclusion", default="", help="得出的结论/判断")
    ap.add_argument("--log", default=LOG)
    a = ap.parse_args()

    out = a.out
    if out == "-":
        out = sys.stdin.read()

    try:
        commit = subprocess.run(
            ["git", "-C", os.path.dirname(a.log), "rev-parse", "--short", "HEAD"],
            capture_output=True, text=True, timeout=5,
        ).stdout.strip()
    except Exception:
        commit = ""

    lines = [f"\n### {now()} · {a.title}\n"]
    if commit:
        lines.append(f"- 代码版本：`{commit}`")
    if a.why:
        lines.append(f"- 为什么：{a.why}")
    if a.cmd:
        lines.append("\n```bash\n" + a.cmd.strip() + "\n```")
    if out.strip():
        body = out.rstrip()
        if len(body) > 12000:
            body = body[:6000] + "\n...（截断）...\n" + body[-3000:]
        lines.append("\n```\n" + body + "\n```")
    if a.conclusion:
        lines.append(f"\n**结论**：{a.conclusion}")
    lines.append(f"\n<!-- phase: {a.phase} -->\n")

    with open(a.log, "a", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print(f"已追加到 {a.log}（{a.phase} / {a.title}）")


if __name__ == "__main__":
    main()
