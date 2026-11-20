# HANDOFF — 接手须知

> 给接手的 AI 或开发者。读完这份再动代码，能省掉大部分返工。
> 最后更新：2026-11-20

## 0. 项目位置

```
/Users/qiaopengjun/Code/Rust/assay
```

远程：`https://github.com/qiaopengjun5162/assay`

## 1. 当前状态（重要）

`main` 就是全 Rust 版本，已合并并上线（2026-11-15）。没有并行的旧版分支了。

已经验证过的事（不要重复怀疑，除非你有新证据）：

- `just check` 全绿（fmt / clippy core+cli / 单测全过（**154 @ 2026-11-20**）/ 证明门禁）
- `just clippy-wasm` 全绿
- 口径对齐：用旧 `out-full/items.jsonl` 重算，`summary.json` 标量与 **2,500 个部门全部一致**
- 证明对齐：391 个可证数据集的 **SHA-256 与顺序全部一致**，`lean` PASS，4,970 条定理，39.5 秒
- 导出对齐：500 条、前 200 条关键字段 0 差异
- 前端：WASM 实测榜单 500 行、搜索过滤、详情六维、AI 解读、证据包、证明页 PASS、浏览器返回正常

## 2. 已完成 / 待办

已完成（2026-11-20 · 发布工程化 + 已发版 v0.2.2）：

- 新增 `.github/workflows/release-please.yml` + `release.yml`：前者开纯发版 PR，
  后者在 tag / 手动触发时构建 **5 平台**二进制（linux amd64/arm64、macOS intel/arm64、
  Windows amd64），每平台出包 + `.sha256`，共 **10 个资产**；资产齐了才把 draft 发出去。
- 打包 smoke 从压缩包里解出来跑 `--version/--help` + `verify-evidence` + `verify-reports`，
  验的是用户下载到的字节，不是 `target/` 里的。
- 配置：`.release-please-config.json`（`bootstrap-sha` = `6532b3a`，`release-type: simple`，
  `draft: true`）+ `.release-please-manifest.json`（当前 `0.2.2`）。`release-type: rust` **不能用**：
  它是虚拟 workspace（根 `Cargo.toml` 没有 `[package]`），子 crate 又用
  `version.workspace = true`，rust 策略会报 `value at path package.version is not tagged`。
  改用 `simple` + 单独的 `.release-please-version` 作版本源，根 `Cargo.toml` 与
  ROADMAP 的版本行带 `x-release-please-version` 标记，由 `generic` extra-files
  只改那一行；并用 4 条 `toml` extra-files 抬 `Cargo.lock` 里的 assay 版本，
  才能守住 CI 的 `--locked`。⚠️ `version-file` **不能**指到 `Cargo.toml`：
  simple 策略的 DefaultUpdater 会把整个文件替换成一行（PR #37 的教训）。
- ⚠️ 本机调试 `release-please release-pr --local --local-path .` 会执行
  `git checkout main && git reset --hard origin/main`，**把未提交改动全清掉**（已踩过一次）。
  要验证配置先提交到分支、用 `--target-branch <分支>` 干跑，别在脏工作区上跑。
- ⚠️ 默认 `GITHUB_TOKEN` 建的 tag **不会触发** `release.yml`。发版必须按
  `docs/RELEASE_CHECKLIST.md` 人工点火；可选 `RELEASE_PAT` secret 走全自动。
  **已发版**：首个 `v0.2.1`、当前 `latest=v0.2.2`（10 资产全自动）。
  发版后 release-please 常开「假发版 PR」（CHANGELOG 汇总历史、增量几十上百行），
  关掉即可；小增量、只含真实 commit 的才是真发版 PR。理由见 D36。
- Windows 侧 `.sha256` 必须 LF：`Out-File` 写 CRLF 会让 `shasum -c` 报
  `FAILED open or read`（v0.2.1 踩过）；已改 `WriteAllText` + LF 根治。
- 改 workflow 后必跑 `actionlint .github/workflows/*.yml`（本机已装）。

已完成（2026-11-15）：

