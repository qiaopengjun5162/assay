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
5. **报告、证明、证据同一选择口径**（`has_preview == 1` + `score` 降序）。三者集合必须一致，
   `web/reports` 的 id 集合要等于 `proofs.json` 的 id 集合，见 `docs/DECISIONS.md` D16。
6. **准确性明确不评价**：没有权威源，评了就是编。
7. **样例级**：完整性来自表头 + 5 行样例，不是全量 2.26 亿行；文案不许说成全量。

## 分层边界

- `crates/core/src/` 纯函数（`matrix` / `scoring` / `guard` / `lean` / `util`）不 import 网络与文件。
- IO 通过 `CatalogSource` trait 与显式路径参数注入，CI 用 `tests/pipeline.rs` 的假平台。
- `crates/cli/src/main.rs` 只解析参数、编排调用；业务逻辑落在 core。
- `crates/web` 只 GET 静态文件。新增前端字段时同步 `export.rs` 与 `model.rs`。

## 常用验证

```bash
just check        # fmt + clippy(core/cli) + 单测 + 证明门禁
just clippy-wasm  # 前端也在门禁内
just lint-docs
just smoke        # 联网，200 条，产物进 out-smoke/
```

改动统计口径时额外跑 `docs/HANDOFF.md` 第 7 节的对齐检查。

## 平台与环境的已知坑

- **代理**：crates.io / GitHub 需 `HTTPS_PROXY=http://127.0.0.1:7890`。
- **trunk 必须保留 `public_url = "./"`**：GitHub Pages 在 `/assay/` 子路径，绝对路径会 404 白屏。
- **`strip = true` 不能开**：wasm-bindgen 读不到 `target_features` 段。
- **平台脏数据**：存在 `2025-07-30 09:70:21` 这类非法时间戳，校验时分秒后跳过。
- **平台分页**：只认 `current`/`size`；`page` 被静默忽略并返回第一页。
- **接口 id** 含 `+ / =`，逐字符编码；落盘文件名走 `util::safe_id`。

完整清单见 `docs/ENGINEERING_LESSONS_ZH.md`。
