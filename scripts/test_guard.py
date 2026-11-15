#!/usr/bin/env python3
"""数字护栏的单测：不联网，CI 里跑。"""
import unittest

from guard import check, extract, facts_text


class GuardTest(unittest.TestCase):
    def test_extract_normalizes(self):
        self.assertEqual(extract("24,322 个数据集，6.7% 空值"), {"24322", "6.7"})

    def test_ratio_and_percent_equivalent(self):
        facts = {"emptyRate": 0.667}
        ok, bad = check("样例空值率 66.7%，属于偏高。", facts)
        self.assertTrue(ok, bad)

    def test_thousands_separator(self):
        facts = {"audited": 24322}
        ok, bad = check("共体检 24,322 个数据集。", facts)
        self.assertTrue(ok, bad)

    def test_fabricated_number_rejected(self):
        facts = {"declaredCols": 4, "sampleCols": 3}
        ok, bad = check("这份数据有 382 条记录，字段 4 个。", facts)
        self.assertFalse(ok)
        self.assertEqual(bad, ["382"])

    def test_plain_text_without_numbers_passes(self):
        ok, bad = check("这份数据的字段说明过短，且长期未更新。", {"declaredCols": 4})
        self.assertTrue(ok, bad)

    def test_facts_text_flattens_lists(self):
        t = facts_text({"missingFields": ["更新时间", "备注"]})
        self.assertIn("更新时间", t)
        self.assertIn("备注", t)


if __name__ == "__main__":
    unittest.main()