- CI：rust（fmt/clippy/test/build）、wasm（trunk 构建 + clippy）、lean（内核校验）、密钥扫描
- Pages 两段式：分支只构建 artifact，`main` 才部署 `crates/web/dist`
- 线上已切到 WASM 版：500 行榜单、详情六维、AI 解读、证据下载、证明页 PASS、浏览器返回正常
- `docs/DECISIONS.md` 已补 D12–D15

已完成（2026-11-18）：

- CI 从 4 个 job 扩到 8 个：新增 `msrv`（1.88 真编）、`coverage`（地板 52%，2026-11-19 两次抬到 74）、
  `commit-lint`（Conventional Commits）、`docs`（markdownlint）
- `assay drift` + `audit.yml` 周度巡检，退出码三档（见 D19、`docs/audit/README.md`）
  首份基线 `baseline/summary.json`（24,322 条 / 抓取成功率 81.53%）
- 改名残留清理完毕：环境变量改 `ASSAY_LLM_KEY` / `ASSAY_CONFIG`，旧名过渡期仍可读
- ⚠️ **`AUDIT_PAT` 尚未配置**：漂移那一周会停在「检查 PR 凭据」并提示缺 secret

已完成（2026-11-19 · 覆盖率口径摊开 + 前端白屏路径）：

- `scripts/coverage.sh` 改为一次测量报两个数：逻辑主体（core+cli）76.88% 地板 74、
  全量（含 api）73.81% 地板 72。原因写在脚本头部与 D32：`crates/api` 917 行
  从来没进过分母（它自己 34%），一个数字被当成整个项目的覆盖率在引用。
- `just test` 补上 `-p assay-api`——它原来只在 CI 跑，本地 `just test` 是 123 绿、
  CI 是 132 绿，「本地过了」是假信号。现在两边同为 **154**（core 118 / cli 2 / api 14 / web 20）。
- api 补 5 条不依赖数据库的测试：`read_run_meta` 的三种失败各自点名
  （读不到 / 不是合法 JSON / 缺哪个字段）、计数超出 i32 范围（顺带修了报错
  把「超出」说成「缺字段」）、`ApiError` 的 404 带原因、500 **不外泄**数据库原文。
- `crates/web/src/sort.rs`：排序从组件里抽成纯函数。原来 `partial_cmp().unwrap()`
  遇到 NaN 会让 WASM **整页白屏**；改用 `total_cmp` 并把 NaN 显式压到最后
  （「不知道」不是「最差」，不能抢榜首）。5 条单测，含同分保持输入顺序——
  首屏第一张卡是讲稿点名的，同分若随构建漂移就对不上。

已完成（2026-11-19 · 报告门禁 + 一批产物的已知差异）：

- 新增 `assay verify-reports`（`just verify-reports`，已进 `just check`）：报告里印的
  SHA-256 必须等于证据文件重算结果。防的是「用错 `--out` 补报告」——跨批次生成时
  每份报告看着都完整，读者逐份核对却全部失败。实测 **500/500 一致**；故意混入一份
  另一批次的报告，会红并说清「不是同一批抓取」。
- 2026-11-20：报告顶部加两行**绝对 URL 回链**（在线详情 / 在线证明，id 逐字节编码），
  证明页加「详情」面包屑和返回/看报告入口。`verify-reports` 同时核验回链：
  注入一处旧链接 → 499/500 并指出文件名。
  ⚠️ **重生成报告只能跑 `just reports 500`**：配方已固定 `--evidence-dir web/evidence`，
  并传 `--no-explain`。直接调 CLI 会从 `out-full/evidence`（只有 100 份）取证据，
  生成 500 份**没有哈希段**的空壳；`--no-explain` 是为了与已发布报告零差异
  （AI 解读只在网页，见下）。
