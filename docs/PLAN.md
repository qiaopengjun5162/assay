# 数证 DataProof · 项目计划

> 第五元素 · 杭州 2026 AI × 数据创新赛｜赛道 02 AI 基础设施与可信数据
> 报名：2026-11-25 截止｜比赛：2026-12-04 ~ 12-08
> 本文件是施工图，不是介绍文。介绍文见 `notes/tech/ai/2026-11-14-dataproof-idea.md`。

---

## 1. 目标与边界

**要做的东西**：一个可在一台笔记本上跑完的开放数据体检器。

```
输入：浙江开放平台的 24,322 个数据集
过程：确定性采集 → 规则判定 → Lean 证明 → LLM 解释（可关）
输出：健康榜 + 单数据集报告 + 可机器复核的证明文件
```

**明确不做**：

- 不处理全量 2.26 亿行记录（只做元数据 + 5 行样例 + 更新时间）
- 不做「准确性」评价（没有权威数据源可比对，硬做就是编）
- 不做账号体系、不做后端服务、不做数据库
- 不在现场依赖外网 API（DeepSeek 能力做成可关闭增强项）

**验收标准**（12/8 路演前必须成立）：

1. `just audit` 一条命令跑完全量，产出报告
2. `just verify` 跑完 Lean 内核校验，退出码 0
3. 打开网页能看到榜单、点进任意数据集看到五维报告、点「查看证明」看到 PASS
4. 断网状态下仍能演示（用已缓存的快照）

---

## 2. 指标口径：对齐国标，不自创维度

第一版我写了「完整性/时效性/一致性/可用性/可理解性」，方向对，但口径是自创的。改成对齐 **GB/T 36344-2018《信息技术 数据质量评价指标》** 的六维框架：

| 国标维度 | 本项目的可计算实现 | 做不到的部分 |
|---|---|---|
| 规范性 | 字段中文名缺失、字典缺失、命名不一致 | 是否符合行业数据标准 |
| 完整性 | 样例空值率、整列全空、无字段清单 | 全量缺失率（只做样例级） |
| 准确性 | — | **不评价**，没有权威源可比 |
| 一致性 | 声明字段清单 vs 实际样例表头 | 跨系统、跨时间一致性 |
| 时效性 | 最后更新时间滞后天数、声称更新频率是否兑现 | 记录级时间戳（平台不提供） |
| 可访问性 | 有无预览、有无字段定义、有无下载/接口 | 真实接口可用性（需登录） |

同时引用国家数据局《高质量数据集建设指引》的「说明文档完整性」思路：平台给的元数据本身是不是完整，也是被评价对象。

**这句话要写进讲稿**：我们只做能算的，算不了的明确写「不评价」。这比给一个 87 分更可信。

---

## 3. 借鉴清单（来源 → 拿什么 → 落到哪）

### 3.1 自己的仓库

| 来源 | 拿什么 | 落到本项目 |
|---|---|---|
| `Rust/rust-template` | `deny.toml`、`.pre-commit-config.yaml`、`.rustfmt.toml`、`Makefile` 思路、`cliff.toml` | 直接复制，改成 Justfile 驱动 |
| `Rust/moonpub` | CI 六件事：fmt / clippy -D warnings / nextest / cargo-audit / **gitleaks 密钥扫描** / MSRV；`Justfile`；`scripts/coverage.sh` 覆盖率地板 | 抄成 `.github/workflows/ci.yml`；密钥扫描必留（我们脚本读 API key） |
| `Rust/moonpub` | release-please + tag 触发 release 的双 workflow 结构（含 GITHUB_TOKEN 不触发下游的坑注释） | 决赛前打 v0.1.0 时用；不阻塞开发 |
| `Web3/kite-rust-axum-template` | axum 分层（router/service/config） | 只在需要 HTTP 时启用；当前静态站不需要 |
| `Lean/ZkProof` | Lean 项目骨架、`lakefile` 组织方式 | 证明模块的目录约定 |
| `Rust/taiyin-guqin` | Rust 核心 + Next.js 前端的构建顺序（先 wasm/lib 后前端） | 前端构建顺序 |

