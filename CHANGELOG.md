# Changelog

本项目所有值得记录的变更。格式参考 [Keep a Changelog](https://keepachangelog.com/)，
版本号遵循 [SemVer](https://semver.org/)。

## [未发布]

### 修复

- **web**：子路径部署 —— trunk public_url 改为相对路径
- **web**：部门榜字段接进 UI，修 wasm clippy 的 dead_code
- **just**：Proofs recipe 改调 check-proofs 子命令

### 文档

- **pitch**：6 分钟路演脚本 + 篡改演示脚本
- **readme**：CI 徽章与仓库链接
- 指向仓库与路演脚本
- **readme**：加在线演示地址
- 一页版项目说明 + 架构图
- **data**：入库聚合结果快照（summary + 两张榜单）
- 交接状态更新（main = 全 Rust，已上线）
- 补证明扩展性实测（500 条 63.9s / 5,923 定理）
- 离线构建预案实测（vendor 200MB，--offline 30s 通过）
- PLAN 与决策记录同步到全 Rust 现状
- 第二个平台的接口探查步骤
- 补齐 Agent 协作、经验库与工程模板

### 新增

- 数证 DataProof 工程骨架
- **web**：榜单 / 详情 / 证明三页静态站
- **evidence**：证据包（确定性抓取 + SHA-256）+ 详情页展示
- **proof**：证明扩到前 100 名 / 2,355 条定理，改跑存证快照
- **pipeline**：一条命令跑通全流程，修掉目录快照缺口
- **explain**：AI 解释层 + 数字护栏
- **web**：首页加部门榜（停更数 Top12）
- **cli**：Report-one 单数据集 Markdown 报告
- **reports**：生成前 100 名单数据集报告并随站点发布
- **reports**：Reports 子命令 + 详情页下载入口

### 杂务

- **just**：Prove 与 evidence 默认 100 条，prove 直接读存证快照
- **secret-scan**：改用 gitleaks CLI
- **web**：加 .nojekyll，准备 GitHub Pages
- **pages**：用 Actions 发布 web/ 静态站

### 重构

- 全 Rust 工作区 + Leptos WASM 前端 (#1)
- **core**：抽出 CatalogSource trait，加离线端到端测试
- 证明门禁搬进 Rust CLI（dataproof check-proofs）

<!-- 由 git-cliff 生成，请勿手改 -->