- **已知差异（不影响可核对性，赛前不修）**：
  - 500 份入库报告是 **11-18 批次**，**没有 `## AI 解读` 段**（那批没有
    `explanations.json`）；页面上有 500 条解读。报告本身仍含字段差异、样例、证据哈希。
  - 不能把两批拼起来补：`out-full`（11-14）与 11-18 的**数据事实完全一致**
    （declaredCols / sampleCols / emptyRate / hasPreview / sampleRows 零差异），
    但 **`score` 500 条全差**——评分口径在这之间变过。拼出来的报告会印一个与
    `data.json` 不同的分数，反而制造新的不一致。补齐只能整批重跑
    （audit 11.7 分钟 + 证据 500 + 证明 391 + 解读 500），赛前不做。

已完成（2026-11-19 · 产物与代码对齐）：

- 入库的 `web/data.json` 里 2,500 个部门仍用旧键名 `empty_avg`，而代码早已输出
  `emptyAvg`；前端靠 `alias` 兜住，所以一直没人发现。已重跑 `report` + `export` 对齐，
  并新增 `crates/core/tests/artifact_parity.rs`——拿入库产物与当前代码比键集合，
  注入一处漂移就会红（验过）。经验记入 `docs/ENGINEERING_LESSONS_ZH.md`。
- 待观察：`proc-macro-error2 v2.0.1`（经 `leptos_macro 0.7.9`）有 future-incompat
  （`pub use proc_macro` 将来变 hard error），上游问题，升级 leptos 才治得了。

已完成（2026-11-19 · 历史曲线 + 口径修正）：

- 新增 `crates/core/src/history.rs` + `assay history` + `history/summary.json`（D31）：
  每周追加一条**原始计数**，`export` 只读它并嵌进 `data.json`；站点两点以上画曲线，
  单点只报数字（单点画成横线会被读成「稳定」）。
- 站点新增「指标在变好还是变坏」区块（`crates/web/src/spark.rs`，坐标是纯函数带单测）。
- 顺手修掉一批**过期的定理数**：文档里广泛写着「100 条 / 2,355 定理」，实际入库的是
  **391 个可证数据集 / 4,970 条定理**（2,355 是 11-15 那次只跑了 100 条的旧值）。
- 修正不变量 5：报告集合 == 证据集合（500/500），证明是**子集**（391，`proof ⊆ evidence`），
  剩下 109 个平台只给表头、没有可证命题——原文写「三者集合必须一致」与实际不符。

已完成（2026-11-18 · 查询后端骨架）：

- 新增 `crates/api`：axum 0.8 + sqlx 0.8 + Postgres，四个**只读**端点
  （`/health`、`/v1/runs`、`/v1/datasets`、`/v1/datasets/{id}/history`）
- 边界：**证据不进库**。库里只有 `platform / run / dataset / observation`，
  服务挂了证据照样能离线验。理由见 `DECISIONS.md` D20
- 本地起法：`docker compose up -d db` → `just api`（或本机 postgres 建库，
  启动时自动跑 migrations）
- 实测过：4 个端点、分页夹紧、两个 404 分支、中文搜索、两次 run 的历史序列
- **尚未做**：任务提交、审计包签发、账号体系。`assay-api` 的覆盖率门禁已补进
  `scripts/coverage.sh` 的第二个口径（全量含 api，地板 72），不再是缺口。

已完成（2026-11-18 · 灌库闭环）：

- `assay-api ingest`：把一次跑批灌进库，**CLI 而非 HTTP 写接口**（见 D21）
- 行来自 `export::index_rows`，与静态 `index.json` 同一个 `row()` 派生，有单测逐字段比对
- 整批一次传 JSON（`jsonb_to_recordset`）：**24,322 条 3.4 秒**，幂等（同日重跑 run_id 不变）
- 修掉一个真缺陷：`/v1/datasets` 取「最近一次」原来用 `max(id)`，而 upsert 冲突会消耗
  序列、id 顺序不等于日期顺序——重灌一次旧日期会让默认榜单悄悄退版本。改按 `captured_at`
- 顺带修：空库时原来返回 500（`fetch_one` 的 `RowNotFound`），现在是 404「还没跑过体检」
- 实测跨两周历史：造 3 条变化，`/history` 正确返回两点，分数 9.0 → 7.5、分类
  「列数不一致」→「字段集合不一致」都能看见