### 3.2 外部优质项目

| 来源 | 拿什么 | 怎么用 |
|---|---|---|
| `yamafaktory/formal`（Lean 4 证明服务，25★） | **三条硬规则**：① 拒绝含 `sorry` 的证明 ② 审计每条被接受证明依赖的公理 ③ 只缓存内核通过的证明，失败永不缓存 | 我们的 Lean 校验器照做：`grep -q sorry` 直接 fail；输出 `#print axioms` 审计；PASS 才写 `VERIFY.txt` |
| `fivetran/great_expectations` | 「期望（expectation）」的表达方式：每条检查是独立、可命名、可解释的断言 | 规则引擎的每条检查要有 id + 人话描述 + 计算式 |
| `sodadata/soda-core` | YAML 声明式检查，人能读、能 review | 后续支持 `checks.yaml`，让数据提供方自己写检查 |
| `DQOPS`（约 150 个内置检查） | 内置检查目录，按维度分组 | 我们首批 12 条检查按国标六维分组，不贪多 |
| `Elementary` | 用历史做异常检测（volume/freshness anomalies） | v2 再加；5 天内不做时间序列模型 |
| `axodotdev/cargo-dist` | 跨平台二进制打包 + release 资产 | 打 v0.1.0 时再上；现场用 `cargo build --release` 即可 |

**不借鉴的**：Soda 的 Elastic License（我们保持 MIT/Apache-2.0 兼容）、任何需要 Spark 的重量级方案。

---

## 4. 仓库结构

```
dataproof/
├── Cargo.toml                # workspace（Milestone 2 拆 crate）
├── Justfile                  # 唯一入口：audit / verify / web / check / fmt
├── rust-toolchain.toml       # 固定工具链，CI 与本机一致
├── .rustfmt.toml             # 抄 rust-template
├── deny.toml                 # 抄 rust-template
├── .pre-commit-config.yaml   # 抄 rust-template + gitleaks
├── .github/workflows/
│   ├── ci.yml                # rust job + lean job + secret-scan job
│   └── release.yml           # tag → 构建二进制（决赛前）
├── fixtures/                 # 固定快照，测试不碰网络
│   ├── snapshot.json         # 3 个数据集的真实样例（已脱敏）
│   └── expected.lean         # 黄金文件：生成的 Lean 应与之一致
├── src/
│   ├── lib.rs                # 纯函数：空值矩阵 / 定理生成 / 字段一致性分类（带单测）
│   ├── main.rs               # 单次体检 CLI（demo 用）
│   └── bin/full_audit.rs     # 全量并发体检 CLI
├── scripts/
│   ├── aggregate.py          # 聚合榜单
│   ├── scoring.py            # 评分与分类（与 Rust 保持同口径）
│   ├── proof_top.py          # 榜单前 N 名生成 Lean
│   ├── export_web.py         # 导出静态 web/data.json
│   ├── check_proofs.sh       # 拒绝 sorry / 审计公理 / 只接受 PASS
│   └── log_step.py           # 全过程留痕
├── web/                      # 静态站（读 data.json，无后端）
│   ├── data.json
│   └── index.html
├── out-full/                 # 运行产物（gitignore，CI 用 fixtures）
└── docs/
    ├── PLAN.md               # 本文件
    └── DECISIONS.md          # 关键取舍与踩坑记录
```

**为什么不做后端**：静态 JSON + 静态页，部署成本为零，断网也能演示。5 天里省掉一整天。

---

## 5. CI/CD

### `ci.yml`（每次 push / PR）

| job | 内容 | 为什么必须有 |
|---|---|---|
| `rust` | `cargo fmt --check` / `clippy -D warnings` / `cargo test` / `cargo build --release` | 主干质量；clippy 零容忍（抄 moonpub） |
| `lean` | 装 elan → `lean fixtures/expected.lean` → 校验退出码 0；再跑 `scripts/check_proofs.sh` 检查无 `sorry` | 证明是本项目的差异点，它坏了等于产品坏了 |
| `secret-scan` | gitleaks（fetch-depth: 0） | 脚本要读 API key，防止误提交 |
| `scripts` | `python3 -m py_compile scripts/*.py` | 聚合脚本是交付链的一环，别在赛场才发现语法错 |

