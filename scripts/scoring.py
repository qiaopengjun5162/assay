#!/usr/bin/env python3
"""体检评分与字段一致性分类（aggregate.py / export_web.py / proof_top.py 共用）。

为什么要单独拿出来：第一版把「声明列名带部门前缀、样例用短名」误判成「字段缺失」，
导致 1,251 条被算成结构不一致。分类必须区分三件事：

  列数不一致    声明 9 列、样例 8 列 —— 真的少了数据
  字段集合不一致 列数相同，但名字对不上且不是前缀关系 —— 文档与数据不对应
  仅命名不一致  列数相同，声明名 = 前缀 + 样例名 —— 只是命名习惯，不算数据缺失
"""
STOPPED = {"-1", "999", "不更新"}


def classify(item: dict) -> str:
    miss = item.get("missingFields") or []
    extra = item.get("extraFields") or []
    dc = item.get("declaredCols")
    sc = item.get("sampleCols")
    if not sc:
        return "无样例"
    if dc == sc and not miss and not extra:
        return "一致"
    if dc != sc:
        return "列数不一致"
    if miss and extra and len(miss) == len(extra):
        matched = sum(1 for m in miss if any(m.endswith(e) or e.endswith(m) for e in extra))
        if matched == len(miss):
            return "仅命名不一致"
    return "字段集合不一致"


def score(item: dict) -> float:
    """问题分：只用于排序，不是科学指标。权重公开写在代码里。"""
    s = 0.0
    kind = classify(item)
    if kind == "列数不一致":
        s += 3
    elif kind == "字段集合不一致":
        s += 3
    elif kind == "仅命名不一致":
        s += 1
    if item.get("allEmptyCols", 0) > 0:
        s += 2
    s += min(3, (item.get("emptyRate") or 0) * 6)
    if str(item.get("updateCycle")) in STOPPED:
        s += 1
    if item.get("descLen", 0) < 10:
        s += 1
    if not item.get("hasPreview"):
        s += 1
    return round(s, 2)
