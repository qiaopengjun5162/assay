# 编程 Agent 协作护栏

给 Claude Code、Codex 等编程 Agent 的元规则。补充 `AGENTS.md` 里的项目-specific 边界，
说明分支、提交、文档、PR 和生产边界等通用协作方式。

## 开始协议

动手前先读：

1. `README.md`
2. `AGENTS.md` 与 `CLAUDE.md`
3. 本文
4. `docs/ENGINEERING_LESSONS_ZH.md`

改动触及统计口径、证明、证据、导出或前端数据契约时，再加读 `docs/ARCHITECTURE.md`
与 `docs/DECISIONS.md`。

## 分支规则

- 不在 `main` 上堆大改；一个分支只做一个模块或一个已记录的计划项。
- 提交信息用 Conventional Commits：`feat` / `fix` / `docs` / `refactor` / `test` / `build` / `ci` / `chore`。
- 本地装钩子：`./scripts/install-git-hooks.sh`。

## 文档先行

新增功能、口径变更、迁移、产物契约变化，先写或更新文档，再写代码。文档里要说明：

- 目标模块与边界
- 验证命令
- 是否触及口径 / 证明 / 证据 / 前端契约
- 回滚或降级方式

拼写错误、机械格式化、单行 bugfix 不需要额外设计文档，但仍要更新受影响的说明。

## 架构边界

- 纯函数层（`matrix` / `scoring` / `guard` / `lean` / `util`）不 import 网络与文件。
- IO 层通过 `CatalogSource` 与显式路径接收依赖，便于用冻结快照做对比测试。
- CLI 只做解析与编排。逻辑落 core，前端只读静态文件。
- 前端字段与 `export.rs` / `model.rs` 必须成对出现；缺一个就是功能漏了，不要 `#[allow(dead_code)]` 糊过去。

## 口径变更纪律

改 `scoring::classify`、`scoring::score`、聚合、舍入、选择口径后必须：

1. 跑 `docs/HANDOFF.md` 第 7 节的对齐检查，逐字段对比冻结产物；
2. 差异逐条解释清楚；
3. 在 `docs/DECISIONS.md` 记一条，写清为什么允许变化；
4. 报告集、证明集、证据集的 id 集合保持一致。

**禁止为了让测试变绿而改断言。** 先确认是实现错了，还是口径本就该变。

## 生产边界

以下改动要在 PR 里写明风险与回滚方式：

- 统计口径 / 评分权重 / 聚合
- Lean 生成或证明门禁（证明坏了等于产品坏了）
- 证据抓取与哈希（历史哈希一旦漂移，全部证据失去可复核性）
- 静态站数据契约（`web/data.json`）与部署路径
- CI 门禁的放宽

## 凭据与体积

- 不提交真实 key、`.env`、Hermes 配置。
- `out-full/`、`crates/web/dist/`、`vendor/` 不进 git。
- 提交前跑 `just check` 与 `just clippy-wasm`。

## 停止条件

遇到以下情况停下来问人，不要自己拍板：

- 需要判断某个指标该不该改变口径
- 要放宽或删除 CI 门禁
- 需要重跑全量、刷新已冻结的快照（会让文章与榜单对不上）
- 需要访问新的第三方平台或引入新依赖族