已完成（2026-11-18 · 审计包）：

- `assay pack --captured-at <日期> [--commit <SHA>]` → `dist/assay-audit-<日期>.tar.gz`。
  对外交付**要传 `--commit`**（`just pack 2026-11-18 $(git rev-parse --short HEAD)`），
  否则 `MANIFEST.json` 的 `commit` 是 null，拿到包的人不知道对应哪版代码；实测带 commit
  连打两次字节一致。
- 包内自带 `VERIFY.md` + 标准 shasum 格式的 `MANIFEST.sha256`：对方解压后
  `shasum -a 256 -c MANIFEST.sha256` 一条命令验完，**不需要装本工具**
- 实测 100 条证据：3.6MB → **0.4MB**，206 条校验全 OK
- 篡改演示：改一个字节 → 只报那一个 FAILED；改 `MANIFEST.json` 把 `sample`
  改成 `full` → 清单自身 FAILED（连「把样例说成全量」都能抓住）
- **可重现**：mtime/uid/gid 固定 0，两次打包 sha256 一致（`c6f4c23b…`），
  所以 `dist/` 不入库也不丢信息
- 两级哈希并存：单文件哈希证明「没被改」，组合指纹
  `H(detail ‖ --split-- ‖ preview)` 证明「两个响应来自同一次抓取」（D22）

现场离线预案（已实测 2026-11-15）：

```bash
# 联网时做一次（vendor 目录 200MB，不要进 git）
cargo vendor vendor

# cargo 配置（可临时用 --config，避免把绝对路径写进仓库）
cargo build --release --offline -p assay \
  --config 'source.crates-io.replace-with="vendored-sources"' \
  --config 'source.vendored-sources.directory="vendor"'
# 实测：30 秒完成，exit 0

# wasm 也要离线时：同样加 --offline，wasm32 目标提前装好
cargo build -p assay-web --target wasm32-unknown-unknown --release --offline ...
# 注意：trunk build 会去下载 wasm-bindgen CLI，离线前先装一次：
cargo install wasm-bindgen-cli --version <与 Cargo.lock 一致>
```

待办（非阻塞）：

1. **第二个平台**（原定上海，2026-11-18 实测被挡，见 D25）：
   上海 `data.sh.gov.cn` 的 `/`、`/api/`、`/api/search` 全部返回 **HTTP 412** + 混淆 JS，
   是瑞数类动态防护，连接口路径都进不去；江苏 403/504；山东、北京的目录页是服务端渲染 HTML。
   **改候选为深圳**（`opendata.sz.gov.cn`）：勘察已做完，见 `docs/audit/SHENZHEN.md`。
   两件事已经明确：目录 `POST /data/api/findPageByParams` **匿名**返回 2,791 条接口
   （带字段数、数据量、更新周期），所以「停更」今天就能算；但**样例行要 appKey**，
   而 appKey 不是注册就有，要提交一个「应用成果」（选 web 应用，我们已有站点）
   并逐个接口订阅。
   拿到 key 后只剩三步：
   a) `./scripts/sz-probe.sh <appKey>` 打一次，确认返回是表头/行还是别的形状
   b) 把 `resId` 的斜杠换成下划线（写成斜杠会得到 200 + HTML，不报错）
   c) 定字段口径：深圳返回拼音缩写（`NF`/`NMCZRKS`），平台声明是中文，
      直接沿用浙江的「字段集合不一致」会把 2,791 条全标成问题
   然后写一个 `CatalogSource` 实现：七个方法（目录分页 / 目录全量 / `meta` / `rows` /
   `rows_from_raw` / `detail_raw` / `preview_raw`）。
   **核心库其余模块不需要改**——平台形状从 trait 边界上清掉了（D25），
   `tests/pipeline.rs` 的假平台不带任何真实平台 JSON 也能跑通全链路。
2. 证明从 100 扩到 500：已实测 63.9s / 5,923 条 / 506K，见 ARCHITECTURE 第 8 节；扩之前先决定要不要一起扩证据包（+16MB）

