# 数证 DataProof

[![CI](https://github.com/qiaopengjun5162/dataproof/actions/workflows/ci.yml/badge.svg)](https://github.com/qiaopengjun5162/dataproof/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

给政府开放数据做体检，并让每一条数值结论都能被机器独立复核。

**一个 Rust 工作区**：采集、规则、证明生成、静态导出都在 Rust；前端是 Leptos + WASM；
验证器是 Lean 4 内核；模型只在「写人话解释」这一层出现，且输出受数字护栏约束。

> 在线演示：**https://qiaopengjun5162.github.io/dataproof/**（静态站，无后端）
> 本地：`just serve` → http://127.0.0.1:8080
> 比赛：第五元素 · 杭州 2026 AI × 数据创新赛｜赛道 02 AI 基础设施与可信数据

## 它回答什么问题

一个开放数据集，页面写着「9 个字段、每年更新」，实际下下来是 8 列、两年没动过。
这类问题靠人工抽查覆盖不了，也不能靠模型「读一遍页面」核实——实测模型会在 0 字节文档上
编出「数据条数：382」。

所以这里只做三件可验证的事：

1. **确定性体检**：直接调平台接口，逐条比对声明字段与样例字段，统计空值、滞后、停更。
2. **可机器复核**：把「第 N 列空值恰好 K 条」写成 Lean 4 定理，`by decide` 由内核穷举验证。
   改任何一位数字，内核立即报错。
3. **可追溯**：问题最多的前 100 名，接口原始响应逐字节落盘并算 SHA-256；证明读的就是这份快照。

## 现状

| 项 | 状态 |
|---|---|
| 数据接口核验 | 完成：24,322 个数据集，8 线程 26 秒全量枚举，免登录 |
| 全量体检 | 完成：11.7 分钟，0 条详情失败 |
| Lean 证明 | 完成：前 100 名 2,355 条定理，内核 PASS，全部基于已存证快照 |
| 证据包 | 完成：100 份原始响应，SHA-256 可复核 |
| 单数据集报告 | 完成：前 100 名 Markdown，页面上可直接下载 |
| AI 解释层 | 完成：100 条解读，数字护栏 100/100 通过 |
| 前端 | 完成：Leptos + WASM，榜单 / 详情 / 证明三视图，刷新可直达 |
| 语言构成 | Rust 全量（CLI / 库 / WASM 前端），Python 已移除；线上为 WASM 版 |

## 快速开始

```bash
# 0) 环境：Rust 1.85+、Lean 4（elan）、trunk
rustup target add wasm32-unknown-unknown
cargo install trunk --locked        # 或下载预编译二进制
cargo install cargo-nextest cargo-deny taplo-cli typos-cli just
./scripts/install-git-hooks.sh      # Conventional Commits 校验

# 1) 本地完整检查（不联网）：格式 + clippy + 测试 + 依赖审计 + 证明门禁
just check
just clippy-wasm

# 2) 小样本端到端自检（联网，约 90 秒）：200 条 → 榜单 → 解读 → 证据 → 证明
just smoke

# 3) 全量跑批（联网，约 14 分钟）
just all

# 4) 本地预览
just serve
```

拆开跑（`all` 就是这八步）：

```bash
just audit 8 0      # 1) 体检：线程数 8，0 = 全部
just report         # 2) 聚合榜单 → summary.json / leaderboard.md
just explain 100    # 3) 解释层（需要 LLM key，见下）
just evidence 100   # 4) 证据包：原始响应 + SHA-256
just prove 100      # 5) 生成定理并跑 Lean 内核
just reports 100    # 6) 单数据集报告 → web/reports/*.md
just export 500     # 7) 导出 web/data.json
just web            # 8) 构建 WASM 前端 → crates/web/dist
just tamper         # 篡改演示：内核必须拒绝
```

`out-full/` 不入库（全量明细 8.7MB），换目录跑批用变量覆盖：`just out=out-smoke audit 8 200`。

> **快照已冻结**：仓库里的榜单、证明、证据都对应 2026-11-14 这次抓取。
> `just all` 会用当天数据重跑，数字会漂移；路演/引用前不要执行，除非确实要刷新时点。

## 目录结构

```
dataproof/
├── Cargo.toml               # workspace：core / cli / web
├── Justfile                 # 唯一入口，本地与 CI 跑同一套命令
├── crates/
│   ├── core/                # 库：所有可复用逻辑
│   │   ├── api.rs           #   平台三个接口 + 原始字节抓取
│   │   ├── audit.rs         #   并发体检 → items.jsonl + catalog_snapshot.json
│   │   ├── scoring.rs       #   字段一致性三分类 + 问题分
│   │   ├── matrix.rs        #   空值矩阵（0/1 位图、列计数、空值率）
│   │   ├── lean.rs          #   Lean 模块与定理生成
│   │   ├── proof.rs         #   快照 → 定理 → lean → proofs.json
│   │   ├── evidence.rs      #   原始响应落盘 + SHA-256
│   │   ├── aggregate.rs     #   聚合：summary.json / 两张榜单
│   │   ├── export.rs        #   导出 web/data.json
│   │   ├── explain.rs       #   解释层：模型写话 + 重试
│   │   ├── single.rs        #   单数据集报告（Markdown）
│   │   ├── guard.rs         #   数字护栏
│   │   ├── config.rs        #   模型 key 读取（不落盘、不打印）
│   │   ├── model.rs         #   数据模型（serde camelCase）
│   │   └── util.rs          #   日期、四舍五入、文件名安全化
│   ├── cli/                 # 二进制 dataproof（audit/report/…/reports/…/serve/tamper）
│   └── web/                 # Leptos + WASM 前端（trunk 打包）
├── web/                     # 数据产物：data.json / evidence/ / reports/ / TopProofs.lean
├── data/                    # 聚合结果快照（可离线核对）
├── fixtures/                # 黄金文件（CI 用，不联网）
├── ci/                      # 证明门禁脚本
└── docs/                    # 架构、需求、决策、路演稿
```

## 关键约定

**指标口径**：对齐 GB/T 36344-2018 六维（规范性、完整性、准确性、一致性、时效性、可访问性）。
只实现可自动计算的维度；准确性明确写「不评价」。

**字段一致性分三类**（第一版最大的坑）：

| 分类 | 含义 | 是否算问题 |
|---|---|---|
| 列数不一致 | 声明 9 列、样例 8 列 | 是 |
| 字段集合不一致 | 列数相同、名字对不上且非前缀 | 是 |
| 仅命名不一致 | 声明名 = 部门前缀 + 样例名 | 否 |

**问题分只用于排序**，不是科学指标；权重写在 `scoring.rs` 里，可对着代码复核。

**模型的位置**：只在解释层。它拿不到原始数据，只能看事实字典；输出里的每个数字都要和事实
逐个比对，最多 3 个；不合格重写一次，仍不合格标记 `rejected`、页面不展示。

**LLM key**：从 `DATAPROOF_LLM_KEY` 环境变量或本机 Hermes 配置读，不打印、不入库。
没有 key 时 `explain` 给出可操作的报错。

## 产物

| 文件 | 内容 |
|---|---|
| `out-full/items.jsonl` | 24,322 条体检明细（不进 git，可重建） |
| `out-full/summary.json` | 聚合结论 + 2,500 个部门维度 |
| `out-full/leaderboard.md` | 问题 Top 50 |
| `out-full/leaderboard_dept.md` | 部门榜 Top 25 |
| `out-full/explanations.json` | 100 条 AI 解读 + 护栏结果 |
| `out-full/evidence.json` | 100 份证据包索引（含 SHA-256） |
| `out-full/proofs.json` | 100 个数据集的定理元信息 |
| `out-full/TopProofs.lean` | 2,355 条定理，`lean` 退出码 0 |
| `web/data.json` | 前端数据（500 条明细） |
| `web/reports/*.md` | 前 100 名单数据集报告（字段差异、原始样例、证据哈希、AI 解读） |
| `crates/web/dist/` | 打包后的静态站 |

## 文档

- [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) — 需求、范围、验收标准、指标定义
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — 分层、数据流、模块契约、不变量
- [docs/DECISIONS.md](docs/DECISIONS.md) — 关键技术决策与理由
- [docs/PITCH.md](docs/PITCH.md) — 6 分钟路演稿
- [docs/SUBMISSION.md](docs/SUBMISSION.md) — 一页版提交材料
- [docs/HANDOFF.md](docs/HANDOFF.md) — 给接手的 AI/开发者的工作规则

## 数据来源

浙江省公共数据开放平台 `data.zjzwfw.gov.cn`：

```
POST /dopServer/dataCatalog/dataQueryPage            目录（current/size 分页）
POST /dopServer/dataCatalog/catalogDetail/{id}       字段定义
POST /dopServer/dataCatalog/catalogDataPreview/{id}  表头 + 5 行样例
```

全部免登录。抓取时间 2026-11-14。平台接口可能变更，快照落盘后不依赖实时接口。

## 边界（写在最前面，避免误解）

- **不评价准确性**：没有权威数据源可比，评了就是编
- **样例级**：完整性来自表头 + 5 行样例，不是全量 2.26 亿行
- **证明证的是推导，不是数据真伪**：快照用 SHA-256 钉住时点，平台数据本身是否真实不在证明范围内

## License

MIT © 2026 Paxon Qiao
