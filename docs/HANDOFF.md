# HANDOFF — 接手须知

> 给接手的 AI 或开发者。读完这份再动代码，能省掉大部分返工。
> 最后更新：2026-11-15

## 0. 项目位置

```
/Users/qiaopengjun/Code/assay
```

远程：`https://github.com/qiaopengjun5162/assay`

## 1. 当前状态（重要）

`main` 就是全 Rust 版本，已合并并上线（2026-11-15）。没有并行的旧版分支了。

已经验证过的事（不要重复怀疑，除非你有新证据）：

- `just check` 全绿（fmt / clippy core+cli / 45 个单测 / 证明门禁）
- `just clippy-wasm` 全绿
- 口径对齐：用旧 `out-full/items.jsonl` 重算，`summary.json` 标量与 **2,500 个部门全部一致**
- 证明对齐：100 条定理的 **SHA-256 与顺序全部一致**，`lean` PASS，2,355 条定理，15.3 秒
- 导出对齐：500 条、前 200 条关键字段 0 差异
- 前端：WASM 实测榜单 500 行、搜索过滤、详情六维、AI 解读、证据包、证明页 PASS、浏览器返回正常

## 2. 已完成 / 待办

已完成（2026-11-15）：

- CI：rust（fmt/clippy/test/build）、wasm（trunk 构建 + clippy）、lean（内核校验）、密钥扫描
- Pages 两段式：分支只构建 artifact，`main` 才部署 `crates/web/dist`
- 线上已切到 WASM 版：500 行榜单、详情六维、AI 解读、证据下载、证明页 PASS、浏览器返回正常
- `docs/DECISIONS.md` 已补 D12–D15

已完成（2026-11-18）：

- CI 从 4 个 job 扩到 8 个：新增 `msrv`（1.88 真编）、`coverage`（地板 52%）、
  `commit-lint`（Conventional Commits）、`docs`（markdownlint）
- `assay drift` + `audit.yml` 周度巡检，退出码三档（见 D19、`docs/audit/README.md`）
  首份基线 `baseline/summary.json`（24,322 条 / 抓取成功率 81.53%）
- 改名残留清理完毕：环境变量改 `ASSAY_LLM_KEY` / `ASSAY_CONFIG`，旧名过渡期仍可读
- ⚠️ **`AUDIT_PAT` 尚未配置**：漂移那一周会停在「检查 PR 凭据」并提示缺 secret

已完成（2026-11-18 · 查询后端骨架）：

- 新增 `crates/api`：axum 0.8 + sqlx 0.8 + Postgres，四个**只读**端点
  （`/health`、`/v1/runs`、`/v1/datasets`、`/v1/datasets/{id}/history`）
- 边界：**证据不进库**。库里只有 `platform / run / dataset / observation`，
  服务挂了证据照样能离线验。理由见 `DECISIONS.md` D20
- 本地起法：`docker compose up -d db` → `just api`（或本机 postgres 建库，
  启动时自动跑 migrations）
- 实测过：4 个端点、分页夹紧、两个 404 分支、中文搜索、两次 run 的历史序列
- **尚未做**：任务提交、审计包签发、账号体系，以及对 `assay-api` 的覆盖率门禁
  （`scripts/coverage.sh` 目前只统计 core 与 cli）

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

- `assay pack --captured-at <日期>` → `dist/assay-audit-<日期>.tar.gz`
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

1. **第二个平台**（上海 data.sh.gov.cn，11,477 个数据集）：先按这三步探明接口，再写一个 `CatalogSource` 实现——
   a) 打开数据集列表页，DevTools 里筛 XHR，找到列表请求的 URL、分页参数（浙江是 `current/size`，`page` 会被忽略）
   b) 找详情与预览两个端点，确认 id 是否需要 URL 编码（浙江是 base64，含 `+ / =`）
   c) 用 `curl` 直接打一次，确认返回结构里「声明字段」与「样例行」分别在哪一层
   拿到这三样，实现 7 个方法即可，后续采集/聚合/证明/导出不用改。
2. 证明从 100 扩到 500：已实测 63.9s / 5,923 条 / 506K，见 ARCHITECTURE 第 8 节；扩之前先决定要不要一起扩证据包（+16MB）

一个已经踩过的坑，别再踩：
`CatalogSource` trait 已经就位，换平台只需新增一个实现（见待办 1）。

另一个已经踩过的坑：
**trunk 默认生成绝对路径**（`/assay-web-*.js`），项目站点在 `/assay/` 子路径下会 404、页面空白。
`Trunk.toml` 里 `public_url = "./"` 不能删。更多见 `docs/ENGINEERING_LESSONS_ZH.md`。

## 3. 必跑命令

```bash
just check        # fmt(Rust+TOML) + clippy + nextest + cargo-deny + 证明门禁，必须全绿
just clippy-wasm  # 前端也在门禁内
just lint-docs    # Markdown lint
just smoke        # 200 条端到端（联网，约 90 秒），产物进 out-smoke/，不覆盖正式产物
just all          # 全量（联网，约 14 分钟）
just drift        # 周度漂移对比：基线 vs 本轮 summary.json（0 正常 / 1 漂移 / 2 不可信）
just coverage     # 行覆盖率地板 52%（实测 54.16%）
just serve        # 本地预览 crates/web/dist
```

首次克隆后必跑 `just hooks`（装 commit-msg 钩子）。

单独跑：`just audit 8 0`、`just report`、`just explain 100`、`just evidence 100`、`just prove 100`、`just reports 100`、`just export 500`、`just export-split`、`just web`、`just tamper`、`just drift`、`just pack`、`just api`、`just api-ingest`。

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

## 8. 交付前检查清单

- [ ] `just check` 全绿（含 `cargo deny check`）
- [ ] `just clippy-wasm` 全绿
- [ ] `just smoke` 通过
- [ ] 口径对齐（若改了统计）
- [ ] `lean` PASS，`verify.txt` 为 PASS
- [ ] 前端三视图可访问，浏览器返回正常
- [ ] `web/reports` 与 `proofs.json` 的 id 集合一致（同为前 100 名）
- [ ] 文档更新（README / ARCHITECTURE / DECISIONS 至少一处）
- [ ] 没有 key、没有 `out-full/` 大文件进 git
