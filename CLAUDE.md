# CLAUDE.md · 数证 DataProof

给在这个仓库里工作的 AI Agent 的项目说明。全局通用规则见 `~/.claude/CLAUDE.md`，
这里只写本项目的架构边界、口径不变量和验证方式。

先读：

1. `README.md`（怎么跑、边界在哪）
2. `AGENTS.md`（架构边界与护栏）
3. `docs/ENGINEERING_AGENT_GUARDAILS_ZH.md`（分支、文档先行、提交、PR 纪律）
4. `docs/ENGINEERING_LESSONS_ZH.md`（已经踩过的坑，排障前先搜）

改统计、证明、证据、导出之前，再加读 `docs/ARCHITECTURE.md` 与 `docs/DECISIONS.md`。

## 项目一句话

给政府开放数据做确定性体检，把「第 N 列恰好 K 条空值」写成 Lean 定理交给内核穷举验证，
并用 SHA-256 钉住抓取时点的原始响应。模型只在解释层出现。

## 硬约束（违反即返工）

1. **口径只有一处实现**：`scoring::classify` / `scoring::score`。改动要跑口径对齐并记 `docs/DECISIONS.md`。
2. **证明对象 = 存证对象**：`proof` 必须先读 `evidence/<id>/preview.json`，不许改成现场重抓。
3. **哈希算原始字节**：走 `api::post_raw`，不许 `send_json` 后重新序列化再哈希。
4. **基准日写死** `NOW_YMD = (2026, 11, 14)`，不读系统时间。
5. **模型不参与取数与算数**：解释层只喂事实字典，输出必须过 `guard::check`。
6. **禁止编造运行结果**：报告里每个数字都要来自真实命令输出；跑不了就说跑不了。
7. **禁止为了让测试过而改断言**：先判断是实现错了还是口径本就该变。
8. **前端只读**：WASM 只 GET 静态文件，不写、不调平台接口。
9. **CI 不访问政府网站**：只用 `fixtures/` 与假平台（`crates/core/tests/pipeline.rs`）。
10. **不引入 Python 运行时依赖**：脚本只允许 bash（`ci/`）。
11. **任何 key 不进仓库/日志/聊天**：本地走 `DATAPROOF_LLM_KEY` 或 Hermes 配置。

## 常用命令

```bash
just check        # fmt(Rust+TOML) + clippy + nextest + cargo-deny + 证明门禁，必须全绿
just clippy-wasm  # 前端也在门禁内
just lint-docs    # Markdown lint
just typos        # 拼写检查
just deny         # 依赖审计（许可证 + 漏洞），白名单在 deny.toml
just changelog    # 由 Conventional Commits 生成 CHANGELOG.md
just smoke        # 200 条端到端（联网），产物进 out-smoke/
just all          # 全量（联网，约 14 分钟）
just serve        # 本地预览
```

首次克隆后跑 `just hooks` 安装 commit-msg 钩子（Conventional Commits 校验）。

## 模块职责

| 层 | 位置 | 约定 |
|---|---|---|
| 纯函数 | `matrix` / `scoring` / `guard` / `lean` / `util` | 不碰网络和文件，CI 全速跑 |
| IO 层 | `api` / `audit` / `evidence` / `proof` / `aggregate` / `export` / `explain` / `single` | 显式接收路径与 `CatalogSource`，便于用快照对比 |
| 入口 | `crates/cli/src/main.rs` | 只做参数解析与编排，逻辑放 core |
| 前端 | `crates/web` | Leptos CSR，只读 `data.json` |

依赖只能向下：`web → cli → core`。
