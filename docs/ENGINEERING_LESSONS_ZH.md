# 工程经验库

已解决问题的长期、可检索记录。排障前先按关键词搜这里；修复后按模板补一条。

写一条的门槛（满足任一）：影响真实用户路径或结论可信度；根因不是一眼可见、很可能复现；
修复引入了新的边界、命令或回归测试。

每条必须包含：现象与影响范围、已确认根因、最终修复、防复发约束、可公开复核的证据。
还没验证的结论放 PR/issue，不要写成既定事实。

## 高优先级经验

### 模型会在 0 字节文档上编出「数据条数：382」

**现象：** 第一版用 `read_url` 读平台详情页，模型给出的体检测试里出现平台根本没提供的字段值，
数字看起来合理，实际全是编的。

**根因：** 平台是 JS 渲染，`read_url` 拿到 0 字节文档；模型不会说「读不到」，它按页面标题
补全了一个合乎想象的答案。

**修复：** 不再让模型读网页。体检改成直接打三个 JSON 接口的确定性抓取；
证据包对原始字节算 SHA-256；模型被限制在「只写人话解释、只许用事实字典里的数字」这一层。

**防复发：** 任何进结论的数字都必须来自代码计算，不许模型参与取数与算数；
新增指标先问「这个值是谁算的」，答案是模型就要重做。

**证据：** `docs/DECISIONS.md` D3 / D4，`crates/core/src/evidence.rs` 文件头注释。

### 语境漂移：Python 的 `score` 覆盖了 import

**现象：** 原型期三套语言各一份评分实现，改分类只改了两处，线上榜单与本地对不上。

**根因：** 同名函数在多语言里各写一遍，没有单一实现点，也没有对齐测试。

**修复：** 全量重写为 Rust 工作区，评分只有 `scoring.rs` 一处；重写期拿冻结的
`items.jsonl` 重算，逐字段对比 summary（含 2,500 个部门）。

**防复发：** 任何分类/评分逻辑只许写在 `scoring.rs`，别处复制即分叉；
重构必须附口径对齐输出。

**证据：** `docs/DECISIONS.md` D12 / D13。

### 跨批次补产物：每份报告都完整，读者核对却全部失败

**现象：** 想给 500 份 Markdown 报告补一段「AI 解读」，用现成的 `out-full`（11-14 批次）
跑 `just reports`。生成成功，500 份文件都完整，`diff` 也看不出结构性错误——只是每份
报告末尾的 `SHA-256` 指向 11-14 的证据文件，而仓库里入库的证据是 11-18 的。

**根因：** 报告、证据、`data.json` 是一次抓取的三份产物，必须同批。`--out` 指向
另一个批次时，`reports` 会读那一批的证据并如实写进报告——代码没错，
**错在拼料**。更麻烦的是没有任何检查会发现：每份报告自洽，CI 全绿，
只有真正去 `shasum` 的读者会看到「哈希不符」。

**修复：** 新增 `assay verify-reports <data.json> <reports> <evidence>`：按
`datasets[].evidence` 找出该有报告的条目，从报告正文抽两个哈希，与证据文件重算结果比对；
不一致时直接说「不是同一批抓取」。接入 `just check`。实测入库 500/500 一致；
故意混入一份另一批次的报告会红。

**防复发：** 产物**整批生成、整批替换**，不许跨 `--out` 拼。想补一段内容却只想重跑
一个步骤时，先问「这一步读哪个目录」——它读的目录决定了这批产物的时点。
另外，两批的数据事实（列数/空值率/样例）可能完全一致而**分数不同**（口径改过），
所以「事实一样就能拼」不成立。

**证据：** `crates/core/src/single.rs::verify_reports`，`just verify-reports`，
`docs/HANDOFF.md` 的 11-19 已知差异。

### 入库产物与代码的字段名会静默漂移

**现象：** `web/data.json` 里 2,500 个部门的平均空值率键名是 `empty_avg`，而当前
`aggregate::dept_object` 输出的是 `emptyAvg`。站点显示正常，部门榜空值率也是真数字，
所以肉眼查不出来。

**根因：** `web/data.json` 是 `assay export` 生成后**提交进仓库**的快照，不在编译路径上。
代码后来把 key 改成 `emptyAvg`（并给 `DeptStat.empty_avg` 加了 `skip_serializing`），
没人重跑导出，也没有任何检查比对两者。前端 `DeptRow` 上那行 `alias = "empty_avg"`
让症状消失——**兜底代码会把「契约不一致」变成静默状态**，比直接报错更难发现。
发现路径也值得记：不是读代码看出来的，是拿冻结明细重算对账时，
「归一化字段名后 0 差异、只剩键名不同」暴露出来的。

