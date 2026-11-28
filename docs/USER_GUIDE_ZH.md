# 数验 Assay 使用指南

> 给第一次接触这个项目的人。读完你应该能回答三件事：这站点怎么看、我自己的数据怎么查、结果怎么读。
> 想看设计原理和取舍去看 [ARCHITECTURE.md](ARCHITECTURE.md)；想看换数据源的代码细节去看 [REUSE.md](REUSE.md)。

## 一、先搞清楚：网站是"展板"，工具才是"体检仪"

很多人第一眼会问："我自己的数据放哪进去？"——网站上没有输入口，这是**故意的**，不是没做完。

| | 网站（你打开的网页） | 命令行工具（下载的技能） |
|---|---|---|
| 是什么 | 一次真实跑批的**冻结快照** | 能对你自己的数据跑同一套检查的**体检仪** |
| 数据 | 固定是 2026-11-18 浙江 24,322 个数据集 | 你给什么就查什么 |
| 为什么这样 | 证据要能被任何人 checkout 复核，快照不能随用户改动而变 | 你本地跑，产物只在你自己的目录 |
| 能不能改 | 不能，也不该改 | 能，`--source` 指到你的文件 |

一句话：**网站是"证明这套方法能查出什么"的样品间；要查你自己的数据，用工具在本地跑。**

## 二、不装任何东西：直接看网站

打开 **https://qiaopengjun5162.github.io/assay/**

不用注册、不用登录、手机上也能开。三步走完：

1. **看最离谱的一条**：首屏那个案例——页面写 7 个字段、实际给 92 列，点"现场验一条"进详情。
2. **看概览图**：四张图把整批数据的更新周期、字段错位、停更部门、滞后分布画出来；点部门条能直接筛过去。
3. **自己验一条**：详情页拉到"证据包"，点 **"在浏览器里重算 SHA-256"**，浏览器当场把两份原始文件取回来重算哈希，逐字符对照页面上印的值。这一步不用装 Rust、不用开终端。

## 三、查你自己的数据（三步，可复制）

### 3.1 准备一个 JSON 文件

照着这个格式存成 `mydata.json`（字段说明见下表）：

```json
{
  "platform": "我的数据",
  "declaredColumns": ["title", "source", "url", "summary", "author", "publishedAt", "category"],
  "cycle": "按日",
  "items": [
    {
      "id": "a001",
      "title": "一条真实条目",
      "source": "来源名",
      "url": "https://example.com/1",
      "summary": "这里写摘要，太短会被检查抓出来。",
      "author": "作者名",
      "publishedAt": "2026-11-22",
      "category": "分类"
    }
  ]
}
```

最省事的起点：直接拿仓库里的 [`fixtures/daily-news.json`](../fixtures/daily-news.json) 改。

| 字段 | 必填 | 作用 |
|---|---|---|
| `platform` | 是 | 报告标题用，写你的数据源名字 |
| `declaredColumns` | 是 | 你**承诺**每条数据都有的字段。assay 拿它和实际给出的字段比对，缺的就是"货不对板" |
| `cycle` | 否 | 更新周期，如"按日""每周" |
| `items` | 是 | 你的数据条目数组，每条是一个对象 |
| `id` | 否 | 条目标识；没有时 assay 用 url 自动派生 |
| `publishedAt` | 否 | 发布时间，用来算"超期/停更"；支持 `2026-11-22` 和 RFC3339 |

> **一个常见坑**：`declaredColumns` 是你真的承诺会给的字段，别把所有可能字段都塞进去。
> 承诺多了、实际没给， assay 会正确报"字段对不上"——但那说明你的承诺写错了，不是数据有问题。
> 没有独立承诺方时（比如样例列名是按条目内容动态生成的），就直接把实际给出的列当声明列，这时一致性检查恒为 0 是**正确结果**。详见 [REUSE.md](REUSE.md) 的口径决策。

### 3.2 下载工具并跑一遍

到 [Releases](https://github.com/qiaopengjun5162/assay/releases/latest) 下载你平台的压缩包（macOS / Linux / Windows 五平台，每个带 `.sha256`），解压出 `assay` 二进制。

```bash
# ① 体检：--source 指向你的 JSON
./assay --out out-mine --source mydata.json audit --threads 2

# ② 聚合成汇总
./assay --out out-mine report

# ③ 逐项列出检查命中
./assay checks --items out-mine/items.jsonl --catalog out-mine/catalog_snapshot.json
```

不想下载二进制、本机有 Rust？在仓库根目录跑 `just reuse-demo`，用内置演示数据走完整条链。

### 3.3 看懂结果

`report` 会在屏幕上打印几个数，重点看这几个：

| 输出 | 人话 |
|---|---|
| `schemaMismatch` | 声明字段和实际字段对不上的条数 |
| `lagOver90Days` | 超过 90 天没更新的条数 |
| `emptyRateOver30` | 样例里空值率超过 30% 的条数 |
| `withPreview` / `audited` | 拿到样例的条数 / 总条数 |

`checks` 会逐条列出检查项和命中数，分母是**全量条数**，不是榜上前 500。

如果想把你的数据也变成网页那样的榜单和证据包（进阶），再加这两步：

```bash
./assay --out out-mine evidence
./assay --out out-mine prove
./assay export --items out-mine/items.jsonl --catalog out-mine/catalog_snapshot.json --evidence out-mine/evidence out-mine/data.json
```

## 四、历史/趋势：它是怎么攒出来的

网站首屏那块"指标在变好还是变坏"，现在只有一条真实跑批记录——**项目刚开始按周跑，这周才第一条**。它不会补假数据凑曲线（补了就等于画一条假趋势骗人）。

机制已经接好：每周的定时巡检会自动把这一周的七个原始计数追加到 `history/summary.json`（同一天重跑是替换、不重复画点），攒到第二条时网站会自动长出趋势线。想立刻看效果不用等——拿两周的数据各跑一次 `just audit` 即可，怎么挂定时任务见 [SCHEDULING.md](SCHEDULING.md)。

曲线只画**原始计数**（滞后数、错位数、整列全空数等），不画"问题分"：分数会随权重变，两把尺子画一张图上就没法比了。

## 五、常见疑问

**Q：准确性为什么不评价？**
没有权威源可比对，评了就是编。项目只查"说的和给的对不对得上、有没有按时更新、能不能取到样例"，这六维里准确性明确写着"不评价"。

**Q：样例级是什么意思？**
完整性来自表头 + 5 行样例，不是把平台全量几亿行都拉下来查。文案里不会说成"全量校验"。

**Q：网站上的 500 条和首页写的 24,322 个是什么关系？**
24,322 是全量体检数；网站榜单只展示问题最突出的前 500 条（每条都带证据包和报告）。部门卡里的数字是全量口径，榜单条数会单独标注，两个数不混。

**Q：我查出了很多"字段对不上"，是我的数据烂吗？**
先检查 `declaredColumns` 是不是真的承诺了这些字段。很多数据源没有独立声明方，这种情况把实际列当声明列即可（见第三节的坑）。

## 相关

- [README.md](../README.md) — 一句话介绍 + 30 秒看懂
- [REUSE.md](REUSE.md) — 换数据源的代码细节与口径决策
- [SCHEDULING.md](SCHEDULING.md) — 公众号日报体检的定时任务操作单
- [docs/OPS_HANDOFF_ZH.md](OPS_HANDOFF_ZH.md) — 给运维同事的交接单
