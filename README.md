# 数证 DataProof

[![CI](https://github.com/qiaopengjun5162/dataproof/actions/workflows/ci.yml/badge.svg)](https://github.com/qiaopengjun5162/dataproof/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

给政府开放数据集做体检：**每个数字都带可机器复核的证明。**

> 在线演示：**https://qiaopengjun5162.github.io/dataproof/**（静态站，无后端）
> 本地：`just serve` → http://localhost:8080 · 路演脚本见 [docs/PITCH.md](docs/PITCH.md)

- 采集浙江开放平台 24,322 个数据集的元数据、字段定义、样例与更新时间
- 按 GB/T 36344-2018 的可计算维度打分：规范性、完整性、一致性、时效性、可访问性
- 数值结论自动编译成 Lean 4 定理，由内核校验；改一个数字，证明立刻报错
- 输出静态报告站，断网可演示

> 比赛：第五元素 · 杭州 2026 AI × 数据创新赛｜赛道 02 AI 基础设施与可信数据
> 施工图见 [`docs/PLAN.md`](docs/PLAN.md)，取舍与踩坑见 [`docs/DECISIONS.md`](docs/DECISIONS.md)

## 现状（2026-11-14）

| 项 | 状态 |
|---|---|
| 数据接口核验 | 完成：24,322 个数据集 77 秒全量枚举，免登录 |
| 全量体检 | 完成：11.7 分钟，0 条详情失败 |
| Lean 证明 | 完成：榜单前 100 名 2,355 条定理，内核 PASS，全部基于已存证快照 |
| 静态前端 | 完成：榜单 / 详情 / 证明三页，无后端，断网可演示 |
| AI 解释层 | 完成：100 条解读，数字护栏 100/100 通过（越界即重写） |
| CI/CD | 本次落地：fmt / clippy / test / 证明校验 / 密钥扫描 / 护栏单测 |

## 快速开始

```bash
just check     # 本地完整检查：fmt / clippy 零容忍 / 单测 / 黄金文件 / 证明门禁
just smoke     # 200 条端到端自检，约 40 秒，产物在 out-smoke/（不覆盖正式产物）
just all       # 全流水线：体检 → 榜单 → 证据 → 证明 → 静态站，约 13 分钟
just serve     # 本地预览 http://localhost:8080
```

拆开跑（`all` 就是这五步）：

```bash
just audit 8 0     # 1) 全量体检；参数 = 线程数、条数上限（0 = 全部）。约 12 分钟
just report        # 2) 聚合榜单 -> summary.json / leaderboard.md
just explain 100   # 3) 模型写解读，每个数字必须来自事实（要约 1 分钟，需 API）
just evidence 100  # 4) 落盘前 100 名的接口原始响应 + SHA-256
just prove 100     # 5) 生成 2,355 条定理并跑 Lean 内核（读存证快照）
just web           # 6) 数据 + 解读 + 证明 + 证据发布到 web/
```

`out-full/` 不入库（`.gitignore` 里排除，全量明细 8.7MB）。换目录跑批用变量覆盖：
`just out=out-smoke audit 8 200`。

> **快照已冻结**：仓库里的榜单、证明、证据都对应 2026-11-14 这次抓取。
> `just all` 会用当天数据重跑，数字会漂移；路演/引用前不要执行，除非确实要刷新时点。

## 结构

```
rust-core/        Rust：采集、规则、Lean 生成、内核校验
  src/lib.rs      纯函数 + 单测（字段一致性分类、空值矩阵、定理生成）
  src/main.rs     单次体检
  src/bin/        full_audit（全量并发）、lean_from_fixture（黄金文件）
  fixtures/       脱敏样例快照 + expected.lean
scripts/          Python：聚合、榜单、证明生成、前端导出、全过程留痕
web/              静态站（读 data.json，无后端）
docs/             计划、决策、路演稿、项目说明
  PLAN.md           里程碑与完成状态
  DECISIONS.md      D1–D10 取舍记录
  PITCH.md          6 分钟路演脚本 + 五问应答
  SUBMISSION.md     一页版项目说明（提交/评审用）
  architecture.svg  架构图
```

## 文档

- [项目说明（一页版）](docs/SUBMISSION.md) · [架构图](docs/architecture.svg)
- [6 分钟路演脚本](docs/PITCH.md) · [里程碑](docs/PLAN.md) · [决策记录](docs/DECISIONS.md)

## 结果快照（不用重跑也能看）

`data/` 下是 2026-11-14 全量跑批的聚合结果：`summary.json`（含全部部门分布）、
`leaderboard.md`（问题 Top 50）、`leaderboard_dept.md`（部门视角）。
逐条明细 `out-full/items.jsonl`（8.7MB）未入库，跑 `just audit` 可重建。

## 数据来源

浙江省公共数据开放平台 `data.zjzwfw.gov.cn`：

```
POST /dopServer/dataCatalog/dataQueryPage         目录（current/size 分页）
POST /dopServer/dataCatalog/catalogDetail/{id}    字段定义
POST /dopServer/dataCatalog/catalogDataPreview/{id}  表头 + 5 行样例
```

全部免登录。抓取时间 2026-11-14。平台接口可能变更，快照落盘后不依赖实时接口。

## 边界（写在最前面，避免误解）

- **不评价准确性**：没有权威数据源可比，评了就是编
- **样例级**：完整性来自表头 + 5 行样例，不是全量 2.26 亿行
- **证明证的是推导，不是数据真伪**：快照用 SHA-256 钉住时点，平台数据本身是否真实不在证明范围内

## License

MIT