**修复：** 重跑 `just report && just export 500 web/data.json` 对齐 2,500 个部门；
`dept_object` 提为 `pub`；新增 `crates/core/tests/artifact_parity.rs`，
拿**入库的** `data.json` 与当前代码输出的键集合逐一比对。

**防复发：** 凡「代码生成、仓库入库」的产物（`data.json`、报告、证明文件），
其接口形状都要有测试当场比对，且必须验证过「注入一处漂移会红」——
没验过会红的门禁等于没有门禁。`serde(alias)` 只能用于兼容外部旧数据，
不许用来掩盖产物没重跑；写 alias 时注释要说清「在为谁兼容」。

**证据：** PR #25，`crates/core/tests/artifact_parity.rs`，
`crates/core/src/aggregate.rs::dept_object` 的文档注释。

### 浮点缩放取整让 0.15535 少了一位

**现象：** 迁移到 Rust 后，一个空值率与旧产物差 0.0001，榜单名次随之变化。

**根因：** `(x * 10000).round() / 10000` 里乘法引入了新误差：
`0.15535 * 10000 = 1553.4999999999998`，再套「半值取偶」得到 1553，而真实值略小于半值。

**修复：** `util::round_to` 走 `format!("{:.n}")` 再解析回来，与 Python `round()` 同口径。

**防复发：** 舍入一律走 `round_to`，不许手写缩放；
有 `round2_matches_python_half_even` 单测兜底。

**证据：** `docs/DECISIONS.md` D14，`crates/core/src/util.rs`。

### 平台时间戳有 `09:70:21`

**现象：** 全量跑批时部分条目解析失败。

**根因：** 平台脏数据，时分秒越界；按 ISO 直接解析会报错或错算滞后天数。

**修复：** 先校验时、分、秒在范围内，越界即跳过该时间戳，不当成有效更新时间。

**防复发：** 平台时间字段统一走校验函数；新平台接入先抽 100 条看非法值分布。

### trunk 默认绝对路径导致子路径部署白屏

**现象：** GitHub Pages 部署后页面全白，控制台 404 找 `/assay-web-*.js`。

**根因：** 站点在 `/assay/` 子路径下，trunk 默认生成根绝对路径。

**修复：** `crates/web/Trunk.toml` 设 `public_url = "./"`。

**防复发：** 这行不能删；改构建配置后必须实际打开子路径验证，不能只看 build 成功。

**证据：** `docs/HANDOFF.md` 第 2 节。

### 证明必须证存证过的那份字节

**现象：** 早期 `proof` 现场重抓预览再生成定理，证据包里的哈希与定理对应的数据是两批。

**根因：** 「证明」与「存证」各抓一次，中间平台改了数据就对不上。

**修复：** `proof` 优先读 `evidence/<id>/preview.json`，快照缺失才回退抓取；
`proofs.json` 记录的就是快照哈希。

**防复发：** 改动 `proof.rs` 的取数路径要同时改 `proofs.json` 的哈希口径，否则历史证明全失效。

**证据：** `docs/ARCHITECTURE.md` 不变量 2。

### 报告集与证明集错位会让页面链接 404

**现象：** 批量报告最初用手工 `report-one` 生成，与证明的筛选条件各写一套，
其中一处多了「名字不能为空」的过滤，两个集合开始漂移；页面若用 `proof.is_some()`
判断有无报告，就会给出死链。

**根因：** 选择口径被复制成两份；链接条件靠猜而不是靠事实。

**修复：** `single::render_top` 直接照抄 `prove_top` 的筛选（`has_preview == 1` + `score` 降序）；
`export` 改为检查文件是否真的存在，落 `report` 布尔字段，前端只认这个字段。

**防复发：** 新增「取前 N 名」的产物一律复用同一选择函数；集合一致性写进交付检查清单。

**证据：** `docs/DECISIONS.md` D16，`crates/core/tests/pipeline.rs` 的报告断言。

### gitleaks-action 在根提交上误报 exit 1

**现象：** 密钥扫描 job 成功扫到 0 条，却以退出码 1 失败。

**根因：** action 按 push 事件拼 `bb83aac^..HEAD`，而 `bb83aac` 是根提交，没有父提交
→ `ambiguous argument`。

**修复：** 不用 action，直接下 CLI 跑 `gitleaks detect --log-opts="--all"` 扫全部分支历史。

**防复发：** 不要换回 action；仓库重建后确认根提交不会让扫描范围塌陷。

**证据：** `.github/workflows/ci.yml` 里的注释。

### `target-cpu=native` 会做出「只能在编译机那一代 CPU 上跑」的产物

