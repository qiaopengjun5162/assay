# Pull Request

> **标题遵循 Conventional Commits**（例：`feat(cli): add drift subcommand`）。
> `commit-lint` 校验的是分支上的提交；PR 标题不过钩子，但如果以后切成 squash
> merge，标题会原样成为 `main` 上的提交信息。

## 改动类型

- [ ] feat：新功能
- [ ] fix：bug 修复
- [ ] docs：文档更新
- [ ] refactor：重构，行为不变
- [ ] test：测试补充或修复
- [ ] build：构建/依赖/工具链
- [ ] ci：CI/check 门禁
- [ ] chore：日常维护

## 改动说明

说明解决了什么问题、为什么这样改。避免营销式表达。

## 涉及模块

例如 `crates/core/src/scoring.rs`、`crates/web/src/app.rs`、`docs/`。

## 验证命令

```bash
just check
just clippy-wasm
just lint-docs
# 口径变更时额外跑 docs/HANDOFF.md 第 7 节的对齐检查
```

实际输出贴在这里，不要写「应该通过」。

## 口径影响

- [ ] 未改统计口径
- [ ] 改了，已跑对齐检查，差异解释：<!-- 填 -->
- [ ] 改了，已在 `docs/DECISIONS.md` 记录

## 产物一致性

- [ ] `web/reports` 的 id 集合仍等于 `proofs.json` 的 id 集合
- [ ] 证明仍读 `evidence/<id>/preview.json`（未改成现场重抓）

## 文档同步

- [ ] 已更新相关 README / ARCHITECTURE / DECISIONS / HANDOFF / PROGRESS
- [ ] 纯格式或机械改动，无需文档更新

## 凭据与体积

- [ ] 没有提交 key / `.env` / Hermes 配置
- [ ] `out-full/`、`crates/web/dist/`、`vendor/` 未入库

## 回滚说明（可选）

涉及口径、证明、证据或部署路径时写清如何回滚。
