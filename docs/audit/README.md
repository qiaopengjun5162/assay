# 周度巡检（audit.yml）

把「一次性体检」变成「每周体检，指标漂移就开 PR」。

这件事只做**巡检**，不重发布：站点数据、证据包、Lean 证明仍由人工跑
`just all` 更新。巡检的产物只有两个——基线 `baseline/summary.json` 和报告
`docs/audit/latest.md`。

## 三档处置

| 退出码 | 判定 | 动作 |
|---|---|---|
| 0 | 正常 | 基线**不动**，job 绿 |
| 1 | 漂移 | 写新基线 + 写报告 → 开 PR 让人确认「是否接受为新常态」 |
| 2 | 跑批不可信 | 基线**不动**，job 红，不开 PR |

基线代表「上次人工确认过的常态」，只有漂移 PR 合并才会前进。这样渐进漂移
会一周周累积，直到越过阈值才报，而不是每周跟着数据走、永远报不出来。

## 为什么 `audited` 变化不告警

平台增删数据集是常态，总量涨了不代表质量变了，所以「体检条目数」这一行只记录。
真正要盯的是**占比**：字段一致性问题、停更、整列为空、声称周期已逾期、无列清单，
全部按「占 `audited` 的百分比点数（pp）」比。

## 阈值（经验值，必须按实测校准）

| 指标 | 单位 | 默认阈值 |
|---|---|---|
| 字段一致性问题占比 | pp | ±2.0 |
| 停更占比 | pp | ±2.0 |
| 声称周期已逾期占比 | pp | ±2.0 |
| 整列为空的数据集占比 | pp | ±2.0 |
| 无列清单占比 | pp | ±2.0 |
| 空值率中位数 | 百分点 | ±5.0 |
| 中位滞后天数 | 天 | ±14 |
| 体检条目数 | 条 | 只记录 |

另有两条**跑批健康闸门**（不判漂移，直接判不可信）：

- 抓取成功率（`withPreview / audited`）下跌 > 5pp
- 目录条目减少 > 20%

## 校准步骤

阈值是拍的，别当定理。连续跑 4 周后按真实波动收：

1. 从 `Actions → audit → 产物` 下载每周的 `summary.json`
2. 算每项指标相邻两周的差值，取 P95
3. 阈值 = P95 × 2（留一倍余量，避免天天报警）
4. 改 `crates/core/src/drift.rs` 的 `Thresholds::default()`，同步改上表

调阈值必须先改代码，不要在 workflow 里传 `--ratio-pp`：那样阈值就游离在
门禁之外，`just check` 拦不住它被改坏。

## 一次性配置：`AUDIT_PAT`

**必须配**，否则漂移 PR 开不出来。原因：GitHub 防递归，用默认
`GITHUB_TOKEN` 创建的 PR 不会触发 `ci.yml`，会永远卡在
「Expected — waiting for status to be reported」，没法按模板「CI 绿了才合」。

1. GitHub → Settings → Developer settings → Personal access tokens →
   **Fine-grained tokens** → Generate new token
2. 只授这个仓库：`Contents: Read and write`、`Pull requests: Read and write`
3. 有效期建议 1 年，到期前一天 GitHub 会提醒
4. 仓库 → Settings → Secrets and variables → Actions → New repository secret →
   名字 `AUDIT_PAT`，值粘 token

没配也能跑：job 会在「检查 PR 凭据」那步红，并明确提示缺 `AUDIT_PAT`。

## 手动试跑

Actions 页面 → audit → Run workflow，`limit` 填 `200` 可以先小样本验证链路
（几秒跑完）。小样本会跳过漂移判定——200 条相对 24,322 条必然触发「目录条目
骤减」闸门，判了也是白红一次。

本地等价命令：

```bash
just audit-weekly      # 全量体检 + 漂移判定（只预览，不写基线）
```

## 已知坑

- **schedule 会被自动停用**：仓库连续 60 天无提交，GitHub 会停掉定时任务，
  需要手动点一次 Enable。巡检本身不产生提交，所以这条一定会遇到。
- **周一 02:00 UTC**＝北京时间周一 10:00。cron 只认 UTC，别写成 `10:00`。
- **平台抖动**：单条抓取失败是容错的，只有成功率整体塌方才判不可信。
  如果连续两周都判不可信，先查平台接口是否改了返回结构，再考虑放宽阈值。