一个已经踩过的坑，别再踩：
`CatalogSource` trait 已经就位，换平台只需新增一个实现（见待办 1）。

另一个已经踩过的坑：
**trunk 默认生成绝对路径**（`/assay-web-*.js`），项目站点在 `/assay/` 子路径下会 404、页面空白。
`Trunk.toml` 里 `public_url = "./"` 不能删。更多见 `docs/ENGINEERING_LESSONS_ZH.md`。

## 3. 必跑命令

```bash
just check        # fmt(Rust+TOML) + clippy + nextest + cargo-deny + typos + 证明门禁，必须全绿
just clippy-wasm  # 前端也在门禁内
just lint-docs    # Markdown lint
just smoke        # 200 条端到端（联网，约 90 秒），产物进 out-smoke/，不覆盖正式产物
just all          # 全量（联网，约 14 分钟）
just drift        # 周度漂移对比：基线 vs 本轮 summary.json（0 正常 / 1 漂移 / 2 不可信）
just coverage     # 两数两地板：core+cli 76.88%（74）/ 全量含 api 73.81%（72）
just serve        # 本地预览 crates/web/dist
```

首次克隆后必跑 `just hooks`（装 commit-msg 钩子）。

单独跑：`just probe`（只验模型配置）、`just audit 8 0`、`just report`、`just explain 500`、`just evidence 500`、`just prove 500`、`just reports 500`、`just export 500`、`just export-split`、`just web`、`just tamper`、`just drift`、`just pack`、`just api`、`just api-ingest`。

查询后端（可选，不影响前面的静态链路）：

```bash
docker compose up -d db        # 或本机 postgres 建库，用 .env.example 里的 DATABASE_URL
just api                       # 启动时自动跑 migrations，监听 127.0.0.1:8081
curl localhost:8081/health

just api-ingest 2026-11-18     # 把 out-full 灌进库（幂等，同一天重跑会覆盖）
```

灌库是 CLI，不是 HTTP 接口（见 D21）。实测 24,322 条 **3.4 秒**，且重复 id 会在入口
被挡下并计数——一条脏数据不会让整批失败。

## 4. 硬规则

1. **口径不能悄悄改。** 分类（`scoring::classify`）、评分（`scoring::score`）、聚合、舍入
   都是对外口径。改了必须跑对齐检查，并在 `docs/DECISIONS.md` 记录为什么允许变化。
2. **禁止为了让测试过而改断言。** 先确认是实现错了还是口径本就该变。
3. **禁止编造运行结果。** 报告里的每个数字都要来自真实命令输出。跑不了就说跑不了。
4. **证明对象 = 存证对象。** `proof` 优先读 `evidence/<id>/preview.json`，不要改成现场重抓。
5. **哈希算原始字节。** 证据包用 `api::post_raw`；不要 `send_json` 后重新序列化再哈希。
6. **`proofs.json` 的 sha256 用 Python 兼容 JSON**（`[["a", "b"], ...]`，分隔符逗号+空格）。
   换成紧凑格式会让历史哈希全部漂移。
7. **基准日写死** `NOW_YMD = (2026, 11, 14)`，不读系统时间。
8. **模型不参与取数与算数。** 解释层输入只有事实字典；输出必须过 `guard::check`。
9. **任何 key 不进仓库/日志/聊天。** 本地走 `ASSAY_LLM_KEY` 或 Hermes 配置。
10. **不要引入 Python 运行时依赖。** 仓库目标是全 Rust；脚本只允许 bash（`ci/`）。
11. **前端只读。** WASM 只 GET `data.json`，不写、不调平台接口。
12. **CI 不访问政府网站。** 只用 `fixtures/` 与单测。

## 5. 代码地图