**现象：** 本机编译一切正常，CI 里偶发 `SIGILL`（Illegal instruction），而且不是每次都有，
报错位置也不固定——看着像随机崩溃。

**根因：** `.cargo/config.toml` 给所有非 wasm 目标加了 `-C target-cpu=native`。GitHub runner
不保证「编译这一台」和「运行这一台」是同一代 CPU，`rust-cache` 又把 `target/` 跨代缓存下来；
一旦命中不同代际的 runner，产物就会用本期 CPU 不支持的指令集。

**修复：** CI 里设置空的 `RUSTFLAGS`（环境变量优先于 config，空串同样生效），强制用默认目标 CPU。
本地保留 native——本机编译和本机运行永远是同一颗 CPU。

**防复发：** 新增 CI job 时确认它继承了 workflow 级 `env: RUSTFLAGS: ""`；跨机构建或分发二进制时
不能用 `native`。

**证据：** `.github/workflows/ci.yml` 顶部 `env` 注释；同一个坑 2026-10-29 在 `moonpub` 上
真崩过一次（rustc 直接死）。

### 「声明 MSRV」不等于「测过 MSRV」

**现象：** README 写「Rust 1.85+」，README/CI 都这么用了很久，从没人验证。

**根因：** `rust-version` 只是给 Cargo 的一个提示（影响 MSRV-aware resolver），编译器不会拿它
做任何检查；CI 一直用 `stable`，永远测不到下限。

**修复：** 用真工具链实测：1.86 直接编不过（`icu_collections` / `icu_provider` 等 2.4.0 要求
rustc 1.88），1.88.0 通过。据此把 `rust-version` 定为 1.88，并加 `msrv` job 用 1.88.0 +
`--locked` 真跑一次。

**防复发：** 改依赖后 MSRV 可能被静默抬高——`msrv` job 会当场红。别用「删掉 job」来修它。
`--locked` 必须留着：否则老工具链会自己解析出另一套更旧的依赖，测的就不是我们发布的东西。

**证据：** `cargo +1.86.0 check --locked` 的报错清单；`.github/workflows/ci.yml` 的 `msrv` job。

### 改了项目路径后，llvm-cov 会把旧产物算进来

**现象：** 项目从 `dataproof` 改名到 `assay` 后跑覆盖率，报告里同时出现 `dataproof/crates/...`（全 0%）
和 `assay/crates/...` 两套路径，总数比实际低十几到二十几个百分点。

**根因：** `target/llvm-cov-target` 里存着旧构建的 `.profraw` 与 instrumented 产物，
`cargo llvm-cov report` 会把目录下**所有** profraw 合并进来。`cargo llvm-cov clean --workspace`
没清干净（实测残留 427MB / 2849 个文件）。

**修复：** `cargo clean --target-dir target/llvm-cov-target` 彻底清，再重测：33.63% → **54.16%**。
`scripts/coverage.sh` 每次测量前先 `cargo llvm-cov clean --workspace`。

**防复发：** 覆盖率数字突然大幅下降、或出现两条不同前缀的同一文件时，先怀疑旧 profraw，
不要先怀疑「测试变少了」。

**证据：** `scripts/coverage.sh` 的清理步骤；本次测量前后的 `--summary-only` 输出对比。

### serial 主键跳号，不等于时间顺序

**现象：** 查询接口「取最近一次体检」写成 `select max(id) from run`，本地单元测试
（只有一条 run）全过。接上真实数据后，重灌一次旧日期就会让默认榜单悄悄退回旧版本——
不报错、不告警，只是数字变了。

**根因：** `run.id` 是 `bigserial`，而灌库用的是 `insert ... on conflict (platform_id, captured_at)
do update`。**upsert 命中冲突也会消耗序列**：同一天重灌两次，表里行数不变，序列已经 +2。
所以 `id` 的大小关系不再等于 `captured_at` 的先后关系。实测灌完两周后重灌第一周，
得到的 id 序列是 1、3、4，而日期是 11-18、11-25、11-01。

**修复：** 按业务时间取，别按代理键取：

```sql
select id from run where platform_id = $1 order by captured_at desc limit 1
```

`captured_at` 存 ISO 文本（D20 那条决策的副产品），字符串排序即时间排序，不需要额外索引。

**防复发：** 任何 `max(id)` / `order by id desc` 都在假设「代理键单调对应业务时间」。
这个假设在 upsert、批量导入、回填历史、跨库迁移之后都不成立。写完就问一句：
这条记录的时间是按什么排的？

**顺带修的另一处：** 同一函数里用了 `.fetch_one()` 配 `Option<i64>`，空表时返回的是
`RowNotFound` 错误而不是 `None`，被映射成 500「内部错误」。本意是给「还没有体检记录」，
实际给的是「服务坏了」。取「可能没有」的单行一律用 `fetch_optional()`。

