# 架构说明（ARCHITECTURE）

> 读这份文档的顺序：先看第 1 节的分层，再看第 3 节的模块契约，
> 最后看第 5 节的不变量——那几条是改动时最容易踩坏的。

## 1. 分层与依赖方向

```
┌──────────────────────────────────────────────────────────────┐
│  crates/web   Leptos CSR + WASM                              │
│  只读 data.json / evidence/* / reports/* / TopProofs.lean，不发写请求 │
└───────────────────────────▲──────────────────────────────────┘
                            │ web/data.json（静态文件）
┌───────────────────────────┴──────────────────────────────────┐
│  crates/cli   assay <audit|report|explain|evidence|      │
│            prove|reports|report-one|export|serve|tamper>     │
└───────────────────────────▲──────────────────────────────────┘
                            │ 直接调用库函数
┌───────────────────────────┴──────────────────────────────────┐
│  crates/core                                                  │
│                                                               │
│  纯函数（无 IO、无网络，CI 可全速跑）                          │
│    scoring  分类与评分                                        │
│    matrix   空值位图 / 列计数 / 空值率                        │
│    guard    数字护栏                                          │
│    lean     Lean 文本生成                                     │
│    util     日期 / 舍入 / 文件名                              │
│                                                               │
│  IO 层（显式接收路径与客户端，便于固定快照测试）                │
│    api      平台接口                                          │
│    audit    并发体检                                          │
│    evidence 原始响应存证                                      │
│    proof    快照 → 定理 → 调 lean                             │
│    aggregate 聚合                                             │
│    export   前端数据                                          │
│    explain  模型调用 + 护栏                                   │
│    single   单数据集报告渲染                                  │
│    config   key 读取                                          │
└──────────────────────────────────────────────────────────────┘
```

依赖只能向下：`web → cli → core`。core 不依赖 cli/web，纯函数不 import IO 层。

## 2. 数据流

```
平台接口
  │  api.rs（3 个端点，重试 3 次）
  ▼
audit.rs ──► out/items.jsonl            每条 = 一个 Dataset
         └─► out/catalog_snapshot.json  目录全量（补 updateTime）
                    │
                    ├─► aggregate.rs ──► summary.json / leaderboard*.md
                    │
                    ├─► explain.rs ────► explanations.json（护栏结果）
                    │
                    ├─► evidence.rs ───► evidence/<id>/{detail,preview}.json
                    │                     evidence.json（SHA-256 索引）
                    │
                    └─► proof.rs ──────► TopProofs.lean（2,355 条定理）
                                          │ lean（外部进程）
                                          ▼
                                        verify.txt + proofs.json
                    │
                    ▼
              export.rs ──► web/data.json
                                 │
                                 ▼
                        crates/web（trunk）──► crates/web/dist
```

关键点：**证明读的是 evidence 里已落盘的 `preview.json`**，不是重新抓取。
只有证据包不存在时才回退到实时抓取，并在 `proofs.json` 里标 `source: "snapshot" | "live"`。

## 3. 模块契约

### 3.1 `api.rs`

```rust
pub const BASE: &str = "https://data.zjzwfw.gov.cn/dopServer";
pub fn enc(id: &str) -> String;           // + / = 逐个编码
pub fn preview_rows(v: &Value) -> Vec<Vec<String>>;  // 第一行是表头

pub struct Client;
impl Client {
    pub fn catalog_page(&self, current: usize) -> Result<(total, pages, Vec<CatalogRecord>)>;
    pub fn catalog_all(&self, threads: usize) -> Result<Vec<CatalogRecord>>; // 保序
    pub fn detail(&self, id: &str) -> Result<Value>;
    pub fn preview(&self, id: &str, size: usize) -> Result<Value>;
    pub fn post_raw(&self, path: &str, body: &Value) -> Result<Vec<u8>>; // 存证用
}
```

- 分页参数必须是 `current` / `size`；`page` 会被服务端静默忽略并返回第一页。
- 失败重试 3 次，退避 300/600/900ms。
- `post_raw` 是给证据包用的：哈希必须算在原始字节上，不能算二次序列化结果。

### 3.2 `audit.rs`

```rust
pub struct AuditOptions { pub threads: usize, pub limit: usize, pub out_dir: String }
pub fn audit(opts, client) -> Result<AuditOutcome>;
pub fn read_items(path: &str) -> Result<Vec<Dataset>>;  // 跳过坏行
```

- `done.txt` 记录已完成条数，用于续跑；结果按索引写回，最后顺序落盘。
- **必须同时写 `catalog_snapshot.json`**——否则新环境跑 `report` 时滞后天数全空。
- 分类用的 `missing_fields` / `extra_fields` 保持声明顺序与表头顺序（与旧产物一致）。

### 3.3 `scoring.rs`