| 你要改… | 去这里 |
|---|---|
| 接口 / 重试 / 编码 | `crates/core/src/api.rs` |
| 体检逻辑 / 续跑 / 目录快照 | `crates/core/src/audit.rs` |
| 字段分类 / 评分 | `crates/core/src/scoring.rs` |
| 空值统计 | `crates/core/src/matrix.rs` |
| Lean 文本 | `crates/core/src/lean.rs` |
| 证明编排 / 调 lean | `crates/core/src/proof.rs` |
| 证据包 / 哈希 | `crates/core/src/evidence.rs` |
| 聚合 / 榜单 / 滞后天数 | `crates/core/src/aggregate.rs` |
| 前端 JSON | `crates/core/src/export.rs` |
| 单数据集报告 | `crates/core/src/single.rs` |
| 解释层 / 重试 | `crates/core/src/explain.rs` |
| 数字护栏 | `crates/core/src/guard.rs` |
| 周度漂移 / 阈值 | `crates/core/src/drift.rs`（口径见 `docs/audit/README.md`） |
| 命令入口 | `crates/cli/src/main.rs` |
| 前端视图 | `crates/web/src/app.rs` |
| 前端数据模型 | `crates/web/src/model.rs` |
| 前端样式 / 响应式断点 | `crates/web/style.css` |
| 分享卡片 / favicon | `web/og.png`、`web/favicon.svg`（`index.html` 里 copy-file） |
| 无障碍门禁 | `scripts/a11y.sh`（`just a11y`） |
| 发版 PR / 二进制资产 | `.github/workflows/release-please.yml`、`release.yml`、`.release-please-{config,manifest}.json`、`docs/RELEASE_CHECKLIST.md` |

## 6. 环境坑

- **代理**：本机访问 crates.io / GitHub 需 `HTTPS_PROXY=http://127.0.0.1:7890`。
  git push 用 `git -c http.proxy=http://127.0.0.1:7890 push`。
- **trunk**：`~/.local/bin/trunk`（v0.22.1）。首次构建会下载 wasm-bindgen 0.2.130（需网络）。
- **Lean**：4.27.0（`lean-toolchain`），`lean` 必须在 PATH。
- **wasm 目标**：`rustup target add wasm32-unknown-unknown`。
- **`strip = true` 不能开**：会让 wasm-bindgen 读不到 `target_features` 段。
- **平台脏数据**：存在 `2025-07-30 09:70:21` 这类非法时间戳，必须校验时分秒后跳过。
- **平台分页**：只认 `current`/`size`；`page` 会被静默忽略。
- **接口 id**：base64，含 `+ / =`，必须逐字符编码。

## 7. 怎么证明你没改坏

```bash
# 口径：拿冻结的明细重算，逐字段对比
mkdir -p /tmp/dp-parity && cp out-full/items.jsonl out-full/catalog_snapshot.json /tmp/dp-parity/
./target/release/assay --out /tmp/dp-parity report
# 期望：/tmp/dp-parity/summary.json 与 out-full/summary.json 完全一致（标量 + byDept）

# 证明：哈希与顺序（100 条，首条哈希与改动前一致，除非确实改了取数口径）
just prove 100

# 前端
just web && just serve   # 打开 8080，确认榜单 500 行、详情、证明页、返回键
```

### 按用户的路径走一遍（改前端后必做）

前三项可脚本化，后三项要在浏览器里实测 + 人眼看。

```bash
# 1) 五视图都能直达且刷新不白屏
for h in "" "#/checks" "#/detail/HuZoVx_QGmxtyEgbHu_Ulw%3D%3D" \
         "#/proof/HuZoVx_QGmxtyEgbHu_Ulw%3D%3D" \
         "#/dept/%E5%AE%81%E6%B3%A2%E5%B8%82%E7%BB%9F%E8%AE%A1%E5%B1%80"; do
  curl -s -o /dev/null -w "%{http_code} $h\n" "http://127.0.0.1:8080/$h"
done

# 2) 深链筛选：进 #/?dept=<部门名> 后列表要筛好，**下拉也必须显示该部门**
#    下拉靠 option 的 selected 同步；改回 prop:value 会被吞（options 还没渲染），
#    表现是列表筛好了、下拉仍写「全部部门」。

# 3) 移动端横向溢出：窄视口下 documentElement.scrollWidth 必须等于视口宽
#    真机口径 390px。桌面 Chrome 有 500px 最小窗口宽，别拿 --window-size=390 当手机。
#    用 iframe 固定 390 宽量：scrollWidth 应等于 390，且没有元素 right 超出视口。

# 4) 浏览器内重算：详情页点「在浏览器里重算 SHA-256」，结果必须是「一致」

# 5) 部门页口径：卡片上的 N（全量）和列表条数 M（榜上前 500）必须同时出现，
#    并写明「其余 N-M 条没进榜，不代表没问题」。只写 N 会被读成「藏了 N-M 条」。

# 6) 交叉引用闭环：证明页面包屑「榜单 / 详情 / 证明」→ 点「详情」回到判定现场；
#    「返回详情 / 新标签看报告 / 下载报告」三个入口都在；报告顶部两行回链的点开的 id
#    与当前条目一致（id 常带 + / =，编码错会落到「找不到这个数据集」）。
```