**证据：** `crates/api/src/lib.rs` 的 `latest_run_id`；本次实测插入 `id=4, captured_at='2026-11-01'`
后，接口仍正确返回 `run 3 / 2026-11-25`。

### 库里 155 处 unwrap 不等于 155 个 bug——先分「自产自销」还是「吃外部输入」

**现象：** 扫 `crates/core/src` + `crates/api/src`，非测试代码里有 155 处 `.unwrap()`/`.expect()`，
集中在 `export.rs`(41) / `proof.rs`(20) / `pack.rs`(20) / `evidence.rs`(17)。
看到这个数字容易下结论「库代码到处 unwrap，遇到坏数据就 panic」，于是发起一轮大规模改写。

**判据（先看再改）：** unwrap 的风险取决于它吃的是谁的数据：

- **自产自销**（写自己刚生成的产物、读自己刚落盘的快照）：数据是本程序上一步写的，格式可控。
  `export`/`pack`/`proof`/`evidence` 多数属于这类，unwrap 失败等于「自己写坏了」，该炸。
- **吃平台抓来的原始响应**：这类才危险。逐个核对后，本项目里落在这一类的 unwrap **全在
  `#[cfg(test)]` 块里**（`api.rs:162/167` 的并发锁、`drift.rs:502+`、`ingest.rs:281+`），
  生产路径没有一处裸 unwrap 直接吃外部输入。
- **有前置不变量保证的**：`aggregate.rs:242` 的 `dept_map.get(k).unwrap()` 看着吓人，但
  `dept_order` 只在 `dept_map` 不含该 key 时 push（206~220 行），两者严格同源，get 不可能失败。
  这种 unwrap 是「把不变量写进代码」，比默默 `if let` 跳过更好——跳过反而会静默丢部门。

**结论：不用改。** 为凑「零 unwrap」去改写这 155 处，会把「自己写坏了就大声炸」变成
「静默跳过坏数据」，对一个卖点是「结论可复核」的项目是倒退。
遇到 unwrap 先问一句：它读的是谁写的数据、写失败了要不要炸。

**另附（本轮只读体检发现，不动）：** `just clippy-wasm` 通过；`cargo tree -d` 的重复版本
都是 leptos / sqlx 各自传递依赖的分层，没有 semver 冲突。但 wasm 构建报一条
`proc-macro-error2 v2.0.1` 的 future-incompat（E0365：re-export 私有 `proc_macro` crate），
来源是 leptos 0.7 宏链（`leptos_macro` / `rstml` / `reactive_stores_macro`）。
当前 Rust 1.100 只是警告，不影响编译与门禁；未来某版会变硬错误，需等上游更新。
**赛前冻结期不要为此 `[patch]` fork 或升 leptos**——为消除一个「未来的」警告去动前端宏链，
风险远大于收益。等上游出修复版本再说。

### trait 签名里的 `&'static str` 会逼运行时适配器 leak——该 borrow `&self` 就 borrow

**现象：** `CatalogSource::platform()` 原本返回 `&'static str`。内置浙江 Client 返回编译期
字符串字面量，天然满足；但第二个适配器（`fixture_source`）的平台名来自运行时 JSON 文件，
为了对齐 `'static` 只能 `Box::leak(f.platform.into_boxed_str())`——每构造一份 source
就永久泄漏一份字符串。单次 CLI 运行只泄漏一份，看不出问题；赛后服务化（每租户/每请求
一份 source）就会累积。

**判据：** 一个"展示用"方法不该逼实现者做内存管理。先查它的实际调用面：
`grep -rn '\.platform()' crates/` 显示生产代码**零调用**（只有测试断言），冻结产物里
也没有 platform 字段——说明这是死接口，为它 leak 完全不值。

**做法：** 把签名放宽成 `fn platform(&self) -> &str`。约束从 `'static` 放宽到 `&self`，
方法集合不变，所有返回字面量的既有实现照旧兼容；运行时适配器改成按值存 `String`、
返回 `&self.platform`，随 source 一起释放。**放宽生命周期约束是向后兼容的重构**，
收紧则不是——改之前先确认没有调用方把返回值存进需要 `'static` 的地方。

**回归测试：** 加一条"多 source 共存、平台名互不覆盖、一份释放后另一份仍可读"的测试，
锁定服务化契约（旧的 leak 方案功能上也通过，但新方案若写错会直接编译失败/串味）。

**结论：本轮只改 trait 签名 + 4 处实现 + 1 处字段类型，未改任何产物与检查逻辑。**
