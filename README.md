# 数证 DataProof

给政府开放数据集做体检：**每个数字都带可机器复核的证明。**

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
| Lean 证明 | 完成：榜单前 20 名 689 条定理，内核 PASS |
| 静态前端 | 待做（`web/data.json` 已导出） |
| CI/CD | 本次落地：fmt / clippy / test / 证明校验 / 密钥扫描 |

## 快速开始

```bash
just check                 # 本地完整检查（格式、clippy、测试、证明）
just audit                 # 全量体检，约 12 分钟，产物在 out-full/
just report                # 聚合榜单
just prove                 # 给榜单前 20 名生成 Lean 并校验内核
just web                   # 导出前端数据
just serve                 # 本地预览 http://localhost:8080
```

只跑一小批，避免全量：

```bash
cd rust-core
cargo run --release --bin dataproof-core -- --limit 8 --out ../out
```

## 结构

```
rust-core/        Rust：采集、规则、Lean 生成、内核校验
  src/lib.rs      纯函数 + 单测（字段一致性分类、空值矩阵、定理生成）
  src/main.rs     单次体检
  src/bin/        full_audit（全量并发）、lean_from_fixture（黄金文件）
  fixtures/       脱敏样例快照 + expected.lean
scripts/          Python：聚合、榜单、证明生成、前端导出、全过程留痕
web/              静态站（读 data.json，无后端）
docs/             计划与决策记录
```

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