人眼两项：

- 手机宽度（≈390–500px）下：案例卡两栏要上下堆叠，8 张数字卡两列，页面不能左右晃。
- 分享出去：`index.html` 的 og: 标签要能在微信/Twitter 展开成卡片（改文案后重新生成 `web/og.png`）。

### 无障碍复查（改前端结构后必做）

```bash
just web && just a11y
```

`scripts/a11y.sh` 会起一个临时静态服务，把五个视图塞进 iframe，用 axe-core 跑
WCAG 2.1 A/AA + best-practice，**任一视图 `violations != 0` 就非零退出**。axe 只下一次，
缓存在 `~/.cache/assay-axe/`。

同一支脚本还会扫**渲染残留**：页面上出现 `{share}` 这种没被替换的花括号就算失败。
Leptos 把 `{var}` 写进字符串字面量时编译不报错、页面也不报错，只有人眼能发现——
部门页的「占全量 {share}」就是这么被扫出来的。

**`incomplete` 不等于通过。** axe 判不了的项目必须人工看，当前只剩两项，都是良性：

| 项 | 为什么判不了 | 结论 |
|---|---|---|
| 榜单 `.vs-arrow`（"→"） | 元素里只有非文字字符 | 对比度规则不适用 |
| 详情 `pre.mono-block` | 背景被其他元素部分遮挡 | 手算 #e8edf3 / #0f1216 = **15.95:1** |

不进 `just check`：CI 的 rust job 没有浏览器，装 Chromium 要拖慢整条流水线。
代价是它靠人记得跑——所以这一节和检查清单里都列了。

几个容易改回去的地方：

- `<main>` 包住视图、页头页脚在它外面；`main` 要带 `tabindex="-1"`，跳转链接才能把焦点送进去。
- 两处 `<nav>` 必须各有 `aria-label`（主导航 / 面包屑），否则地标重复。
- 下拉框和搜索框的 `aria-label` 去掉会立刻报 `select-name`（critical）。
- 标题层级不许跳级：案例卡曾是 `h3` 紧跟在 `h1` 后面。
- `pre` 有 `overflow-x: auto` 就必须 `tabindex="0"`，否则键盘滚不到右边。

## 8. 交付前检查清单

- [ ] `just check` 全绿（含 `cargo deny check`）
- [ ] `just clippy-wasm` 全绿
- [ ] `just smoke` 通过
- [ ] 口径对齐（若改了统计）
- [ ] `lean` PASS，`verify.txt` 为 PASS
- [ ] 前端五视图（榜单 / 详情 / 证明 / 判定规则 / 部门）可访问，浏览器返回正常
- [ ] 窄视口（≈390px）无横向溢出，分享卡片能展开（见上一节）
- [ ] `just a11y` 五个视图 `violations=0`，`incomplete` 项已人工确认
- [ ] `web/reports` 覆盖导出的前 N 名；`proofs.json` 是其中有数据行的那部分（现为 500 / 391）
- [ ] 文档更新（README / ARCHITECTURE / DECISIONS 至少一处）
- [ ] 没有 key、没有 `out-full/` 大文件进 git
