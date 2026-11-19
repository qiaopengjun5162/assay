# 数验 Assay

[![CI](https://github.com/qiaopengjun5162/assay/actions/workflows/ci.yml/badge.svg)](https://github.com/qiaopengjun5162/assay/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

> ## 政府开放数据「货不对板」榜
>
> 浙江 24,322 个数据集全查了一遍：页面写着 7 个字段，实际给你 92 列；标着「每年更新」，两年没动。
> 有问题的排成榜，每条结论你都能自己验——不用信我。

**打开榜单 → https://qiaopengjun5162.github.io/assay/**　不用注册、不用装东西。
本地跑：`just serve` → http://127.0.0.1:8080

---

## 30 秒看懂

榜上真实的一条，不是我编的例子：

**票务线上交易明细信息** · 南浔区文旅集团

| 平台页面上写着 | 实际点开拿到的 |
|---|---|
| 7 个字段 | **92 列** |
| 应该有人维护 | 其中 **46 列整列是空的** |
| 正常更新 | 平台自己标了 **停更** |

这类问题人工抽查覆盖不了，也不能让模型「读一遍页面」判断——实测模型会在 0 字节的文档上
编出「数据条数：382」。

所以这里只做三件事：**全查、排名、留证。**

## 三个数字

| 数字 | 含义 |
|---|---|
| **24,322** | 纳入体检的数据集，全量枚举，不是抽样 |
| **13,665**（56.2%）| 超过 90 天没更新；更新滞后中位数 115 天 |
| **1,284** | 字段清单和实际数据对不上；另有 **3,976** 个存在「整列全空」的列 |

## 凭什么信这个榜

不看代码也能验的三条：

1. **证据是原件，不是我的转述。** 每份数据的接口原始响应逐字节存盘、算 SHA-256；
   详情页有按钮，点了浏览器当场把文件取回来重算一遍，对不上就说明证据被动过。
2. **结论是数学题，不是我的话。** 「第 3 列恰好有 12 条空值」写成一条定理，交给 Lean 4
   内核穷举验算。改任何一位数字，内核立即报错。
3. **AI 只负责说人话。** 模型拿不到原始数据，只能看事实字典；输出里每个数字都要和事实
   逐个比对，写错就重写，仍不合格标记 `rejected`、页面不显示。

## 边界（写在最前面，避免误解）

- **不评价准确性**：没有权威数据源可比，评了就是编。
- **样例级**：完整性来自表头 + 5 行样例，不是全量 2.26 亿行。
- **证明证的是推导，不是数据真伪**：快照用 SHA-256 钉住时点，平台数据本身是否真实不在证明范围内。
- **覆盖是前 500 名**：证据 500 份（榜上每一条都能点开自己重算），证明 391 个——
  剩下 109 个数据集平台只给了表头、没有数据行，没有可证的命题。全量存证约 780MB，不进 git。

---

<details>
<summary><b>工程细节</b>：技术栈 / 命令 / 目录 / 约定（给开发者，普通读者到此为止）</summary>

## 技术栈

**一个 Rust 工作区**：采集、规则、证明生成、静态导出都在 Rust；前端是 Leptos + WASM；
验证器是 Lean 4 内核；模型只在「写人话解释」这一层出现，且输出受数字护栏约束。

比赛：第五元素 · 杭州 2026 AI × 数据创新赛｜赛道 02 AI 基础设施与可信数据

## 现状

| 项 | 状态 |
|---|---|
| 数据接口核验 | 完成：24,322 个数据集，8 线程 26 秒全量枚举，免登录 |
| 全量体检 | 完成：11.7 分钟，0 条详情失败 |
| 证据包 | 完成：**500 份**原始响应（榜上每一条都有），逐份重算 SHA-256 通过 |
| Lean 证明 | 完成：500 名里 391 个可证数据集、4,970 条定理，内核 PASS，全部基于已存证快照 |
| 单数据集报告 | 完成：500 份 Markdown，页面上可直接下载 |
| AI 解释层 | 完成：**500 条**解读（榜上全部），数字护栏 500/500 通过；首轮越界数字自动重写 |
| 前端 | 完成：Leptos + WASM，榜单 / 详情 / 证明 / 判定规则四视图，刷新可直达，手机可直接看 |
| 工程门禁 | 完成：fmt(Rust+TOML) / clippy / nextest / cargo-deny / lean / 密钥扫描 / 文档 lint |
| 周度巡检 | 完成：每周一全量体检，指标漂移自动开 PR（基线 + 报告），并往 `history/summary.json` 追加一条指标 |
| 语言构成 | Rust 全量（CLI / 库 / WASM 前端），Python 已移除；线上为 WASM 版 |

## 快速开始

```bash
# 0) 环境：Rust 1.88+、Lean 4（elan）、trunk
rustup target add wasm32-unknown-unknown
cargo install trunk --locked        # 或下载预编译二进制
cargo install cargo-nextest cargo-deny taplo-cli typos-cli just
./scripts/install-git-hooks.sh      # Conventional Commits 校验

# 1) 本地完整检查（不联网）：格式 + clippy + 测试 + 依赖审计 + 拼写 + 证明门禁
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
just explain 500    # 3) 解释层（需要 LLM key；先跑 just probe 验证配置）
just evidence 100   # 4) 证据包：原始响应 + SHA-256
just prove 100      # 5) 生成定理并跑 Lean 内核
just reports 100    # 6) 单数据集报告 → web/reports/*.md
just history        # 6b) 跑批历史：把本轮指标追加进 history/summary.json（补跑旧快照用 just history 2026-11-14）
just export 500     # 7) 导出 web/data.json（history 一并嵌进站点的曲线数据）
just web            # 8) 构建 WASM 前端 → crates/web/dist
just tamper         # 篡改演示：内核必须拒绝
just drift          # 周度漂移对比：拿基线比本轮 summary.json
just pack 2026-11-18  # 审计包：打成交付用单文件（含 shasum 清单）
```

`out-full/` 不入库（全量明细 8.7MB），换目录跑批用变量覆盖：`just out=out-smoke audit 8 200`。

**审计包**（`just pack <日期>`）产出 `dist/assay-audit-<日期>.tar.gz`，包内自带
`VERIFY.md` 与标准 `MANIFEST.sha256`。对方解压后一条命令验完全部文件，不需要装本工具：

```bash
tar xzf assay-audit-2026-11-18.tar.gz && cd assay-audit-2026-11-18
shasum -a 256 -c MANIFEST.sha256
```

同样输入两次打包字节一致，所以包本身可以当批次指纹；`dist/` 因此不入库。

周度巡检由 `.github/workflows/audit.yml` 每周一自动跑一遍全量体检，指标漂移就开 PR
（基线 + 报告），口径与阈值见 [docs/audit/README.md](docs/audit/README.md)。
同一趟会往 `history/summary.json` 追加一条**原始计数**（不记问题分：权重一改分数就不可比），
站点在两点以上时画成曲线，见 [D31](docs/DECISIONS.md)。

> **快照已冻结**：仓库里的榜单、证明、证据都对应 2026-11-14 这次抓取。
> `just all` 会用当天数据重跑，数字会漂移；路演/引用前不要执行，除非确实要刷新时点。

## 目录结构

```
assay/
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
│   ├── cli/                 # 二进制 assay（audit/report/…/reports/…/serve/tamper）
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

**LLM key / 端点 / 模型**：从环境变量或本机 Hermes 配置读，不打印、不入库。

| 变量 | 作用 | 例 |
|---|---|---|
| `ASSAY_LLM_KEY` | key | `sk-...` |
| `ASSAY_LLM_BASE` | 端点 | `https://platform.yeta.ai/v1` |
| `ASSAY_LLM_MODEL` | 模型名 | `deepseek-v4.1-flash` |
| `ASSAY_LLM_PROXY` | 代理 | `socks5h://127.0.0.1:7890` |

不设就走配置文件里的值，再不行才用兜底值。端点带 `/anthropic` 按 Anthropic messages 发，
其余按 OpenAI 兼容发——换渠道不用改代码。

配置对不对，一条命令就能验（只发一次请求，不跑批）：

```bash
assay explain --probe
# 端点   https://platform.yeta.ai/v1/chat/completions
# 模型   deepseek-v4.1-flash
# 协议   openai-chat
# key    TLDA90…Kmbi（来源：配置文件 ~/.hermes/profiles/ai/config.yaml）
# 代理   直连
# 响应   "可以"  1.9s
```

旧名 `DATAPROOF_LLM_KEY` 在过渡期仍可用（读到时会在 stderr 提示一次）。

## 产物

| 文件 | 内容 |
|---|---|
| `out-full/items.jsonl` | 24,322 条体检明细（不进 git，可重建） |
| `out-full/summary.json` | 聚合结论 + 2,500 个部门维度 |
| `out-full/leaderboard.md` | 问题 Top 50 |
| `out-full/leaderboard_dept.md` | 部门榜 Top 25 |
| `out-full/explanations.json` | 500 条 AI 解读 + 护栏结果 |
| `out-full/evidence.json` | 500 份证据包索引（含 SHA-256） |
| `out-full/proofs.json` | 391 个可证数据集的定理元信息 |
| `out-full/TopProofs.lean` | 4,970 条定理（391 个数据集），`lean` 退出码 0 |
| `web/data.json` | 前端数据（500 条明细） |
| `web/reports/*.md` | 前 500 名单数据集报告（字段差异、原始样例、证据哈希、AI 解读） |
| `crates/web/dist/` | 打包后的静态站 |

## 文档

- [docs/TEAM_DAY_ONE.md](docs/TEAM_DAY_ONE.md) — 新队友第一天怎么加入（10 分钟上手，先读这个）
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

---

</details>

## License

MIT © 2026 Paxon Qiao
