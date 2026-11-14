"""字段一致性分类的口径测试。

与 rust-core/src/lib.rs 里的 classify 测试用同一组用例，
任何一边改了规则，这里或那边会先红。
"""
import unittest

from scoring import classify


def item(declared, sample, missing=None, extra=None):
    return {
        "declaredCols": declared,
        "sampleCols": sample,
        "missingFields": missing or [],
        "extraFields": extra or [],
    }


class ClassifyTest(unittest.TestCase):
    def test_no_sample(self):
        self.assertEqual(classify(item(9, 0)), "无样例")

    def test_consistent(self):
        self.assertEqual(classify(item(3, 3)), "一致")

    def test_column_count_mismatch(self):
        self.assertEqual(
            classify(item(9, 8, ["更新时间"])), "列数不一致"
        )

    def test_prefix_naming_only(self):
        self.assertEqual(
            classify(item(2, 2, ["鹿西乡民宿企业名称", "鹿西乡民宿房间数"], ["企业名称", "房间数"])),
            "仅命名不一致",
        )

    def test_field_set_mismatch(self):
        self.assertEqual(
            classify(item(1, 1, ["甲公司名称"], ["联系人电话"])), "字段集合不一致"
        )


if __name__ == "__main__":
    unittest.main()
