# 复用：从参赛作品到能干活的 Agent

> 这份文档回答一个问题：比赛结束之后，assay 能不能不只是留下一份榜单，
> 而是变成一个持续解决自己/公司实际业务问题的工具？答案是能，而且大部分零件已经就位。

## 一句话

assay 的真正资产不是那个榜单，而是「**说的和给的对不对得上、并且每条结论你都能自己验**」这套内核。
它是平台无关的。榜单只是这套内核在「浙江开放数据」这一个数据源上的一次输出。

## 三层结构

```
参赛外壳（一次性）    榜单网站 / 路演稿 / 冻结快照  —— 12-08 之后归档即可
─────────────────────────────────────────────────────────
可复用内核（资产）    体检 · 证据 · 证明 · 检查项 · 门禁  —— 换数据源不用改
─────────────────────────────────────────────────────────
调度与触达（要接）    定时跑批 · 推给人  —— 已有现成零件，见下
```

内核平台无关的**证据**：`CatalogSource` trait（`crates/core/src/api.rs`）只要求实现七个方法，
`audit` / `evidence` / `proof` 全部接收 `&dyn CatalogSource`。
本仓库已有两个实现：内置浙江 `Client`，和 `crates/core/src/fixture_source.rs` 的本地 JSON 适配器。

## 赛前已做的最小演示（可现场复现）

```bash
just reuse-demo
```

它用 `fixtures/daily-news.json`（公众号日报的新闻条目形状）当数据源，
走 `audit → report → checks` 全链路，产物写到临时目录，**不碰 `web/` 冻结快照**。
实测输出：4 条新闻里查出 3 条字段对不上、2 条字段说明不足 10 字、1 条超 90 天没更新。

这证明：**换一份完全不是政府开放数据的数据，同一套内核照样查出问题。**
路演可以据此说一句：这是 AI 基础设施，不是一次性脚本。

### 字段是怎么映射的

一个新闻条目被当成一个「数据集」体检：

| 日报/新闻概念 | 映射到 assay | 触发的检查 |
|---|---|---|
| 模板声明的字段全集 | `declaredColumns` → 声明字段 | 与非空实际字段比差集 → 字段对不上 |
| 条目实际返回的字段 | 非空字段 → 样例表头 | 缺摘要/作者 → `missingFields` |
| 发布时间 `publishedAt` | 目录 `updateTime` | 超期 / 停更检查 |
| 摘要 `summary` 字数 | `descriptionLen` | 说明过短检查 |

于是「日报声称每条都有摘要和作者，某源实际只给了标题和链接」这种货不对板，
用的是和 24,322 个政府数据集**完全相同**的 10 条检查，不是另写一套。

## 赛后接真实业务：公众号日报体检

选定的第一个落地场景：每天 08:00 公众号日报推送**之前**，先验当天抓的数据有没有货不对板，
有问题先拦下来，别把缺字段、超期的内容发出去。

### 已完成：数据源适配器（2026-11-26）

`--source news:<path>` 直接读 qunmind 的 `PublicNewsItem` 数组（snake_case、无 id、
Option 字段），**上游不用改数据结构**：

```bash
assay --out /tmp/x --source news:fixtures/qunmind-items.json audit --threads 2
assay --out /tmp/x report
assay checks --items /tmp/x/items.jsonl --catalog /tmp/x/catalog_snapshot.json

# 或者一键跑完三步并逐条列出缺哪个字段
just reuse-news
```

实现：`crates/core/src/news_source.rs`（`NewsSource`），CLI 分支在 `make_source()`。

三条口径决策，改动前先读：

- **声明列只取 7 个**（title / source / url / summary / author / published_at / category）。
  上游的 `score` / `comments` / `ai_score` 是排序用的内部量，不是对读者的承诺；把它们
  算进声明列，会把「内部字段为空」误判成「日报缺字段」——那是口径错误，不是数据问题。
- **id 由 assay 派生**：url 的 sha256 前 16 个 hex；没有 url 时退化到 `title|published_at`。
  不用 `safe_id(url)`——长链接会被截断到 64 字符而撞车，中文标题更会被替换成一串下划线。
- **RFC3339 时间直接可用**：`parse_update_time` 按固定位置读日期与时分秒，
  `2026-11-25T08:00:00+00:00` 能正常算 `lagDays`。有单测钉住这条——若改成「只认空格
  分隔」，超期检查会静默失效，而且不报错。

实测（`fixtures/qunmind-items.json`，4 条）：3 条字段对不上、2 条说明过短、
1 条超 90 天未更新。用的是和 24,322 个政府数据集同一套检查项。

### 还差什么（按顺序）

1. **qunmind 导出条目**：`PublicNewsItem` 目前只 derive 了 `Debug/Clone/PartialEq`，
   没有 `Serialize`。加一行 `Serialize`，再给 CLI 一个 `export-items --output items.json`
   （内部调 `CompositePublicNewsSource::fetch_top_items()` 后写数组）。**上游改动就这两处。**
2. **接调度**：用现成的 Hermes cron，把 `audit → report` 挂在日报推送前；
   退出码或命中数超阈值就不发报、报警。
3. **接触达**：结果推到 WeCom（小满/二花已有通道），或写进 Obsidian 知识库存档。

内核（audit / evidence / proof / checks / 门禁）**一行不用改**。

## 边界与诚实说明

- **CLI 编排层不是完全零改动**：`--source` 这个选择点是本次新加的；接真实源时要在这里加一个分支。
  内核层确实零改动，但「整个程序零改动」是假的，别对外这么说。
- **准确评价仍不做**：assay 回答「说的和给的对不对」，不回答「新闻内容是不是真的」。
  后者需要权威源，没有权威源去评就是编（见 `docs/DECISIONS.md`）。
- **超期阈值复用政府数据的口径**（90 / 365 天）。新闻是日更内容，这套阈值偏宽松；
  接日报时应在 `checks.rs` 里加新闻专用阈值——但这就是「改尺子」，要走口径对齐流程。
- **演示数据是编的**：`fixtures/daily-news.json` 里的条目是构造的，只为演示检查链路，
  不是真实抓取结果。赛后接真源时替换掉。

## 相关

- `docs/PITCH.md` — 比赛路演口径
- `docs/ARCHITECTURE.md` — 内核分层与不变量
- `docs/DECISIONS.md` — D20（证明针对快照）、D31（曲线只记原始计数）
