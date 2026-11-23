# 贡献指南

## 环境

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-nextest cargo-deny taplo-cli typos-cli just
cargo install cargo-llvm-cov        # 覆盖率门禁要用
./scripts/install-git-hooks.sh     # Conventional Commits 校验
```

本机访问 crates.io 需代理：`export HTTPS_PROXY=http://127.0.0.1:7890`。

需要 **Rust 1.88+**（实际下限，`icu_*` 依赖要求；CI 会用 1.88.0 真跑一次）。

## 提交前必须全绿

```bash
just check        # fmt(Rust+TOML) + clippy + nextest + cargo-deny + 证明门禁
just clippy-wasm  # 前端也在门禁内
just coverage     # 行覆盖率地板 74%（实测 76.88%）
```

`just check` 红了不要提交，也不要为了让测试过而改断言——先判断是实现错了还是口径本就该变。
覆盖率地板红了也别调低地板：补测试，或者明确说明为什么这段测不了。

## 口径改动

`scoring::classify` / `scoring::score` 是唯一口径来源。改了要：

1. 跑 `docs/HANDOFF.md` 第 7 节的口径对齐，确认 2,500 个部门逐字段无差异（或差异已解释）
2. 在 `docs/DECISIONS.md` 记一条，写明为什么允许变化

## 不得破坏的不变量

完整清单在 `docs/ARCHITECTURE.md` 第 5 节。最常踩的三条：

- 证明对象 = 存证对象：`proof` 优先读 `evidence/<id>/preview.json`，不现场重抓
- 哈希算原始字节：证据包走 `api::post_raw`
- 前端只读：WASM 只 GET 静态文件

## 提交信息

Conventional Commits，英文，钩子会拦。类型：`feat` `fix` `docs` `refactor` `test` `chore` `perf` `ci`。

```
feat(reports): 生成前 100 名单数据集报告并随站点发布
```

## 不提交的东西

`out-full/`、`out-smoke/`、`vendor/`、`crates/web/dist/`、任何形式的 key。

## 遇到问题

先搜 `docs/ENGINEERING_LESSONS_ZH.md`——踩过的坑都记在那里。