**不做**：MSRV（单人项目，工具链固定）、覆盖率地板（5 天内收益低）、Windows 冒烟（现场是 macOS）。

### `release.yml`（打 tag 时）

抄 moonpub 的结构，并在注释里保留那个坑：**release-please 用 GITHUB_TOKEN 创建的 release 不会触发下游 workflow**，需要手动 `gh workflow run` 或用 PAT。

---

## 6. 里程碑

### 准备期（11/14 – 11/25，报名审核中）

- [x] 可行性核验：接口、字段、样例、官方是否已做
- [x] 单次体检链路（Rust）+ Lean 端到端
- [x] 全量 24,322 条跑通，出榜单
- [ ] **M1 工程骨架**：Justfile / CI / fixtures / 单测（本次）
- [ ] **M2 前端三页**：榜单 / 详情 / 证明（静态站）
- [ ] 用 DeepSeek `read_url` 做存证原型（可选）
- [ ] 5 分钟讲稿 + 3 分钟 demo 脚本

### 现场（12/04 – 12/08）

| 日期 | 任务 | 完成标志 |
|---|---|---|
| 12/04 | 组队、分工、确认现场网络与投影 | 一页分工表 |
| 12/05 上午 | 重跑全量、固化快照 | 新 `out-full/`，`just verify` PASS |
| 12/05 下午 | 前端接真实数据、补证明页 | 三页可点通 |
| 12/06 | LLM 解释层 + 打磨（可关） | 开关关闭时功能完整 |
| 12/07 | 排练 5 遍、准备降级方案 | 断网也能演 |
| 12/08 | 路演 | — |

**降级顺序**（出问题时按此砍）：LLM 解释 → 证明页动画 → 榜单筛选 → 只保留「榜单 + 单条详情」。

---

## 7. 测试策略

| 层 | 手段 | 网络 |
|---|---|---|
| 纯逻辑 | `src/lib.rs` 单测：空值矩阵、定理字符串、字段一致性分类 | 无 |
| 黄金文件 | `fixtures/snapshot.json` → 生成 Lean → 与 `fixtures/expected.lean` 逐字比对 | 无 |
| 证明 | CI 跑 `lean fixtures/expected.lean` + `check_proofs.sh`（拒 `sorry`、审公理） | 无 |
| 采集 | 手工冒烟：`--limit 20`，断言 0 失败 | 有（不进 CI） |

**原则**：CI 永不访问政府网站。数据快照进仓库，只存脱敏后的 5 行样例。

---

## 8. 风险

| 风险 | 触发信号 | 应对 |
|---|---|---|
| 平台接口变更 | 全量报错率 > 5% | 已缓存快照兜底；README 写清接口版本与抓取日期 |
| 现场网络不通 | 连不上 data.zjzwfw.gov.cn | 本地快照 + 静态站，全程离线可演 |
| Lean 现场编译失败 | `lean` 不在 PATH | 预编译 + 提供 `VERIFY.txt`；讲稿里先演示「改数字报错」 |
| 被问「准确性怎么不评」 | 评委追问 | 正面回答：没有权威源，评了就是编；给出接入权威源的路线 |
| 5 天做不完 | 12/06 前端未通 | 按降级顺序砍，保主干 |
| 需求质疑（没人付费） | 评委问商业模式 | 定位为交易所上架审核工具，不做商业假设 |

---

## 9. 交付物

```
仓库：可复现的 Rust + Lean + 静态前端
文档：README（怎么跑）、PLAN（怎么想）、DECISIONS（为什么这么选）
数据：out-full/items.jsonl + summary.json + leaderboard.md
证明：TopProofs.lean + VERIFY.txt + proofs.md
文章素材：notes/tech/ai/2026-11-14-dataproof-build-log.md（全过程留痕）
```