```rust
pub fn classify(d: &Dataset) -> SchemaKind;  // 五类，见 REQUIREMENTS 4.1
pub fn score(d: &Dataset) -> f64;            // 仅排序
pub const STOPPED_CYCLES: [&str; 3];
```

唯一口径来源。任何地方要判断「字段是否一致」，都调这里，不要自己写比较。

### 3.4 `lean.rs` / `proof.rs`

```rust
// lean.rs：纯文本生成
pub fn count_empty_theorem(dataset: usize, col: usize, count: usize) -> String;
pub fn rows_block(name, dataset, rows, ncols, rate_percent) -> String;
pub fn module(blocks: &[String], theorems: &[String]) -> String;

// proof.rs：编排
pub fn prove_top(items, top, out_dir, client) -> Result<ProofOutcome>;
```

定理形如：

```lean
theorem empty_0_3 : countEmpty rows_0 3 = 2 := by decide
```

- 数据以 `List (List Nat)` 嵌入，1 = 空值。
- `proofs.json` 的 `sha256` 是**样例行**的哈希，格式必须与
  `json.dumps(rows, ensure_ascii=False)` 一致（分隔符带空格），否则历史哈希全变。
- 定理数 = 每个数据集列数之和（当前 2,355）。

### 3.5 `evidence.rs`

```rust
pub fn capture_top(items, top, out_dir, client, threads) -> Result<EvidenceOutcome>;
pub fn hex(bytes: &[u8]) -> String;

// CLI: assay verify-evidence <索引> <证据目录>
// 索引支持 evidence.json（items[]）与 data.json（datasets[].evidence），逐份重算哈希比对
```

- 每条存 `detail.json`、`preview.json`（原始字节，不做任何转换）。
- SHA-256 = `sha256(detail || "\n--split--\n" || preview)`。
- 目录名 = `safe_id(id)`：只保留字母数字与 `- _`，长度 ≤ 64。
- 线程并发，索引按原顺序落盘。
- `verify-evidence` 是这个哈希的**逆操作**，进 `just check` 与 CI：改证据里一个字节就红。
  网站详情页把等价命令（`shasum -a 256`）直接印在证据面板上，评委不用装任何东西就能自己验。

### 3.6 `aggregate.rs`

```rust
pub const NOW_YMD: (i64, i64, i64) = (2026, 11, 14);
pub fn lag_days(update_time: &str) -> Option<i64>;
pub fn aggregate(items_path, catalog_path, out_dir) -> Result<Value>;
```

- 排序并列时按首次出现顺序（复刻 Python `Counter.most_common` / 稳定排序）。
- 所有比例保留 4 位小数，`round_to` 用格式化实现，与 Python `round` 同口径。
- 时间戳必须校验时分秒；平台存在 `09:70:21` 这类脏值，Python 会拒绝，Rust 也要拒绝。

### 3.7 `guard.rs` / `explain.rs`

```rust
pub fn check(text: &str, facts: &[(&str, String)], max_numbers: usize) -> (bool, Vec<String>);
pub fn dataset_facts(d: &Dataset, lag: Option<i64>, dept: &str) -> Vec<(&'static str, String)>;
```

- 数字归一：去千分位、去尾零；比例同时接受 `0.6667` 与 `66.67%`。
- 全文最多 3 个数字；空输出、结尾不完整（不以 `。！？”` 结尾）、数字越界都算不合格。
- 不合格重试 1 次；仍不合格标 `rejected`，前端不展示。
- 模型看不到原始数据，只能看 `dataset_facts`。

### 3.8 `single.rs`

```rust
pub fn render(d: &Dataset, lag, ev, rows, explain) -> String;
pub fn render_top(items, top, out_dir, dest, explanations) -> Result<RenderOutcome>;
pub fn load_explanations(out_dir) -> HashMap<String, String>;
```

- 一条数据集一页 Markdown：字段差异、原始样例、证据哈希、通过护栏的 AI 解读。
- `render_top` 的筛选与 `proof::prove_top` 同口径（`has_preview == 1` + 按 `score` 降序），
  报告集与证明集因此严格一致；文件名走 `util::safe_id`，前端用同一映射拼链接。
- 只收 `status == "pass"` 的解读，与页面同口径。

### 3.9 `export.rs`

```rust
pub fn export(opts: &ExportOptions) -> Result<ExportOutcome>;
```

- 输出单文件 JSON，字段名 camelCase，与前端 `model.rs` 一一对应。
- summary 里补 `proofCount` / `evidenceCount` / `explainCount`。
- 按 `score` 降序，`limit` 截断。
- `reports_dir` 下存在 `safe_id`.md 的行输出 `report: true`，前端据此渲染下载链接。

### 3.10 `crates/web`

