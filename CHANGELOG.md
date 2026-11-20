# Changelog

本项目所有值得记录的变更。格式参考 [Keep a Changelog](https://keepachangelog.com/)，
版本号遵循 [SemVer](https://semver.org/)。

## [0.2.2](https://github.com/qiaopengjun5162/assay/compare/v0.2.1...v0.2.2) (2026-11-20)


### Bug Fixes

* **release:** Windows sha256 用 LF，修发版资产自检 ([#41](https://github.com/qiaopengjun5162/assay/issues/41)) ([0a64b7c](https://github.com/qiaopengjun5162/assay/commit/0a64b7cf5ce20261a03d9da3a64edf0e5aeef2de))

## [0.2.1](https://github.com/qiaopengjun5162/assay/compare/v0.2.0...v0.2.1) (2026-11-20)


### Features

* **api:** ingest 子命令，把跑批产物灌进查询库 ([96e64a2](https://github.com/qiaopengjun5162/assay/commit/96e64a22b1458be95f668406e85bf42d7bb218cf))
* **api:** ingest 灌库闭环 + 修两个只在真实数据量下暴露的缺陷 ([7a235c2](https://github.com/qiaopengjun5162/assay/commit/7a235c25dfd66ae02827eed6049afd69746042ee))
* **api:** 只读查询后端骨架，证据仍留静态 ([7da07c8](https://github.com/qiaopengjun5162/assay/commit/7da07c828fc0ac5df5490569b80ab7ccc6822374))
* **api:** 只读查询后端骨架，证据仍留静态 ([f22032d](https://github.com/qiaopengjun5162/assay/commit/f22032d3c3d69714671252adbc66a276bd59136f))
* **audit:** 周度漂移巡检 + 补三道工程门禁 ([15d0628](https://github.com/qiaopengjun5162/assay/commit/15d0628eaf0896bdd073e053b9c678f759f5c7c5))
* **audit:** 周度漂移巡检，指标漂移自动开 PR ([2a69ef0](https://github.com/qiaopengjun5162/assay/commit/2a69ef04820a9f1b6713c3463523243c398943ef))
* **cli:** report-one 单数据集 Markdown 报告 ([54399fe](https://github.com/qiaopengjun5162/assay/commit/54399fe8301f2b6fe2a813cf26a238ce65b16bb4))
* **explain:** 模型端点/模型名/代理可配，并加一条命令验配置 ([bae8648](https://github.com/qiaopengjun5162/assay/commit/bae8648051193af9cdefc351462d89e3aa70f485))
* **explain:** 模型端点可配，加一条命令验配置，解读扩到 500 条 ([cf0b300](https://github.com/qiaopengjun5162/assay/commit/cf0b300de926808d00655f871fde21f30f511901))
* **export:** --split 产出全量索引与详情分片 ([014bd63](https://github.com/qiaopengjun5162/assay/commit/014bd632b5c947fd1fc1352fac1f05dc5c15b4c5))
* **export:** --split 产出全量索引与详情分片 ([ab69c62](https://github.com/qiaopengjun5162/assay/commit/ab69c623660510ba29b982aad7fbf9bf4abdd1fb))
* **gate:** 报告里的哈希必须对上证据，防跨批次拼产物 ([ad073a2](https://github.com/qiaopengjun5162/assay/commit/ad073a22f40dd8ef71ebf33c31d214fb9466e1ec))
* **gate:** 报告里的哈希必须对上证据，防跨批次拼产物 ([a4c2d15](https://github.com/qiaopengjun5162/assay/commit/a4c2d159f5f8b8c39d122e69c028c9f6835b4084))
* **gate:** 报告里的哈希必须对上证据，防跨批次拼产物 ([#28](https://github.com/qiaopengjun5162/assay/issues/28)) ([ad073a2](https://github.com/qiaopengjun5162/assay/commit/ad073a22f40dd8ef71ebf33c31d214fb9466e1ec))
* **history:** 跑批历史日志与站点曲线 ([e82f2c1](https://github.com/qiaopengjun5162/assay/commit/e82f2c15235c457f5352275ec10d3e96d5edcfb9))
* **history:** 跑批历史日志与站点曲线 ([#22](https://github.com/qiaopengjun5162/assay/issues/22)) ([929dc23](https://github.com/qiaopengjun5162/assay/commit/929dc23fdc0011028d52101e24e4275b1bdd4951))
* **history:** 跑批历史日志与站点曲线（并修正过期的定理数） ([929dc23](https://github.com/qiaopengjun5162/assay/commit/929dc23fdc0011028d52101e24e4275b1bdd4951))
* **pack:** 审计包单文件交付，解压一条 shasum 命令即可验 ([61808ad](https://github.com/qiaopengjun5162/assay/commit/61808adafa324ce48a49a6b0939412986e46d36b))
* **pack:** 审计包单文件交付，解压一条 shasum 命令即可验 ([ee76959](https://github.com/qiaopengjun5162/assay/commit/ee769595c9d2d2bc28f61b0c72441f421753f2be))
* **release:** 配置 release-please 与五平台二进制发布 ([#35](https://github.com/qiaopengjun5162/assay/issues/35)) ([76a9132](https://github.com/qiaopengjun5162/assay/commit/76a91326a7305db9c753f77d7bd53b308266962d))
* **reports:** reports 子命令 + 详情页下载入口 ([9f5306b](https://github.com/qiaopengjun5162/assay/commit/9f5306b35a94acb72e679a658f62d75f54b8a8b9))
* **reports:** 生成前 100 名单数据集报告并随站点发布 ([21fca47](https://github.com/qiaopengjun5162/assay/commit/21fca479df69fb5cdc6bfbe4d70b3c8c9cca7fe8))
* **web:** 一句话讲清这站在干什么，首屏换成一个真实案例 ([448d115](https://github.com/qiaopengjun5162/assay/commit/448d115b3cad13a1d1093dd371e81eeb8ae1330f))
* **web:** 可验覆盖从 100 扩到 500，并修好「完整性」把最差说成最好 ([7371e6c](https://github.com/qiaopengjun5162/assay/commit/7371e6c9cc5550d22cf0351ca6f17d0d9c439a49))
* **web:** 可验覆盖扩到 500 条，并修好「完整性」把最差说成最好 ([33b68a1](https://github.com/qiaopengjun5162/assay/commit/33b68a1549a1af4d3e232eb3c566dd0acde3d340))
* **web:** 按访客路径走查后补齐导航、分享与窄屏可用性 ([#29](https://github.com/qiaopengjun5162/assay/issues/29)) ([bc9e425](https://github.com/qiaopengjun5162/assay/commit/bc9e425f6054047f52f97d367b9406fe9b4cf753))
* **web:** 无障碍按 axe 判据过一遍，加 just a11y 门禁 ([#31](https://github.com/qiaopengjun5162/assay/issues/31)) ([1e517b6](https://github.com/qiaopengjun5162/assay/commit/1e517b67f816bf55a8cec67164ae7435a3de0291))
* **web:** 部门页（#/dept/&lt;部门名&gt;），同屏写清全量 N 与榜上 M ([#32](https://github.com/qiaopengjun5162/assay/issues/32)) ([b696260](https://github.com/qiaopengjun5162/assay/commit/b696260496f71c1794c99017fca1b2282f15bc37))
* **web:** 首屏改成一个具体案例，一句话讲清这站在干什么 ([6600948](https://github.com/qiaopengjun5162/assay/commit/66009481605ceee3abaf2d21790ac1b8526bcbba))
* **web:** 首页加部门榜（停更数 Top12） ([41dd8cd](https://github.com/qiaopengjun5162/assay/commit/41dd8cdbf57025b0ad46d552c945d503d1a0625f))
* 报告与证明的站内交叉引用闭环 ([#34](https://github.com/qiaopengjun5162/assay/issues/34)) ([95debb2](https://github.com/qiaopengjun5162/assay/commit/95debb28083e4a1b2bc31f893e22b36e6cbfdc89))
* 证据复核 verify-evidence 落库（逐份重算 SHA-256） ([0c125f2](https://github.com/qiaopengjun5162/assay/commit/0c125f286fea7fee9eeddadfe727f59fcb381c6c))


### Bug Fixes

* **api:** 最近一次 run 按日期取，空库返回 404 而非 500 ([6397e2c](https://github.com/qiaopengjun5162/assay/commit/6397e2c5fa2d8779150ca6d854e4b7d3369b98d4))
* **data:** 入库 data.json 与代码字段名不一致，并加检查防复发 ([025f620](https://github.com/qiaopengjun5162/assay/commit/025f62065ca56420224e002d8dbc285c84aa8d0b))
* **data:** 入库 data.json 与代码字段名不一致，并加检查防复发 ([#25](https://github.com/qiaopengjun5162/assay/issues/25)) ([025f620](https://github.com/qiaopengjun5162/assay/commit/025f62065ca56420224e002d8dbc285c84aa8d0b))
* **data:** 入库的 data.json 与代码字段名不一致，并加检查防复发 ([7559a5e](https://github.com/qiaopengjun5162/assay/commit/7559a5e61e71d20294505eadd522d0f4e21fd6b1))
* **just:** proofs recipe 改调 check-proofs 子命令 ([3669459](https://github.com/qiaopengjun5162/assay/commit/3669459caddc86fbaa38efc1123b6a12dfe645ca))
* **release:** 版本源改为独立文件，避免清空 Cargo.toml ([#38](https://github.com/qiaopengjun5162/assay/issues/38)) ([7d5df30](https://github.com/qiaopengjun5162/assay/commit/7d5df30c3b89bb9047a65b6b8fa634ddd5b4ee76))
* **release:** 虚拟 workspace 改用 simple 发版策略 ([#36](https://github.com/qiaopengjun5162/assay/issues/36)) ([db7d2db](https://github.com/qiaopengjun5162/assay/commit/db7d2dbd4a09e3de18ac5b601787b0523452e91f))
* **web:** 复核命令改成绝对 URL + 一键复制，评委真能验了 ([560e4c5](https://github.com/qiaopengjun5162/assay/commit/560e4c5e843d92369338ddd2c9fe1f1d7e920a1e))
* **web:** 复核命令改成绝对 URL + 一键复制，评委真能验了 ([3086627](https://github.com/qiaopengjun5162/assay/commit/3086627c37723dcc454b7fd9e82a53d321c261bb))
* **web:** 子路径部署 —— trunk public_url 改为相对路径 ([49d9c31](https://github.com/qiaopengjun5162/assay/commit/49d9c31168e7a3a6a3011055aa53bdeb20ee5294))
* **web:** 排序不再走 partial_cmp().unwrap()，NaN 不该白屏 ([eba768c](https://github.com/qiaopengjun5162/assay/commit/eba768c259e7994d2e0653c2149f20ec1c486b7c))
* **web:** 日期不从中间断行 ([#30](https://github.com/qiaopengjun5162/assay/issues/30)) ([ed58a9d](https://github.com/qiaopengjun5162/assay/commit/ed58a9dc29209cd08489a862911f226ab87b9228))
* **web:** 部门榜字段接进 UI，修 wasm clippy 的 dead_code ([ac25741](https://github.com/qiaopengjun5162/assay/commit/ac25741bf165aaf5a320451afec27ff20800f79b))
* 演示路径上两处过期文案 ([79922d5](https://github.com/qiaopengjun5162/assay/commit/79922d5bbe80f994c1a6c4563cf9d940aa315374))
* 页头日期不再写死，拼写门禁进 CI ([ea6d91f](https://github.com/qiaopengjun5162/assay/commit/ea6d91f241e7cae840ad41f8506809212d107885))
* 页头日期不再写死，拼写门禁进 CI ([aeaee4b](https://github.com/qiaopengjun5162/assay/commit/aeaee4bd73186593b7ec424232d26971ceea955a))


### Refactoring

* **api:** 把平台响应形状从 CatalogSource 边界上清掉 ([de23d5f](https://github.com/qiaopengjun5162/assay/commit/de23d5fa6718cc81f23d8470d4194cd962529904))
* **api:** 换平台只需新增一个 impl，并记下第二平台为什么接不上 ([19efeac](https://github.com/qiaopengjun5162/assay/commit/19efeacde757262f030faeecb749717f668b9f34))
* **checks:** 导出的键由结构体派生，不再手写 json! ([1f5bd2f](https://github.com/qiaopengjun5162/assay/commit/1f5bd2fbf75e478903b51c6a93951b5e1a2bfcfd))
* **checks:** 导出的键由结构体派生，不再手写 json! ([e6e46dc](https://github.com/qiaopengjun5162/assay/commit/e6e46dcddf4a45ca55fb4d2549b67a6faefdd5ba))
* **checks:** 导出的键由结构体派生，不再手写 json! ([#27](https://github.com/qiaopengjun5162/assay/issues/27)) ([1f5bd2f](https://github.com/qiaopengjun5162/assay/commit/1f5bd2fbf75e478903b51c6a93951b5e1a2bfcfd))
* **core:** 把阈值与判定收成一张表，并把这张表发布出去 ([1e6355d](https://github.com/qiaopengjun5162/assay/commit/1e6355d78fd530faacb3516a0bb7ef4886b92a1d))
* **core:** 把阈值与判定收成一张表，并把这张表发布出去 ([4d9d58e](https://github.com/qiaopengjun5162/assay/commit/4d9d58ebe51a8614d3fec9068539e355fbdd6ae6))
* **core:** 抽出 CatalogSource trait，加离线端到端测试 ([15a42c0](https://github.com/qiaopengjun5162/assay/commit/15a42c023996708f9908668574f508fee297e5da))
* 标识符改名 dataproof → assay ([0cd5e57](https://github.com/qiaopengjun5162/assay/commit/0cd5e5723363134dd5f0a8582ad46a7ea286bcca))
* 标识符改名 dataproof → assay ([bc19607](https://github.com/qiaopengjun5162/assay/commit/bc196078e653bbed5db742bb19438631b61213a0))
* 清理改名残留并给环境变量加双读过渡 ([6cb5f44](https://github.com/qiaopengjun5162/assay/commit/6cb5f44b700a892ec62e7f300eec3db29381bcdb))
* 证明门禁搬进 Rust CLI（dataproof check-proofs） ([4c11e5a](https://github.com/qiaopengjun5162/assay/commit/4c11e5a3c3b10d1745045cc39a74fe8279c03fbc))

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

- 数验 Assay 工程骨架
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
- 证明门禁搬进 Rust CLI（assay check-proofs）

<!-- 由 git-cliff 生成，请勿手改 -->
