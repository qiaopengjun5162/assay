# AGENTS.md

数验 Assay 是纯 Rust 工作区：对公共开放数据做确定性体检，用 Lean 4 定理证明数值结论，
用原始响应 + SHA-256 存证。前端是 Leptos + WASM 静态站，没有后端。

本文件写项目-specific 的架构边界和排障规则。通用的分支、提交、文档先行、PR、生产边界等
元规则见 [docs/ENGINEERING_AGENT_GUARDAILS_ZH.md](docs/ENGINEERING_AGENT_GUARDAILS_ZH.md)。

新增或改动文件前先读：

- 本文件
- `CLAUDE.md`
- `docs/ENGINEERING_LESSONS_ZH.md`（排障前必搜）
- 目标模块对应的文档或 `docs/ARCHITECTURE.md` 章节

相关工程文件：

- `.github/PULL_REQUEST_TEMPLATE.md`：PR 模板（含口径与凭据检查）
- `.github/CODEOWNERS`：owner 规则
- `.markdownlint-cli2.yaml` + `scripts/lint-docs.sh`：文档 lint
- `.githooks/commit-msg` + `scripts/install-git-hooks.sh`：Conventional Commits 检查
- `_typos.toml`：拼写检查豁免
- `docs/ENGINEERING_LESSONS_ZH.md`：工程经验库

## 沟通

- 默认中文。
- 先验证真实状态，再更新进度或下结论；不要只信文档自述。
- 改动后同步更新相关的 README / ARCHITECTURE / DECISIONS / HANDOFF / PROGRESS。
- 不要读取、打印或提交真实凭据。

## 项目不变量

1. **字段分类只有一处实现**（`scoring::classify`）。复制到别处必分叉。
2. **证明对象 = 存证对象**：证的是已落盘并算过哈希的 `preview.json`，不是现场重抓的新数据。
3. **哈希只算原始字节**，走 `api::post_raw`。
4. **基准日 `NOW_YMD` 是常量**，保证今天跑和明天跑结论相同。
5. **报告、证明、证据同一选择口径**（`has_preview == 1` + `score` 降序）。**报告集合 ==
   证据集合**（500/500）；**证明是其中的子集**（391，`proof ⊆ evidence`）——差的 109 个
   是平台只给表头、没有数据行的，没有可证的命题。这三个关系有实测脚本可复核；
   别把「证明少」当成漏做，也别让报告多出证据没有的行。见 D16 / D23。
6. **准确性明确不评价**：没有权威源，评了就是编。
7. **样例级**：完整性来自表头 + 5 行样例，不是全量 2.26 亿行；文案不许说成全量。
8. **基线只在漂移 PR 合并时前进**：`baseline/summary.json` 是「上次人工确认过的常态」。
   跑批被判不可信（`assay drift` 退出码 2）时绝不写基线——脏数据进对照系，之后所有对比全废。
   见 `docs/DECISIONS.md` D19。
9. **榜单行只有一个来源**（`export::row` → `light_row`）：静态 `index.json`、灌库写入的
   `observation`、接口返回的 `DatasetRow` 三者必须逐字段同形。有单测跨两侧比对，
   改名或加字段会当场红。见 `docs/DECISIONS.md` D21。
10. **阈值与权重只在 `checks.rs` 出现一次**：`desc_len < 10`、空值率 `> 30%`、
    滞后 `> 90/365` 天、停更词表都在那张登记表里。`scoring` / `aggregate` /
    `export` / 前端全部引用它，别再写字面量。判定函数 `fires()` 也只有一处实现——
    同一个概念有两种算法时（历史上 `schemaMismatch` 有 1,284 和 1,256 两个数），
    错的那一处不会报错，只会安静地给出另一个答案。见 D30。
11. **历史曲线只记原始计数，不记问题分**：`history/summary.json` 里的字段是七个计数，
    权重一变分数就不可比，分数进曲线等于把两把尺子画在一张图上。缺字段的那一轮整条不记
    （补 0 会画出假的「降到 0」）。见 D31。

## 分层边界

- `crates/core/src/` 纯函数（`matrix` / `scoring` / `guard` / `lean` / `util`）不 import 网络与文件。
- IO 通过 `CatalogSource` trait 与显式路径参数注入，CI 用 `tests/pipeline.rs` 的假平台。
- `crates/cli/src/main.rs` 只解析参数、编排调用；业务逻辑落在 core。
- `crates/web` 只 GET 静态文件。新增前端字段时同步 `export.rs` 与 `model.rs`。
- `crates/api` 依赖 core，反向不成立。它对外**只读**；唯一的写路径是 `ingest` CLI，
  不监听端口——写库只需要一个人，查询才需要服务。

## 常用验证

```bash
just check        # fmt + clippy(core/cli) + 全量单测（core/cli/api/web）+ 证明门禁
just clippy-wasm  # 前端也在门禁内
just coverage     # 两组数：core+cli 地板 74%（实测 76.88%）/ 含 api 地板 72%（实测 73.81%）
just lint-docs
just smoke        # 联网，200 条，产物进 out-smoke/
```

改动统计口径时额外跑 `docs/HANDOFF.md` 第 7 节的对齐检查。

## 平台与环境的已知坑

- **代理**：crates.io / GitHub 需 `HTTPS_PROXY=http://127.0.0.1:7890`。
- **CI 必须置空 `RUSTFLAGS`**：`.cargo/config.toml` 有 `-C target-cpu=native`，
  异代 runner 上产物会 SIGILL。见 `docs/ENGINEERING_LESSONS_ZH.md` 与 ci.yml 顶部注释。
- **trunk 必须保留 `public_url = "./"`**：GitHub Pages 在 `/assay/` 子路径，绝对路径会 404 白屏。
- **`strip = true` 不能开**：wasm-bindgen 读不到 `target_features` 段。
- **平台脏数据**：存在 `2025-07-30 09:70:21` 这类非法时间戳，校验时分秒后跳过。
- **平台分页**：只认 `current`/`size`；`page` 被静默忽略并返回第一页。
- **接口 id** 含 `+ / =`，逐字符编码；落盘文件名走 `util::safe_id`。

完整清单见 `docs/ENGINEERING_LESSONS_ZH.md`。
