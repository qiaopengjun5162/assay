/-
  自动生成 by dataproof-core
  平台: 浙江省公共数据开放平台
  快照 SHA-256: fixture-not-a-real-hash
  说明: 1 = 空值。改动任何数字，内核将报错。
-/

/-- 某列空值条数 -/
def countEmpty (rs : List (List Nat)) (col : Nat) : Nat :=
  (rs.filter (fun r => r.getD col 0 == 1)).length

-- 事业单位法人年度报告信息 | 样例 3x4
def rows_0 : List (List Nat) := [
  [0, 0, 0, 0],
  [0, 0, 0, 1],
  [0, 1, 0, 0]
]

-- 公共资源交易中标结果信息 | 样例 4x4
def rows_1 : List (List Nat) := [
  [0, 0, 0, 0],
  [0, 0, 0, 1],
  [0, 0, 1, 1],
  [0, 0, 0, 0]
]

-- 城市公厕基础信息 | 样例 3x3
def rows_2 : List (List Nat) := [
  [0, 0, 0],
  [0, 0, 0],
  [0, 1, 0]
]

theorem empty_0_0 : countEmpty rows_0 0 = 0 := by decide
theorem empty_0_1 : countEmpty rows_0 1 = 1 := by decide
theorem empty_0_2 : countEmpty rows_0 2 = 0 := by decide
theorem empty_0_3 : countEmpty rows_0 3 = 1 := by decide
theorem empty_1_0 : countEmpty rows_1 0 = 0 := by decide
theorem empty_1_1 : countEmpty rows_1 1 = 0 := by decide
theorem empty_1_2 : countEmpty rows_1 2 = 1 := by decide
theorem empty_1_3 : countEmpty rows_1 3 = 2 := by decide
theorem empty_2_0 : countEmpty rows_2 0 = 0 := by decide
theorem empty_2_1 : countEmpty rows_2 1 = 1 := by decide
theorem empty_2_2 : countEmpty rows_2 2 = 0 := by decide
