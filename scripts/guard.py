#!/usr/bin/env python3
"""数字护栏：解释层唯一允许使用的数字，必须来自既定事实。

模型写人话没问题，但它会在没有依据时编数字（实测在 0 字节页面上编出「数据条数：382」）。
这个模块只做一件事：把输出里出现的每个数字，和输入事实里的数字做集合比对。
对不上就判不合格——不做语义判断，只做数字核对，规则简单到可以被单测覆盖。
"""
import re

NUM = re.compile(r"\d+(?:[.,]\d+)?")


def extract(text: str) -> set[str]:
    """抽出文本里的数字，统一成可比形式：去掉千分位与百分号，去尾零。"""
    out = set()
    for m in NUM.findall(text or ""):
        s = m.replace(",", "")
        if "." in s:
            s = s.rstrip("0").rstrip(".")
        out.add(s)
    return out


def facts_text(facts: dict) -> str:
    """把事实字典摊平成可比文本；同一数值同时给出比例与百分数两种写法。"""
    parts = []
    for k, v in facts.items():
        if isinstance(v, bool):
            continue
        if isinstance(v, (int, float)):
            parts.append(f"{k}={v}")
            if isinstance(v, float) and 0 <= v <= 1:
                parts.append(f"{k}={round(v * 100, 2)}%")
        elif isinstance(v, (list, tuple)):
            parts.append(f"{k}=" + "、".join(str(x) for x in v))
        elif v is not None:
            parts.append(f"{k}={v}")
    return "\n".join(parts)


def check(text: str, facts: dict, extra_allowed=()) -> tuple[bool, list[str]]:
    """返回 (是否合格, 不在事实里的数字)。"""
    allowed = extract(facts_text(facts)) | set(extra_allowed)
    used = extract(text)
    bad = sorted(used - allowed, key=lambda s: (len(s), s))
    return (not bad), bad