- Leptos 0.7 CSR，trunk 打包；hash 路由：`#/`、`#/detail/<urlencoded id>`、`#/proof/<id>`。
- 数据只读 `data.json`；不做任何写操作、不调平台接口。
- 三个视图：榜单（搜索/部门/排序/三个筛选）、详情（六维 + 字段差异 + AI 解读 + 证据 + 证明入口 + 报告下载）、证明（PASS、哈希、定理示例、下载）。
- 浏览器前进/后退通过 `hashchange` 监听同步。

## 4. 产物契约

| 文件 | 生产者 | 消费者 | 稳定性 |
|---|---|---|---|
| `items.jsonl` | audit | aggregate / export / evidence / proof | 可重建 |
| `catalog_snapshot.json` | audit | aggregate / export / explain | 可重建 |
| `summary.json` | aggregate | export / data/ 快照 | **口径敏感** |
| `explanations.json` | explain | export | 需 API |
| `evidence.json` | evidence | export / proof | **哈希敏感** |
| `proofs.json` | proof | export | **哈希敏感** |
| `TopProofs.lean` | proof | lean | 可重生成 |
| `reports/<safe_id>.md` | single | 浏览器 / Pages | 可重生成 |
| `web/data.json` | export | 前端 | 可重生成 |
| `crates/web/dist` | trunk | 浏览器 / Pages | 可重生成 |

## 5. 不变量（改动时最容易踩坏）

1. **字段分类只有一处实现**：`scoring::classify`。任何地方复制这段逻辑都会分叉。
2. **证明对象 = 存证对象**：`proof` 必须优先读 `evidence/<id>/preview.json`。
3. **哈希算在原始字节上**：证据包用 `post_raw`，不要 `send_json` 后重新序列化。
4. **`proofs.json` 的 sha256 用 Python 兼容 JSON**（分隔符逗号+空格），否则历史哈希漂移。
5. **基准日写死**：`NOW_YMD` 是常量，不读系统时间；否则今天跑和明天跑结论不同。
6. **模型不参与取数、不参与算数**：解释层的输入只有事实字典，输出必须过 `guard::check`。
7. **前端零写入**：WASM 只 GET，不 POST；断网也要能看已加载的数据。
8. **CI 不访问政府网站**：CI 只跑单测与黄金文件（`fixtures/expected.lean`）。

## 6. 构建与部署

```bash
cargo build --release -p assay-core -p assay   # CLI
cd crates/web && trunk build --release                 # WASM → crates/web/dist
```

- GitHub Pages：`pages.yml` 在 CI 里装 trunk、构建 dist、上传 artifact、部署。
- 本地预览：`assay serve --dir crates/web/dist --port 8080`（Rust 静态服务，替代 `python -m http.server`）。
- 离线演示：`dist/` 里已含 `data.json`、`evidence/`、`TopProofs.lean`，不依赖网络。

## 7. 失败模式与处置

| 失败 | 表现 | 处置 |
|---|---|---|
| 平台接口变更 | 目录总数异常 / 详情 404 | 检查 `catalog_page` 的字段路径；快照产物不受影响 |
| 详情失败 | `detailError` 非空 | 计入失败数，继续跑，不中断整批 |
| 预览无数据 | `hasPreview == 1` 但无行 | 记 `sampleRows = 0`，分类为「无样例」 |
| Lean 未安装 | `require_lean` 报错 | 提示安装 elan；CI 单独装 |
| Lean 校验失败 | `verify.txt` 写 FAIL | 命令退出码 1，禁止发布 |
| 模型 key 缺失 | `explain` 报错 | 设置 `DATAPROOF_LLM_KEY` |
| 模型编数字 | 护栏 `rejected` | 重写一次；仍不合格不展示，产物里保留 `illegal` 供审计 |
| 数字漂移 | `summary.json` 与 `data/` 快照不一致 | 说明重跑了新快照；引用前确认时点 |

## 8. 扩展点

- **换平台**：`api.rs` 抽 `Trait CatalogSource`，把三个端点参数化；`audit` 不感知具体平台。
- **加指标**：在 `Dataset` 增加字段 → `audit` 计算 → `aggregate` 统计 → `export` 透出 → 前端展示。
- **扩证明**：`prove --top N`。实测（2026-11-15，M2 Pro）：
  100 条 → 2,355 定理 / 15.3s / 186K；500 条 → 5,923 定理 / 63.9s / 506K（含补抓 400 份预览）。
  一次 `lean` 编译约 60s 是可以接受的，但**扩证明必须同时扩证据包**，否则前 100 之外
  的定理会依赖现场抓取，破坏「证明对象 = 存证对象」。500 份证据约 16MB，Pages 与仓库都要掂量。
- **接交易所门禁**：把 `threshold` 做成配置，`prove` 的输出作为准入证据。
