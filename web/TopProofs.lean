/-
  自动生成 by proof_top.py
  平台: 浙江省公共数据开放平台
  说明: 1 = 空值；每行对应一条真实样例记录。改动任何数字，内核报错。
-/

/-- 某列空值条数 -/
def countEmpty (rs : List (List Nat)) (col : Nat) : Nat :=
  (rs.filter (fun r => r.getD col 0 == 1)).length

-- 0：少数民族行政村信息 | 样例 5x3 | 空值率 66.7%

def rows_0 : List (List Nat) := [
  [0, 1, 1],
  [0, 1, 1],
  [0, 1, 1],
  [0, 1, 1],
  [0, 1, 1]
]

-- 1：杭州残联-服务阵地-残疾人服务阵地信息 | 样例 4x16 | 空值率 50.0%

def rows_1 : List (List Nat) := [
  [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
]

-- 2：票务线上交易明细信息 | 样例 5x92 | 空值率 50.0%

def rows_2 : List (List Nat) := [
  [1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 1],
  [1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 1],
  [1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 1],
  [1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 1],
  [1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 1]
]

-- 3：南浔区_行政许可信息 | 样例 5x10 | 空值率 50.0%

def rows_3 : List (List Nat) := [
  [1, 1, 0, 1, 1, 0, 0, 1, 0, 0],
  [1, 1, 0, 1, 1, 0, 0, 1, 0, 0],
  [1, 1, 0, 1, 1, 0, 0, 1, 0, 0],
  [1, 1, 0, 1, 1, 0, 0, 1, 0, 0],
  [1, 1, 0, 1, 1, 0, 0, 1, 0, 0]
]

-- 4：安吉县综合执法局农家乐监管一件事协同应用系统-机构主体信息 | 样例 3x63 | 空值率 32.8%

def rows_4 : List (List Nat) := [
  [0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1],
  [0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1],
  [0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 1]
]

-- 5：数字政协_工作建议信息 | 样例 5x18 | 空值率 27.8%

def rows_5 : List (List Nat) := [
  [1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0],
  [1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0],
  [1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0],
  [1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0],
  [1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0]
]

-- 6：票务线上交易主表信息 | 样例 5x72 | 空值率 43.1%

def rows_6 : List (List Nat) := [
  [1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 0],
  [0, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 0, 1, 0],
  [1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 0],
  [1, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 1, 1, 1, 1, 0, 1, 0],
  [1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 0]
]

-- 7：票务线下交易主表信息 | 样例 5x74 | 空值率 38.1%

def rows_7 : List (List Nat) := [
  [1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0],
  [1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0],
  [1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0],
  [1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0],
  [1, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0]
]

-- 8：省级高新技术企业研发中心数据统计信息 | 样例 5x38 | 空值率 36.8%

def rows_8 : List (List Nat) := [
  [1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 1],
  [1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 1],
  [1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 1],
  [1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 1],
  [1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 1]
]

-- 9：德清县-水利局-智慧水利-测站基础信息 | 样例 5x15 | 空值率 20.0%

def rows_9 : List (List Nat) := [
  [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0],
  [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0],
  [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0],
  [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0],
  [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0]
]

-- 10：洞头空间中心-洞头区花园村庄信息 | 样例 5x11 | 空值率 36.4%

def rows_10 : List (List Nat) := [
  [0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1],
  [0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1],
  [0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1],
  [0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1],
  [0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1]
]

-- 11：德清县-水利局-智慧水利-水库基础信息 | 样例 5x98 | 空值率 19.0%

def rows_11 : List (List Nat) := [
  [0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1],
  [0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1],
  [0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1],
  [0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1],
  [0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1]
]

-- 12：数字政协_基层协商信息 | 样例 5x16 | 空值率 18.8%

def rows_12 : List (List Nat) := [
  [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0],
  [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0],
  [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0],
  [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0],
  [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]
]

-- 13：建设项目基础信息 | 样例 5x8 | 空值率 62.5%

def rows_13 : List (List Nat) := [
  [0, 1, 1, 0, 1, 0, 1, 1],
  [0, 1, 1, 0, 1, 0, 1, 1],
  [0, 1, 1, 0, 1, 0, 1, 1],
  [0, 1, 1, 0, 1, 0, 1, 1],
  [0, 1, 1, 0, 1, 0, 1, 1]
]

-- 14：鹿城区规划道路-鹿城区空间大数据汇聚共享平台资源信息 | 样例 5x10 | 空值率 50.0%

def rows_14 : List (List Nat) := [
  [0, 1, 0, 0, 1, 1, 1, 0, 0, 1],
  [0, 1, 0, 0, 1, 1, 1, 0, 0, 1],
  [0, 1, 0, 0, 1, 1, 1, 0, 0, 1],
  [0, 1, 0, 0, 1, 1, 1, 0, 0, 1],
  [0, 1, 0, 0, 1, 1, 1, 0, 0, 1]
]

-- 15：温岭市自规局国土空间农转用地块信息 | 样例 5x9 | 空值率 33.3%

def rows_15 : List (List Nat) := [
  [0, 1, 1, 1, 1, 0, 1, 0, 0],
  [0, 0, 0, 0, 0, 0, 1, 0, 0],
  [0, 0, 1, 0, 0, 0, 1, 0, 0],
  [0, 1, 1, 1, 1, 0, 1, 0, 0],
  [0, 0, 0, 1, 0, 0, 1, 0, 0]
]

-- 16：数字政协_提案交办信息 | 样例 5x26 | 空值率 15.4%

def rows_16 : List (List Nat) := [
  [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0],
  [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0],
  [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0],
  [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0],
  [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0]
]

-- 17：数字政协_委员基础信息 | 样例 5x43 | 空值率 14.4%

def rows_17 : List (List Nat) := [
  [0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 0, 0],
  [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
]

-- 18：南浔区-电子发票申请结果信息 | 样例 5x14 | 空值率 28.6%

def rows_18 : List (List Nat) := [
  [0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0],
  [0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0],
  [0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0],
  [0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0],
  [0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0]
]

-- 19：宅基地数据汇交-建房审批-用地建房申请信息 | 样例 5x53 | 空值率 28.3%

def rows_19 : List (List Nat) := [
  [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0],
  [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 0]
]

theorem empty_0_0 : countEmpty rows_0 0 = 0 := by decide
theorem empty_0_1 : countEmpty rows_0 1 = 5 := by decide
theorem empty_0_2 : countEmpty rows_0 2 = 5 := by decide
theorem empty_1_0 : countEmpty rows_1 0 = 0 := by decide
theorem empty_1_1 : countEmpty rows_1 1 = 0 := by decide
theorem empty_1_2 : countEmpty rows_1 2 = 0 := by decide
theorem empty_1_3 : countEmpty rows_1 3 = 0 := by decide
theorem empty_1_4 : countEmpty rows_1 4 = 0 := by decide
theorem empty_1_5 : countEmpty rows_1 5 = 0 := by decide
theorem empty_1_6 : countEmpty rows_1 6 = 0 := by decide
theorem empty_1_7 : countEmpty rows_1 7 = 0 := by decide
theorem empty_1_8 : countEmpty rows_1 8 = 0 := by decide
theorem empty_1_9 : countEmpty rows_1 9 = 0 := by decide
theorem empty_1_10 : countEmpty rows_1 10 = 0 := by decide
theorem empty_1_11 : countEmpty rows_1 11 = 0 := by decide
theorem empty_1_12 : countEmpty rows_1 12 = 0 := by decide
theorem empty_1_13 : countEmpty rows_1 13 = 0 := by decide
theorem empty_1_14 : countEmpty rows_1 14 = 0 := by decide
theorem empty_1_15 : countEmpty rows_1 15 = 0 := by decide
theorem empty_2_0 : countEmpty rows_2 0 = 5 := by decide
theorem empty_2_1 : countEmpty rows_2 1 = 0 := by decide
theorem empty_2_2 : countEmpty rows_2 2 = 5 := by decide
theorem empty_2_3 : countEmpty rows_2 3 = 0 := by decide
theorem empty_2_4 : countEmpty rows_2 4 = 5 := by decide
theorem empty_2_5 : countEmpty rows_2 5 = 5 := by decide
theorem empty_2_6 : countEmpty rows_2 6 = 0 := by decide
theorem empty_2_7 : countEmpty rows_2 7 = 0 := by decide
theorem empty_2_8 : countEmpty rows_2 8 = 5 := by decide
theorem empty_2_9 : countEmpty rows_2 9 = 5 := by decide
theorem empty_2_10 : countEmpty rows_2 10 = 0 := by decide
theorem empty_2_11 : countEmpty rows_2 11 = 5 := by decide
theorem empty_2_12 : countEmpty rows_2 12 = 5 := by decide
theorem empty_2_13 : countEmpty rows_2 13 = 5 := by decide
theorem empty_2_14 : countEmpty rows_2 14 = 0 := by decide
theorem empty_2_15 : countEmpty rows_2 15 = 0 := by decide
theorem empty_2_16 : countEmpty rows_2 16 = 0 := by decide
theorem empty_2_17 : countEmpty rows_2 17 = 5 := by decide
theorem empty_2_18 : countEmpty rows_2 18 = 5 := by decide
theorem empty_2_19 : countEmpty rows_2 19 = 5 := by decide
theorem empty_2_20 : countEmpty rows_2 20 = 0 := by decide
theorem empty_2_21 : countEmpty rows_2 21 = 0 := by decide
theorem empty_2_22 : countEmpty rows_2 22 = 5 := by decide
theorem empty_2_23 : countEmpty rows_2 23 = 5 := by decide
theorem empty_2_24 : countEmpty rows_2 24 = 5 := by decide
theorem empty_2_25 : countEmpty rows_2 25 = 0 := by decide
theorem empty_2_26 : countEmpty rows_2 26 = 5 := by decide
theorem empty_2_27 : countEmpty rows_2 27 = 0 := by decide
theorem empty_2_28 : countEmpty rows_2 28 = 5 := by decide
theorem empty_2_29 : countEmpty rows_2 29 = 0 := by decide
theorem empty_2_30 : countEmpty rows_2 30 = 0 := by decide
theorem empty_2_31 : countEmpty rows_2 31 = 0 := by decide
theorem empty_2_32 : countEmpty rows_2 32 = 0 := by decide
theorem empty_2_33 : countEmpty rows_2 33 = 5 := by decide
theorem empty_2_34 : countEmpty rows_2 34 = 5 := by decide
theorem empty_2_35 : countEmpty rows_2 35 = 5 := by decide
theorem empty_2_36 : countEmpty rows_2 36 = 5 := by decide
theorem empty_2_37 : countEmpty rows_2 37 = 0 := by decide
theorem empty_2_38 : countEmpty rows_2 38 = 0 := by decide
theorem empty_2_39 : countEmpty rows_2 39 = 5 := by decide
theorem empty_2_40 : countEmpty rows_2 40 = 5 := by decide
theorem empty_2_41 : countEmpty rows_2 41 = 5 := by decide
theorem empty_2_42 : countEmpty rows_2 42 = 5 := by decide
theorem empty_2_43 : countEmpty rows_2 43 = 5 := by decide
theorem empty_2_44 : countEmpty rows_2 44 = 0 := by decide
theorem empty_2_45 : countEmpty rows_2 45 = 0 := by decide
theorem empty_2_46 : countEmpty rows_2 46 = 5 := by decide
theorem empty_2_47 : countEmpty rows_2 47 = 0 := by decide
theorem empty_2_48 : countEmpty rows_2 48 = 0 := by decide
theorem empty_2_49 : countEmpty rows_2 49 = 5 := by decide
theorem empty_2_50 : countEmpty rows_2 50 = 0 := by decide
theorem empty_2_51 : countEmpty rows_2 51 = 0 := by decide
theorem empty_2_52 : countEmpty rows_2 52 = 0 := by decide
theorem empty_2_53 : countEmpty rows_2 53 = 0 := by decide
theorem empty_2_54 : countEmpty rows_2 54 = 0 := by decide
theorem empty_2_55 : countEmpty rows_2 55 = 5 := by decide
theorem empty_2_56 : countEmpty rows_2 56 = 0 := by decide
theorem empty_2_57 : countEmpty rows_2 57 = 0 := by decide
theorem empty_2_58 : countEmpty rows_2 58 = 5 := by decide
theorem empty_2_59 : countEmpty rows_2 59 = 5 := by decide
theorem empty_2_60 : countEmpty rows_2 60 = 5 := by decide
theorem empty_2_61 : countEmpty rows_2 61 = 0 := by decide
theorem empty_2_62 : countEmpty rows_2 62 = 0 := by decide
theorem empty_2_63 : countEmpty rows_2 63 = 5 := by decide
theorem empty_2_64 : countEmpty rows_2 64 = 0 := by decide
theorem empty_2_65 : countEmpty rows_2 65 = 0 := by decide
theorem empty_2_66 : countEmpty rows_2 66 = 0 := by decide
theorem empty_2_67 : countEmpty rows_2 67 = 0 := by decide
theorem empty_2_68 : countEmpty rows_2 68 = 5 := by decide
theorem empty_2_69 : countEmpty rows_2 69 = 5 := by decide
theorem empty_2_70 : countEmpty rows_2 70 = 0 := by decide
theorem empty_2_71 : countEmpty rows_2 71 = 5 := by decide
theorem empty_2_72 : countEmpty rows_2 72 = 0 := by decide
theorem empty_2_73 : countEmpty rows_2 73 = 5 := by decide
theorem empty_2_74 : countEmpty rows_2 74 = 5 := by decide
theorem empty_2_75 : countEmpty rows_2 75 = 0 := by decide
theorem empty_2_76 : countEmpty rows_2 76 = 0 := by decide
theorem empty_2_77 : countEmpty rows_2 77 = 5 := by decide
theorem empty_2_78 : countEmpty rows_2 78 = 5 := by decide
theorem empty_2_79 : countEmpty rows_2 79 = 0 := by decide
theorem empty_2_80 : countEmpty rows_2 80 = 5 := by decide
theorem empty_2_81 : countEmpty rows_2 81 = 0 := by decide
theorem empty_2_82 : countEmpty rows_2 82 = 0 := by decide
theorem empty_2_83 : countEmpty rows_2 83 = 0 := by decide
theorem empty_2_84 : countEmpty rows_2 84 = 5 := by decide
theorem empty_2_85 : countEmpty rows_2 85 = 0 := by decide
theorem empty_2_86 : countEmpty rows_2 86 = 0 := by decide
theorem empty_2_87 : countEmpty rows_2 87 = 5 := by decide
theorem empty_2_88 : countEmpty rows_2 88 = 5 := by decide
theorem empty_2_89 : countEmpty rows_2 89 = 5 := by decide
theorem empty_2_90 : countEmpty rows_2 90 = 0 := by decide
theorem empty_2_91 : countEmpty rows_2 91 = 5 := by decide
theorem empty_3_0 : countEmpty rows_3 0 = 5 := by decide
theorem empty_3_1 : countEmpty rows_3 1 = 5 := by decide
theorem empty_3_2 : countEmpty rows_3 2 = 0 := by decide
theorem empty_3_3 : countEmpty rows_3 3 = 5 := by decide
theorem empty_3_4 : countEmpty rows_3 4 = 5 := by decide
theorem empty_3_5 : countEmpty rows_3 5 = 0 := by decide
theorem empty_3_6 : countEmpty rows_3 6 = 0 := by decide
theorem empty_3_7 : countEmpty rows_3 7 = 5 := by decide
theorem empty_3_8 : countEmpty rows_3 8 = 0 := by decide
theorem empty_3_9 : countEmpty rows_3 9 = 0 := by decide
theorem empty_4_0 : countEmpty rows_4 0 = 0 := by decide
theorem empty_4_1 : countEmpty rows_4 1 = 0 := by decide
theorem empty_4_2 : countEmpty rows_4 2 = 0 := by decide
theorem empty_4_3 : countEmpty rows_4 3 = 0 := by decide
theorem empty_4_4 : countEmpty rows_4 4 = 0 := by decide
theorem empty_4_5 : countEmpty rows_4 5 = 3 := by decide
theorem empty_4_6 : countEmpty rows_4 6 = 0 := by decide
theorem empty_4_7 : countEmpty rows_4 7 = 0 := by decide
theorem empty_4_8 : countEmpty rows_4 8 = 3 := by decide
theorem empty_4_9 : countEmpty rows_4 9 = 0 := by decide
theorem empty_4_10 : countEmpty rows_4 10 = 0 := by decide
theorem empty_4_11 : countEmpty rows_4 11 = 0 := by decide
theorem empty_4_12 : countEmpty rows_4 12 = 3 := by decide
theorem empty_4_13 : countEmpty rows_4 13 = 3 := by decide
theorem empty_4_14 : countEmpty rows_4 14 = 3 := by decide
theorem empty_4_15 : countEmpty rows_4 15 = 0 := by decide
theorem empty_4_16 : countEmpty rows_4 16 = 0 := by decide
theorem empty_4_17 : countEmpty rows_4 17 = 0 := by decide
theorem empty_4_18 : countEmpty rows_4 18 = 0 := by decide
theorem empty_4_19 : countEmpty rows_4 19 = 2 := by decide
theorem empty_4_20 : countEmpty rows_4 20 = 0 := by decide
theorem empty_4_21 : countEmpty rows_4 21 = 0 := by decide
theorem empty_4_22 : countEmpty rows_4 22 = 0 := by decide
theorem empty_4_23 : countEmpty rows_4 23 = 0 := by decide
theorem empty_4_24 : countEmpty rows_4 24 = 1 := by decide
theorem empty_4_25 : countEmpty rows_4 25 = 3 := by decide
theorem empty_4_26 : countEmpty rows_4 26 = 3 := by decide
theorem empty_4_27 : countEmpty rows_4 27 = 0 := by decide
theorem empty_4_28 : countEmpty rows_4 28 = 0 := by decide
theorem empty_4_29 : countEmpty rows_4 29 = 0 := by decide
theorem empty_4_30 : countEmpty rows_4 30 = 0 := by decide
theorem empty_4_31 : countEmpty rows_4 31 = 0 := by decide
theorem empty_4_32 : countEmpty rows_4 32 = 0 := by decide
theorem empty_4_33 : countEmpty rows_4 33 = 3 := by decide
theorem empty_4_34 : countEmpty rows_4 34 = 0 := by decide
theorem empty_4_35 : countEmpty rows_4 35 = 3 := by decide
theorem empty_4_36 : countEmpty rows_4 36 = 0 := by decide
theorem empty_4_37 : countEmpty rows_4 37 = 3 := by decide
theorem empty_4_38 : countEmpty rows_4 38 = 0 := by decide
theorem empty_4_39 : countEmpty rows_4 39 = 0 := by decide
theorem empty_4_40 : countEmpty rows_4 40 = 0 := by decide
theorem empty_4_41 : countEmpty rows_4 41 = 0 := by decide
theorem empty_4_42 : countEmpty rows_4 42 = 0 := by decide
theorem empty_4_43 : countEmpty rows_4 43 = 0 := by decide
theorem empty_4_44 : countEmpty rows_4 44 = 3 := by decide
theorem empty_4_45 : countEmpty rows_4 45 = 3 := by decide
theorem empty_4_46 : countEmpty rows_4 46 = 0 := by decide
theorem empty_4_47 : countEmpty rows_4 47 = 3 := by decide
theorem empty_4_48 : countEmpty rows_4 48 = 0 := by decide
theorem empty_4_49 : countEmpty rows_4 49 = 3 := by decide
theorem empty_4_50 : countEmpty rows_4 50 = 0 := by decide
theorem empty_4_51 : countEmpty rows_4 51 = 2 := by decide
theorem empty_4_52 : countEmpty rows_4 52 = 3 := by decide
theorem empty_4_53 : countEmpty rows_4 53 = 0 := by decide
theorem empty_4_54 : countEmpty rows_4 54 = 3 := by decide
theorem empty_4_55 : countEmpty rows_4 55 = 0 := by decide
theorem empty_4_56 : countEmpty rows_4 56 = 0 := by decide
theorem empty_4_57 : countEmpty rows_4 57 = 3 := by decide
theorem empty_4_58 : countEmpty rows_4 58 = 0 := by decide
theorem empty_4_59 : countEmpty rows_4 59 = 0 := by decide
theorem empty_4_60 : countEmpty rows_4 60 = 0 := by decide
theorem empty_4_61 : countEmpty rows_4 61 = 3 := by decide
theorem empty_4_62 : countEmpty rows_4 62 = 3 := by decide
theorem empty_5_0 : countEmpty rows_5 0 = 5 := by decide
theorem empty_5_1 : countEmpty rows_5 1 = 0 := by decide
theorem empty_5_2 : countEmpty rows_5 2 = 0 := by decide
theorem empty_5_3 : countEmpty rows_5 3 = 0 := by decide
theorem empty_5_4 : countEmpty rows_5 4 = 0 := by decide
theorem empty_5_5 : countEmpty rows_5 5 = 0 := by decide
theorem empty_5_6 : countEmpty rows_5 6 = 0 := by decide
theorem empty_5_7 : countEmpty rows_5 7 = 2 := by decide
theorem empty_5_8 : countEmpty rows_5 8 = 0 := by decide
theorem empty_5_9 : countEmpty rows_5 9 = 3 := by decide
theorem empty_5_10 : countEmpty rows_5 10 = 5 := by decide
theorem empty_5_11 : countEmpty rows_5 11 = 0 := by decide
theorem empty_5_12 : countEmpty rows_5 12 = 0 := by decide
theorem empty_5_13 : countEmpty rows_5 13 = 5 := by decide
theorem empty_5_14 : countEmpty rows_5 14 = 0 := by decide
theorem empty_5_15 : countEmpty rows_5 15 = 5 := by decide
theorem empty_5_16 : countEmpty rows_5 16 = 0 := by decide
theorem empty_5_17 : countEmpty rows_5 17 = 0 := by decide
theorem empty_6_0 : countEmpty rows_6 0 = 4 := by decide
theorem empty_6_1 : countEmpty rows_6 1 = 0 := by decide
theorem empty_6_2 : countEmpty rows_6 2 = 5 := by decide
theorem empty_6_3 : countEmpty rows_6 3 = 2 := by decide
theorem empty_6_4 : countEmpty rows_6 4 = 5 := by decide
theorem empty_6_5 : countEmpty rows_6 5 = 5 := by decide
theorem empty_6_6 : countEmpty rows_6 6 = 0 := by decide
theorem empty_6_7 : countEmpty rows_6 7 = 5 := by decide
theorem empty_6_8 : countEmpty rows_6 8 = 0 := by decide
theorem empty_6_9 : countEmpty rows_6 9 = 5 := by decide
theorem empty_6_10 : countEmpty rows_6 10 = 0 := by decide
theorem empty_6_11 : countEmpty rows_6 11 = 0 := by decide
theorem empty_6_12 : countEmpty rows_6 12 = 4 := by decide
theorem empty_6_13 : countEmpty rows_6 13 = 0 := by decide
theorem empty_6_14 : countEmpty rows_6 14 = 0 := by decide
theorem empty_6_15 : countEmpty rows_6 15 = 0 := by decide
theorem empty_6_16 : countEmpty rows_6 16 = 2 := by decide
theorem empty_6_17 : countEmpty rows_6 17 = 0 := by decide
theorem empty_6_18 : countEmpty rows_6 18 = 3 := by decide
theorem empty_6_19 : countEmpty rows_6 19 = 4 := by decide
theorem empty_6_20 : countEmpty rows_6 20 = 0 := by decide
theorem empty_6_21 : countEmpty rows_6 21 = 0 := by decide
theorem empty_6_22 : countEmpty rows_6 22 = 5 := by decide
theorem empty_6_23 : countEmpty rows_6 23 = 0 := by decide
theorem empty_6_24 : countEmpty rows_6 24 = 0 := by decide
theorem empty_6_25 : countEmpty rows_6 25 = 5 := by decide
theorem empty_6_26 : countEmpty rows_6 26 = 5 := by decide
theorem empty_6_27 : countEmpty rows_6 27 = 5 := by decide
theorem empty_6_28 : countEmpty rows_6 28 = 3 := by decide
theorem empty_6_29 : countEmpty rows_6 29 = 0 := by decide
theorem empty_6_30 : countEmpty rows_6 30 = 5 := by decide
theorem empty_6_31 : countEmpty rows_6 31 = 3 := by decide
theorem empty_6_32 : countEmpty rows_6 32 = 0 := by decide
theorem empty_6_33 : countEmpty rows_6 33 = 0 := by decide
theorem empty_6_34 : countEmpty rows_6 34 = 3 := by decide
theorem empty_6_35 : countEmpty rows_6 35 = 0 := by decide
theorem empty_6_36 : countEmpty rows_6 36 = 3 := by decide
theorem empty_6_37 : countEmpty rows_6 37 = 5 := by decide
theorem empty_6_38 : countEmpty rows_6 38 = 2 := by decide
theorem empty_6_39 : countEmpty rows_6 39 = 0 := by decide
theorem empty_6_40 : countEmpty rows_6 40 = 3 := by decide
theorem empty_6_41 : countEmpty rows_6 41 = 3 := by decide
theorem empty_6_42 : countEmpty rows_6 42 = 5 := by decide
theorem empty_6_43 : countEmpty rows_6 43 = 0 := by decide
theorem empty_6_44 : countEmpty rows_6 44 = 5 := by decide
theorem empty_6_45 : countEmpty rows_6 45 = 0 := by decide
theorem empty_6_46 : countEmpty rows_6 46 = 0 := by decide
theorem empty_6_47 : countEmpty rows_6 47 = 5 := by decide
theorem empty_6_48 : countEmpty rows_6 48 = 0 := by decide
theorem empty_6_49 : countEmpty rows_6 49 = 2 := by decide
theorem empty_6_50 : countEmpty rows_6 50 = 2 := by decide
theorem empty_6_51 : countEmpty rows_6 51 = 2 := by decide
theorem empty_6_52 : countEmpty rows_6 52 = 0 := by decide
theorem empty_6_53 : countEmpty rows_6 53 = 3 := by decide
theorem empty_6_54 : countEmpty rows_6 54 = 0 := by decide
theorem empty_6_55 : countEmpty rows_6 55 = 0 := by decide
theorem empty_6_56 : countEmpty rows_6 56 = 0 := by decide
theorem empty_6_57 : countEmpty rows_6 57 = 5 := by decide
theorem empty_6_58 : countEmpty rows_6 58 = 0 := by decide
theorem empty_6_59 : countEmpty rows_6 59 = 0 := by decide
theorem empty_6_60 : countEmpty rows_6 60 = 0 := by decide
theorem empty_6_61 : countEmpty rows_6 61 = 5 := by decide
theorem empty_6_62 : countEmpty rows_6 62 = 0 := by decide
theorem empty_6_63 : countEmpty rows_6 63 = 0 := by decide
theorem empty_6_64 : countEmpty rows_6 64 = 2 := by decide
theorem empty_6_65 : countEmpty rows_6 65 = 5 := by decide
theorem empty_6_66 : countEmpty rows_6 66 = 5 := by decide
theorem empty_6_67 : countEmpty rows_6 67 = 5 := by decide
theorem empty_6_68 : countEmpty rows_6 68 = 5 := by decide
theorem empty_6_69 : countEmpty rows_6 69 = 0 := by decide
theorem empty_6_70 : countEmpty rows_6 70 = 5 := by decide
theorem empty_6_71 : countEmpty rows_6 71 = 0 := by decide
theorem empty_7_0 : countEmpty rows_7 0 = 5 := by decide
theorem empty_7_1 : countEmpty rows_7 1 = 0 := by decide
theorem empty_7_2 : countEmpty rows_7 2 = 0 := by decide
theorem empty_7_3 : countEmpty rows_7 3 = 0 := by decide
theorem empty_7_4 : countEmpty rows_7 4 = 0 := by decide
theorem empty_7_5 : countEmpty rows_7 5 = 5 := by decide
theorem empty_7_6 : countEmpty rows_7 6 = 5 := by decide
theorem empty_7_7 : countEmpty rows_7 7 = 0 := by decide
theorem empty_7_8 : countEmpty rows_7 8 = 5 := by decide
theorem empty_7_9 : countEmpty rows_7 9 = 0 := by decide
theorem empty_7_10 : countEmpty rows_7 10 = 5 := by decide
theorem empty_7_11 : countEmpty rows_7 11 = 0 := by decide
theorem empty_7_12 : countEmpty rows_7 12 = 0 := by decide
theorem empty_7_13 : countEmpty rows_7 13 = 4 := by decide
theorem empty_7_14 : countEmpty rows_7 14 = 0 := by decide
theorem empty_7_15 : countEmpty rows_7 15 = 0 := by decide
theorem empty_7_16 : countEmpty rows_7 16 = 5 := by decide
theorem empty_7_17 : countEmpty rows_7 17 = 0 := by decide
theorem empty_7_18 : countEmpty rows_7 18 = 5 := by decide
theorem empty_7_19 : countEmpty rows_7 19 = 0 := by decide
theorem empty_7_20 : countEmpty rows_7 20 = 0 := by decide
theorem empty_7_21 : countEmpty rows_7 21 = 5 := by decide
theorem empty_7_22 : countEmpty rows_7 22 = 0 := by decide
theorem empty_7_23 : countEmpty rows_7 23 = 5 := by decide
theorem empty_7_24 : countEmpty rows_7 24 = 5 := by decide
theorem empty_7_25 : countEmpty rows_7 25 = 0 := by decide
theorem empty_7_26 : countEmpty rows_7 26 = 5 := by decide
theorem empty_7_27 : countEmpty rows_7 27 = 0 := by decide
theorem empty_7_28 : countEmpty rows_7 28 = 0 := by decide
theorem empty_7_29 : countEmpty rows_7 29 = 5 := by decide
theorem empty_7_30 : countEmpty rows_7 30 = 0 := by decide
theorem empty_7_31 : countEmpty rows_7 31 = 5 := by decide
theorem empty_7_32 : countEmpty rows_7 32 = 4 := by decide
theorem empty_7_33 : countEmpty rows_7 33 = 0 := by decide
theorem empty_7_34 : countEmpty rows_7 34 = 5 := by decide
theorem empty_7_35 : countEmpty rows_7 35 = 0 := by decide
theorem empty_7_36 : countEmpty rows_7 36 = 4 := by decide
theorem empty_7_37 : countEmpty rows_7 37 = 0 := by decide
theorem empty_7_38 : countEmpty rows_7 38 = 0 := by decide
theorem empty_7_39 : countEmpty rows_7 39 = 0 := by decide
theorem empty_7_40 : countEmpty rows_7 40 = 0 := by decide
theorem empty_7_41 : countEmpty rows_7 41 = 5 := by decide
theorem empty_7_42 : countEmpty rows_7 42 = 0 := by decide
theorem empty_7_43 : countEmpty rows_7 43 = 0 := by decide
theorem empty_7_44 : countEmpty rows_7 44 = 0 := by decide
theorem empty_7_45 : countEmpty rows_7 45 = 0 := by decide
theorem empty_7_46 : countEmpty rows_7 46 = 4 := by decide
theorem empty_7_47 : countEmpty rows_7 47 = 5 := by decide
theorem empty_7_48 : countEmpty rows_7 48 = 5 := by decide
theorem empty_7_49 : countEmpty rows_7 49 = 5 := by decide
theorem empty_7_50 : countEmpty rows_7 50 = 0 := by decide
theorem empty_7_51 : countEmpty rows_7 51 = 0 := by decide
theorem empty_7_52 : countEmpty rows_7 52 = 5 := by decide
theorem empty_7_53 : countEmpty rows_7 53 = 0 := by decide
theorem empty_7_54 : countEmpty rows_7 54 = 0 := by decide
theorem empty_7_55 : countEmpty rows_7 55 = 0 := by decide
theorem empty_7_56 : countEmpty rows_7 56 = 0 := by decide
theorem empty_7_57 : countEmpty rows_7 57 = 0 := by decide
theorem empty_7_58 : countEmpty rows_7 58 = 0 := by decide
theorem empty_7_59 : countEmpty rows_7 59 = 0 := by decide
theorem empty_7_60 : countEmpty rows_7 60 = 0 := by decide
theorem empty_7_61 : countEmpty rows_7 61 = 0 := by decide
theorem empty_7_62 : countEmpty rows_7 62 = 0 := by decide
theorem empty_7_63 : countEmpty rows_7 63 = 0 := by decide
theorem empty_7_64 : countEmpty rows_7 64 = 5 := by decide
theorem empty_7_65 : countEmpty rows_7 65 = 5 := by decide
theorem empty_7_66 : countEmpty rows_7 66 = 0 := by decide
theorem empty_7_67 : countEmpty rows_7 67 = 5 := by decide
theorem empty_7_68 : countEmpty rows_7 68 = 0 := by decide
theorem empty_7_69 : countEmpty rows_7 69 = 5 := by decide
theorem empty_7_70 : countEmpty rows_7 70 = 5 := by decide
theorem empty_7_71 : countEmpty rows_7 71 = 0 := by decide
theorem empty_7_72 : countEmpty rows_7 72 = 5 := by decide
theorem empty_7_73 : countEmpty rows_7 73 = 0 := by decide
theorem empty_8_0 : countEmpty rows_8 0 = 5 := by decide
theorem empty_8_1 : countEmpty rows_8 1 = 0 := by decide
theorem empty_8_2 : countEmpty rows_8 2 = 5 := by decide
theorem empty_8_3 : countEmpty rows_8 3 = 5 := by decide
theorem empty_8_4 : countEmpty rows_8 4 = 0 := by decide
theorem empty_8_5 : countEmpty rows_8 5 = 0 := by decide
theorem empty_8_6 : countEmpty rows_8 6 = 0 := by decide
theorem empty_8_7 : countEmpty rows_8 7 = 0 := by decide
theorem empty_8_8 : countEmpty rows_8 8 = 5 := by decide
theorem empty_8_9 : countEmpty rows_8 9 = 0 := by decide
theorem empty_8_10 : countEmpty rows_8 10 = 0 := by decide
theorem empty_8_11 : countEmpty rows_8 11 = 0 := by decide
theorem empty_8_12 : countEmpty rows_8 12 = 0 := by decide
theorem empty_8_13 : countEmpty rows_8 13 = 5 := by decide
theorem empty_8_14 : countEmpty rows_8 14 = 5 := by decide
theorem empty_8_15 : countEmpty rows_8 15 = 0 := by decide
theorem empty_8_16 : countEmpty rows_8 16 = 0 := by decide
theorem empty_8_17 : countEmpty rows_8 17 = 0 := by decide
theorem empty_8_18 : countEmpty rows_8 18 = 0 := by decide
theorem empty_8_19 : countEmpty rows_8 19 = 5 := by decide
theorem empty_8_20 : countEmpty rows_8 20 = 0 := by decide
theorem empty_8_21 : countEmpty rows_8 21 = 5 := by decide
theorem empty_8_22 : countEmpty rows_8 22 = 0 := by decide
theorem empty_8_23 : countEmpty rows_8 23 = 0 := by decide
theorem empty_8_24 : countEmpty rows_8 24 = 0 := by decide
theorem empty_8_25 : countEmpty rows_8 25 = 5 := by decide
theorem empty_8_26 : countEmpty rows_8 26 = 0 := by decide
theorem empty_8_27 : countEmpty rows_8 27 = 0 := by decide
theorem empty_8_28 : countEmpty rows_8 28 = 5 := by decide
theorem empty_8_29 : countEmpty rows_8 29 = 0 := by decide
theorem empty_8_30 : countEmpty rows_8 30 = 5 := by decide
theorem empty_8_31 : countEmpty rows_8 31 = 5 := by decide
theorem empty_8_32 : countEmpty rows_8 32 = 0 := by decide
theorem empty_8_33 : countEmpty rows_8 33 = 0 := by decide
theorem empty_8_34 : countEmpty rows_8 34 = 0 := by decide
theorem empty_8_35 : countEmpty rows_8 35 = 5 := by decide
theorem empty_8_36 : countEmpty rows_8 36 = 0 := by decide
theorem empty_8_37 : countEmpty rows_8 37 = 5 := by decide
theorem empty_9_0 : countEmpty rows_9 0 = 5 := by decide
theorem empty_9_1 : countEmpty rows_9 1 = 0 := by decide
theorem empty_9_2 : countEmpty rows_9 2 = 0 := by decide
theorem empty_9_3 : countEmpty rows_9 3 = 0 := by decide
theorem empty_9_4 : countEmpty rows_9 4 = 0 := by decide
theorem empty_9_5 : countEmpty rows_9 5 = 0 := by decide
theorem empty_9_6 : countEmpty rows_9 6 = 0 := by decide
theorem empty_9_7 : countEmpty rows_9 7 = 0 := by decide
theorem empty_9_8 : countEmpty rows_9 8 = 0 := by decide
theorem empty_9_9 : countEmpty rows_9 9 = 0 := by decide
theorem empty_9_10 : countEmpty rows_9 10 = 0 := by decide
theorem empty_9_11 : countEmpty rows_9 11 = 5 := by decide
theorem empty_9_12 : countEmpty rows_9 12 = 0 := by decide
theorem empty_9_13 : countEmpty rows_9 13 = 5 := by decide
theorem empty_9_14 : countEmpty rows_9 14 = 0 := by decide
theorem empty_10_0 : countEmpty rows_10 0 = 0 := by decide
theorem empty_10_1 : countEmpty rows_10 1 = 0 := by decide
theorem empty_10_2 : countEmpty rows_10 2 = 0 := by decide
theorem empty_10_3 : countEmpty rows_10 3 = 0 := by decide
theorem empty_10_4 : countEmpty rows_10 4 = 0 := by decide
theorem empty_10_5 : countEmpty rows_10 5 = 0 := by decide
theorem empty_10_6 : countEmpty rows_10 6 = 5 := by decide
theorem empty_10_7 : countEmpty rows_10 7 = 5 := by decide
theorem empty_10_8 : countEmpty rows_10 8 = 5 := by decide
theorem empty_10_9 : countEmpty rows_10 9 = 0 := by decide
theorem empty_10_10 : countEmpty rows_10 10 = 5 := by decide
theorem empty_11_0 : countEmpty rows_11 0 = 0 := by decide
theorem empty_11_1 : countEmpty rows_11 1 = 5 := by decide
theorem empty_11_2 : countEmpty rows_11 2 = 0 := by decide
theorem empty_11_3 : countEmpty rows_11 3 = 2 := by decide
theorem empty_11_4 : countEmpty rows_11 4 = 0 := by decide
theorem empty_11_5 : countEmpty rows_11 5 = 0 := by decide
theorem empty_11_6 : countEmpty rows_11 6 = 5 := by decide
theorem empty_11_7 : countEmpty rows_11 7 = 0 := by decide
theorem empty_11_8 : countEmpty rows_11 8 = 0 := by decide
theorem empty_11_9 : countEmpty rows_11 9 = 5 := by decide
theorem empty_11_10 : countEmpty rows_11 10 = 5 := by decide
theorem empty_11_11 : countEmpty rows_11 11 = 0 := by decide
theorem empty_11_12 : countEmpty rows_11 12 = 0 := by decide
theorem empty_11_13 : countEmpty rows_11 13 = 0 := by decide
theorem empty_11_14 : countEmpty rows_11 14 = 0 := by decide
theorem empty_11_15 : countEmpty rows_11 15 = 0 := by decide
theorem empty_11_16 : countEmpty rows_11 16 = 0 := by decide
theorem empty_11_17 : countEmpty rows_11 17 = 0 := by decide
theorem empty_11_18 : countEmpty rows_11 18 = 5 := by decide
theorem empty_11_19 : countEmpty rows_11 19 = 0 := by decide
theorem empty_11_20 : countEmpty rows_11 20 = 0 := by decide
theorem empty_11_21 : countEmpty rows_11 21 = 0 := by decide
theorem empty_11_22 : countEmpty rows_11 22 = 0 := by decide
theorem empty_11_23 : countEmpty rows_11 23 = 5 := by decide
theorem empty_11_24 : countEmpty rows_11 24 = 0 := by decide
theorem empty_11_25 : countEmpty rows_11 25 = 0 := by decide
theorem empty_11_26 : countEmpty rows_11 26 = 0 := by decide
theorem empty_11_27 : countEmpty rows_11 27 = 0 := by decide
theorem empty_11_28 : countEmpty rows_11 28 = 5 := by decide
theorem empty_11_29 : countEmpty rows_11 29 = 0 := by decide
theorem empty_11_30 : countEmpty rows_11 30 = 0 := by decide
theorem empty_11_31 : countEmpty rows_11 31 = 0 := by decide
theorem empty_11_32 : countEmpty rows_11 32 = 0 := by decide
theorem empty_11_33 : countEmpty rows_11 33 = 0 := by decide
theorem empty_11_34 : countEmpty rows_11 34 = 5 := by decide
theorem empty_11_35 : countEmpty rows_11 35 = 0 := by decide
theorem empty_11_36 : countEmpty rows_11 36 = 0 := by decide
theorem empty_11_37 : countEmpty rows_11 37 = 0 := by decide
theorem empty_11_38 : countEmpty rows_11 38 = 0 := by decide
theorem empty_11_39 : countEmpty rows_11 39 = 5 := by decide
theorem empty_11_40 : countEmpty rows_11 40 = 0 := by decide
theorem empty_11_41 : countEmpty rows_11 41 = 0 := by decide
theorem empty_11_42 : countEmpty rows_11 42 = 0 := by decide
theorem empty_11_43 : countEmpty rows_11 43 = 0 := by decide
theorem empty_11_44 : countEmpty rows_11 44 = 0 := by decide
theorem empty_11_45 : countEmpty rows_11 45 = 0 := by decide
theorem empty_11_46 : countEmpty rows_11 46 = 0 := by decide
theorem empty_11_47 : countEmpty rows_11 47 = 5 := by decide
theorem empty_11_48 : countEmpty rows_11 48 = 0 := by decide
theorem empty_11_49 : countEmpty rows_11 49 = 0 := by decide
theorem empty_11_50 : countEmpty rows_11 50 = 0 := by decide
theorem empty_11_51 : countEmpty rows_11 51 = 0 := by decide
theorem empty_11_52 : countEmpty rows_11 52 = 0 := by decide
theorem empty_11_53 : countEmpty rows_11 53 = 0 := by decide
theorem empty_11_54 : countEmpty rows_11 54 = 5 := by decide
theorem empty_11_55 : countEmpty rows_11 55 = 0 := by decide
theorem empty_11_56 : countEmpty rows_11 56 = 0 := by decide
theorem empty_11_57 : countEmpty rows_11 57 = 5 := by decide
theorem empty_11_58 : countEmpty rows_11 58 = 0 := by decide
theorem empty_11_59 : countEmpty rows_11 59 = 0 := by decide
theorem empty_11_60 : countEmpty rows_11 60 = 0 := by decide
theorem empty_11_61 : countEmpty rows_11 61 = 0 := by decide
theorem empty_11_62 : countEmpty rows_11 62 = 0 := by decide
theorem empty_11_63 : countEmpty rows_11 63 = 0 := by decide
theorem empty_11_64 : countEmpty rows_11 64 = 0 := by decide
theorem empty_11_65 : countEmpty rows_11 65 = 0 := by decide
theorem empty_11_66 : countEmpty rows_11 66 = 0 := by decide
theorem empty_11_67 : countEmpty rows_11 67 = 1 := by decide
theorem empty_11_68 : countEmpty rows_11 68 = 5 := by decide
theorem empty_11_69 : countEmpty rows_11 69 = 0 := by decide
theorem empty_11_70 : countEmpty rows_11 70 = 0 := by decide
theorem empty_11_71 : countEmpty rows_11 71 = 5 := by decide
theorem empty_11_72 : countEmpty rows_11 72 = 0 := by decide
theorem empty_11_73 : countEmpty rows_11 73 = 0 := by decide
theorem empty_11_74 : countEmpty rows_11 74 = 0 := by decide
theorem empty_11_75 : countEmpty rows_11 75 = 0 := by decide
theorem empty_11_76 : countEmpty rows_11 76 = 0 := by decide
theorem empty_11_77 : countEmpty rows_11 77 = 5 := by decide
theorem empty_11_78 : countEmpty rows_11 78 = 0 := by decide
theorem empty_11_79 : countEmpty rows_11 79 = 0 := by decide
theorem empty_11_80 : countEmpty rows_11 80 = 0 := by decide
theorem empty_11_81 : countEmpty rows_11 81 = 0 := by decide
theorem empty_11_82 : countEmpty rows_11 82 = 0 := by decide
theorem empty_11_83 : countEmpty rows_11 83 = 0 := by decide
theorem empty_11_84 : countEmpty rows_11 84 = 0 := by decide
theorem empty_11_85 : countEmpty rows_11 85 = 0 := by decide
theorem empty_11_86 : countEmpty rows_11 86 = 0 := by decide
theorem empty_11_87 : countEmpty rows_11 87 = 0 := by decide
theorem empty_11_88 : countEmpty rows_11 88 = 0 := by decide
theorem empty_11_89 : countEmpty rows_11 89 = 0 := by decide
theorem empty_11_90 : countEmpty rows_11 90 = 0 := by decide
theorem empty_11_91 : countEmpty rows_11 91 = 5 := by decide
theorem empty_11_92 : countEmpty rows_11 92 = 5 := by decide
theorem empty_11_93 : countEmpty rows_11 93 = 0 := by decide
theorem empty_11_94 : countEmpty rows_11 94 = 0 := by decide
theorem empty_11_95 : countEmpty rows_11 95 = 0 := by decide
theorem empty_11_96 : countEmpty rows_11 96 = 0 := by decide
theorem empty_11_97 : countEmpty rows_11 97 = 5 := by decide
theorem empty_12_0 : countEmpty rows_12 0 = 5 := by decide
theorem empty_12_1 : countEmpty rows_12 1 = 5 := by decide
theorem empty_12_2 : countEmpty rows_12 2 = 0 := by decide
theorem empty_12_3 : countEmpty rows_12 3 = 0 := by decide
theorem empty_12_4 : countEmpty rows_12 4 = 0 := by decide
theorem empty_12_5 : countEmpty rows_12 5 = 0 := by decide
theorem empty_12_6 : countEmpty rows_12 6 = 0 := by decide
theorem empty_12_7 : countEmpty rows_12 7 = 0 := by decide
theorem empty_12_8 : countEmpty rows_12 8 = 0 := by decide
theorem empty_12_9 : countEmpty rows_12 9 = 0 := by decide
theorem empty_12_10 : countEmpty rows_12 10 = 5 := by decide
theorem empty_12_11 : countEmpty rows_12 11 = 0 := by decide
theorem empty_12_12 : countEmpty rows_12 12 = 0 := by decide
theorem empty_12_13 : countEmpty rows_12 13 = 0 := by decide
theorem empty_12_14 : countEmpty rows_12 14 = 0 := by decide
theorem empty_12_15 : countEmpty rows_12 15 = 0 := by decide
theorem empty_13_0 : countEmpty rows_13 0 = 0 := by decide
theorem empty_13_1 : countEmpty rows_13 1 = 5 := by decide
theorem empty_13_2 : countEmpty rows_13 2 = 5 := by decide
theorem empty_13_3 : countEmpty rows_13 3 = 0 := by decide
theorem empty_13_4 : countEmpty rows_13 4 = 5 := by decide
theorem empty_13_5 : countEmpty rows_13 5 = 0 := by decide
theorem empty_13_6 : countEmpty rows_13 6 = 5 := by decide
theorem empty_13_7 : countEmpty rows_13 7 = 5 := by decide
theorem empty_14_0 : countEmpty rows_14 0 = 0 := by decide
theorem empty_14_1 : countEmpty rows_14 1 = 5 := by decide
theorem empty_14_2 : countEmpty rows_14 2 = 0 := by decide
theorem empty_14_3 : countEmpty rows_14 3 = 0 := by decide
theorem empty_14_4 : countEmpty rows_14 4 = 5 := by decide
theorem empty_14_5 : countEmpty rows_14 5 = 5 := by decide
theorem empty_14_6 : countEmpty rows_14 6 = 5 := by decide
theorem empty_14_7 : countEmpty rows_14 7 = 0 := by decide
theorem empty_14_8 : countEmpty rows_14 8 = 0 := by decide
theorem empty_14_9 : countEmpty rows_14 9 = 5 := by decide
theorem empty_15_0 : countEmpty rows_15 0 = 0 := by decide
theorem empty_15_1 : countEmpty rows_15 1 = 2 := by decide
theorem empty_15_2 : countEmpty rows_15 2 = 3 := by decide
theorem empty_15_3 : countEmpty rows_15 3 = 3 := by decide
theorem empty_15_4 : countEmpty rows_15 4 = 2 := by decide
theorem empty_15_5 : countEmpty rows_15 5 = 0 := by decide
theorem empty_15_6 : countEmpty rows_15 6 = 5 := by decide
theorem empty_15_7 : countEmpty rows_15 7 = 0 := by decide
theorem empty_15_8 : countEmpty rows_15 8 = 0 := by decide
theorem empty_16_0 : countEmpty rows_16 0 = 0 := by decide
theorem empty_16_1 : countEmpty rows_16 1 = 0 := by decide
theorem empty_16_2 : countEmpty rows_16 2 = 5 := by decide
theorem empty_16_3 : countEmpty rows_16 3 = 0 := by decide
theorem empty_16_4 : countEmpty rows_16 4 = 0 := by decide
theorem empty_16_5 : countEmpty rows_16 5 = 0 := by decide
theorem empty_16_6 : countEmpty rows_16 6 = 0 := by decide
theorem empty_16_7 : countEmpty rows_16 7 = 0 := by decide
theorem empty_16_8 : countEmpty rows_16 8 = 0 := by decide
theorem empty_16_9 : countEmpty rows_16 9 = 0 := by decide
theorem empty_16_10 : countEmpty rows_16 10 = 5 := by decide
theorem empty_16_11 : countEmpty rows_16 11 = 0 := by decide
theorem empty_16_12 : countEmpty rows_16 12 = 0 := by decide
theorem empty_16_13 : countEmpty rows_16 13 = 0 := by decide
theorem empty_16_14 : countEmpty rows_16 14 = 0 := by decide
theorem empty_16_15 : countEmpty rows_16 15 = 0 := by decide
theorem empty_16_16 : countEmpty rows_16 16 = 0 := by decide
theorem empty_16_17 : countEmpty rows_16 17 = 0 := by decide
theorem empty_16_18 : countEmpty rows_16 18 = 0 := by decide
theorem empty_16_19 : countEmpty rows_16 19 = 0 := by decide
theorem empty_16_20 : countEmpty rows_16 20 = 5 := by decide
theorem empty_16_21 : countEmpty rows_16 21 = 0 := by decide
theorem empty_16_22 : countEmpty rows_16 22 = 0 := by decide
theorem empty_16_23 : countEmpty rows_16 23 = 5 := by decide
theorem empty_16_24 : countEmpty rows_16 24 = 0 := by decide
theorem empty_16_25 : countEmpty rows_16 25 = 0 := by decide
theorem empty_17_0 : countEmpty rows_17 0 = 0 := by decide
theorem empty_17_1 : countEmpty rows_17 1 = 5 := by decide
theorem empty_17_2 : countEmpty rows_17 2 = 0 := by decide
theorem empty_17_3 : countEmpty rows_17 3 = 0 := by decide
theorem empty_17_4 : countEmpty rows_17 4 = 0 := by decide
theorem empty_17_5 : countEmpty rows_17 5 = 0 := by decide
theorem empty_17_6 : countEmpty rows_17 6 = 0 := by decide
theorem empty_17_7 : countEmpty rows_17 7 = 1 := by decide
theorem empty_17_8 : countEmpty rows_17 8 = 0 := by decide
theorem empty_17_9 : countEmpty rows_17 9 = 0 := by decide
theorem empty_17_10 : countEmpty rows_17 10 = 1 := by decide
theorem empty_17_11 : countEmpty rows_17 11 = 0 := by decide
theorem empty_17_12 : countEmpty rows_17 12 = 5 := by decide
theorem empty_17_13 : countEmpty rows_17 13 = 0 := by decide
theorem empty_17_14 : countEmpty rows_17 14 = 1 := by decide
theorem empty_17_15 : countEmpty rows_17 15 = 0 := by decide
theorem empty_17_16 : countEmpty rows_17 16 = 1 := by decide
theorem empty_17_17 : countEmpty rows_17 17 = 0 := by decide
theorem empty_17_18 : countEmpty rows_17 18 = 5 := by decide
theorem empty_17_19 : countEmpty rows_17 19 = 0 := by decide
theorem empty_17_20 : countEmpty rows_17 20 = 0 := by decide
theorem empty_17_21 : countEmpty rows_17 21 = 0 := by decide
theorem empty_17_22 : countEmpty rows_17 22 = 1 := by decide
theorem empty_17_23 : countEmpty rows_17 23 = 0 := by decide
theorem empty_17_24 : countEmpty rows_17 24 = 0 := by decide
theorem empty_17_25 : countEmpty rows_17 25 = 0 := by decide
theorem empty_17_26 : countEmpty rows_17 26 = 5 := by decide
theorem empty_17_27 : countEmpty rows_17 27 = 0 := by decide
theorem empty_17_28 : countEmpty rows_17 28 = 0 := by decide
theorem empty_17_29 : countEmpty rows_17 29 = 0 := by decide
theorem empty_17_30 : countEmpty rows_17 30 = 1 := by decide
theorem empty_17_31 : countEmpty rows_17 31 = 0 := by decide
theorem empty_17_32 : countEmpty rows_17 32 = 1 := by decide
theorem empty_17_33 : countEmpty rows_17 33 = 1 := by decide
theorem empty_17_34 : countEmpty rows_17 34 = 0 := by decide
theorem empty_17_35 : countEmpty rows_17 35 = 0 := by decide
theorem empty_17_36 : countEmpty rows_17 36 = 1 := by decide
theorem empty_17_37 : countEmpty rows_17 37 = 0 := by decide
theorem empty_17_38 : countEmpty rows_17 38 = 0 := by decide
theorem empty_17_39 : countEmpty rows_17 39 = 1 := by decide
theorem empty_17_40 : countEmpty rows_17 40 = 1 := by decide
theorem empty_17_41 : countEmpty rows_17 41 = 0 := by decide
theorem empty_17_42 : countEmpty rows_17 42 = 0 := by decide
theorem empty_18_0 : countEmpty rows_18 0 = 0 := by decide
theorem empty_18_1 : countEmpty rows_18 1 = 5 := by decide
theorem empty_18_2 : countEmpty rows_18 2 = 0 := by decide
theorem empty_18_3 : countEmpty rows_18 3 = 5 := by decide
theorem empty_18_4 : countEmpty rows_18 4 = 0 := by decide
theorem empty_18_5 : countEmpty rows_18 5 = 0 := by decide
theorem empty_18_6 : countEmpty rows_18 6 = 0 := by decide
theorem empty_18_7 : countEmpty rows_18 7 = 0 := by decide
theorem empty_18_8 : countEmpty rows_18 8 = 0 := by decide
theorem empty_18_9 : countEmpty rows_18 9 = 5 := by decide
theorem empty_18_10 : countEmpty rows_18 10 = 0 := by decide
theorem empty_18_11 : countEmpty rows_18 11 = 0 := by decide
theorem empty_18_12 : countEmpty rows_18 12 = 5 := by decide
theorem empty_18_13 : countEmpty rows_18 13 = 0 := by decide
theorem empty_19_0 : countEmpty rows_19 0 = 0 := by decide
theorem empty_19_1 : countEmpty rows_19 1 = 0 := by decide
theorem empty_19_2 : countEmpty rows_19 2 = 0 := by decide
theorem empty_19_3 : countEmpty rows_19 3 = 0 := by decide
theorem empty_19_4 : countEmpty rows_19 4 = 0 := by decide
theorem empty_19_5 : countEmpty rows_19 5 = 0 := by decide
theorem empty_19_6 : countEmpty rows_19 6 = 0 := by decide
theorem empty_19_7 : countEmpty rows_19 7 = 5 := by decide
theorem empty_19_8 : countEmpty rows_19 8 = 0 := by decide
theorem empty_19_9 : countEmpty rows_19 9 = 0 := by decide
theorem empty_19_10 : countEmpty rows_19 10 = 0 := by decide
theorem empty_19_11 : countEmpty rows_19 11 = 0 := by decide
theorem empty_19_12 : countEmpty rows_19 12 = 0 := by decide
theorem empty_19_13 : countEmpty rows_19 13 = 0 := by decide
theorem empty_19_14 : countEmpty rows_19 14 = 0 := by decide
theorem empty_19_15 : countEmpty rows_19 15 = 0 := by decide
theorem empty_19_16 : countEmpty rows_19 16 = 0 := by decide
theorem empty_19_17 : countEmpty rows_19 17 = 0 := by decide
theorem empty_19_18 : countEmpty rows_19 18 = 0 := by decide
theorem empty_19_19 : countEmpty rows_19 19 = 0 := by decide
theorem empty_19_20 : countEmpty rows_19 20 = 5 := by decide
theorem empty_19_21 : countEmpty rows_19 21 = 1 := by decide
theorem empty_19_22 : countEmpty rows_19 22 = 3 := by decide
theorem empty_19_23 : countEmpty rows_19 23 = 2 := by decide
theorem empty_19_24 : countEmpty rows_19 24 = 0 := by decide
theorem empty_19_25 : countEmpty rows_19 25 = 2 := by decide
theorem empty_19_26 : countEmpty rows_19 26 = 2 := by decide
theorem empty_19_27 : countEmpty rows_19 27 = 2 := by decide
theorem empty_19_28 : countEmpty rows_19 28 = 2 := by decide
theorem empty_19_29 : countEmpty rows_19 29 = 0 := by decide
theorem empty_19_30 : countEmpty rows_19 30 = 5 := by decide
theorem empty_19_31 : countEmpty rows_19 31 = 5 := by decide
theorem empty_19_32 : countEmpty rows_19 32 = 5 := by decide
theorem empty_19_33 : countEmpty rows_19 33 = 0 := by decide
theorem empty_19_34 : countEmpty rows_19 34 = 0 := by decide
theorem empty_19_35 : countEmpty rows_19 35 = 0 := by decide
theorem empty_19_36 : countEmpty rows_19 36 = 5 := by decide
theorem empty_19_37 : countEmpty rows_19 37 = 0 := by decide
theorem empty_19_38 : countEmpty rows_19 38 = 5 := by decide
theorem empty_19_39 : countEmpty rows_19 39 = 5 := by decide
theorem empty_19_40 : countEmpty rows_19 40 = 5 := by decide
theorem empty_19_41 : countEmpty rows_19 41 = 0 := by decide
theorem empty_19_42 : countEmpty rows_19 42 = 0 := by decide
theorem empty_19_43 : countEmpty rows_19 43 = 0 := by decide
theorem empty_19_44 : countEmpty rows_19 44 = 5 := by decide
theorem empty_19_45 : countEmpty rows_19 45 = 0 := by decide
theorem empty_19_46 : countEmpty rows_19 46 = 5 := by decide
theorem empty_19_47 : countEmpty rows_19 47 = 5 := by decide
theorem empty_19_48 : countEmpty rows_19 48 = 1 := by decide
theorem empty_19_49 : countEmpty rows_19 49 = 0 := by decide
theorem empty_19_50 : countEmpty rows_19 50 = 0 := by decide
theorem empty_19_51 : countEmpty rows_19 51 = 0 := by decide
theorem empty_19_52 : countEmpty rows_19 52 = 0 := by decide
