# 公众号日报体检 · 上线交接单

> **给谁**：能登录 Hermes 服务器的同事。这份只管「照着敲」，每步都给命令和「做完的标志」。
> **原理 / 阈值依据**：看 [SCHEDULING.md](SCHEDULING.md)，别在这份里找。
> **边界**：本会话（AI）不碰生产服务器、不改 qunmind 配置、不碰密钥。下面所有命令都是同事在自己的登录会话里跑。

## 0. 三句话讲清这是个什么东西

1. 以前每天到点就把新闻发出去，发完才发现里面混了一年前的旧闻，或者多数条目只有标题。
2. 现在在发送前加一道安检：先给素材做体检，体检不过就先拦下来、叫人看一眼。
3. 安检只判断「这批数据可不可信」，不评新闻真假，也不写日报正文。

它是一道**旁路检查**，不是新起一个发报任务——原来的日报链路一点不动。

## 1. 现在卡在哪、谁做什么

| 角色 | 做什么 | 做完的标志 |
|---|---|---|
| AI（已交付） | 体检脚本、门禁脚本、三档阈值、CI 冒烟、本交接单 | 已完成 |
| 能登录 Hermes 的同事 | 确认定时器在哪、照第 3~5 节挂上、空跑一周 | `crontab -l` 能看到那一行 |
| 编辑 / 值班人 | 收到「拦截」通知时，看摘要决定补说明还是放行 | 值班群能收到通知文本 |

⚠️ **通知没接好之前，cron 只记日志、不要真的拦发报**。拦了却没人知道，比不拦更糟。

## 2. 开工前先确认两件事

### 2.1 现有日报的定时器在哪（别挂两遍）

同一个日报挂两遍会发两篇。先看现有链路是哪种：

```bash
# A. Hermes cron（本机 Hermes 服务）
hermes cron list 2>/dev/null | grep -i '日报\|report'

# B. qunmind 自己的调度配置
grep -n -A5 'daily_reports' /path/to/qunmind/config.toml

# C. systemd timer（服务器上）
systemctl list-timers | grep -i 'qunmind\|report'
```

三处最多只能有一处在发日报。门禁是旁路，只加一道检查。

### 2.2 确认 qunmind 二进制够新（最容易卡这一步）

`news-check.sh --export` 要调 qunmind 的 `export-items` 子命令，它是 2026-11-26 才进 qunmind `origin/main` 的（提交 `66db373`）。**旧构建不含它**，拿旧二进制跑会报「未知子命令」，看着像配置问题，其实是版本问题。先验：

```bash
/path/to/qunmind/target/release/qunmind --help | grep export-items
```

有输出 → 继续。没输出 → 更新后重建：

```bash
cd /path/to/qunmind && git fetch origin && git checkout origin/main \
  && cargo build --release -p qunmind
```

## 3. 本地先验收（别直接上 cron）

```bash
cd /path/to/assay

QUNMIND_BIN=/path/to/qunmind/target/release/qunmind \
QUNMIND_CONFIG=/path/to/qunmind/config.toml \
bash scripts/news-check.sh --export --gate -o out-news
```

**做完的标志**：看到 `GATE=PASS`。

若是 `GATE=UNTRUSTED`，先看提示里的具体原因：

- 「条目只有 0 条」→ `config.toml` 里可能没启用任何 `public_sources`；
- 「产物过期」→ 导出那步没成功，读到了上次的旧文件。

## 4. 挂上去（建议 07:55，日报 08:00 前 5 分钟）

```bash
55 7 * * * cd /path/to/assay && \
  QUNMIND_BIN=/path/to/qunmind/target/release/qunmind \
  QUNMIND_CONFIG=/path/to/qunmind/config.toml \
  bash scripts/news-check.sh --export --gate -o /path/to/assay/out-news \
  >> /var/log/assay-news-gate.log 2>&1
```

**做完的标志**：`crontab -l` 能看到这一行。

### 4.1 日志要轮转（别漏）

cron 每次把整段输出追加到同一个文件，不轮转一年后会长到几 MB。加一个 `/etc/logrotate.d/assay-news-gate`：

```text
/var/log/assay-news-gate.log {
    daily
    rotate 14
    missingok
    notifempty
    compress
}
```

保留 14 天，够覆盖观察窗口。

## 5. 退出码怎么处理

| 退出码 | 屏幕上的字 | 人话 | 对日报链路做什么 |
|---|---|---|---|
| `0` | `GATE=PASS` | 可发 | 什么都不用做，日报按原节奏发 |
| `1` | `GATE=BLOCK` | 先别发 | **空跑期先只记日志**；接好通知后：跳过本次推送，通知人确认 |
| `2` | `GATE=UNTRUSTED` | 这趟车本身坏了 | 跳过推送并报警：「体检没做成」和「素材有问题」是两回事，措辞别混 |

⚠️ 通知的原因和三个数散在摘要里，**不要只发最后 20 行**，截掉就丢了上下文。用第 6 节拼整段。

## 6. 通知文本怎么拼（不用装新脚本）

体检摘要和门禁结论在第 4 节的日志里已经齐了。下面这条命令只取**最后一次运行**的整段输出（每遇到一个新的「公众号日报体检」就丢掉前面所有内容），复制出来贴值班群即可：

```bash
LOG=/var/log/assay-news-gate.log
awk '/公众号日报体检 · /{buf=""} {buf=buf $0 "\n"} END{printf "%s", buf}' "$LOG"
```

贴出来长这样（`GATE=` 那行是关键：`PASS` 不用动，`BLOCK`/`UNTRUSTED` 叫人看一眼）：

```text
公众号日报体检 · 公众号日报（qunmind 条目）
条目 61 条 / 来源 9 个：ArXiv AI(15)、Hacker News(11)…
① 三个数
   说明不足：51 条
   超 90 天未更新：1 条
   一致性错位：0 条
③ 产物
   out-news/items.jsonl / summary.json / checks.txt / leaderboard.md
── 发报门禁 ──────────────────────────────────────────
GATE=PASS
可发报：61 条，抓取成功率 100.0%，一致性错位 0，超 90 天 1.6%。
```

发到小满 / 二花的现有 WeCom 通道，还是先贴值班群人工转，由能登录的同事定。**本会话不内置 webhook、不读密钥、不代发消息。**

## 7. 观察窗口（先空跑一周）

1. **第 1 周**：门禁只记日志，不拦发报。每天看 `GATE=` 那一行和条目数。
2. **第 2 周**：确认零误拦后，才把 `1` / `2` 接到「跳过推送」。
3. **上线后**：盯 7 天；出现一次 `UNTRUSTED` 就去查上游，**不要直接放宽阈值**。

阈值校准办法和依据见 [SCHEDULING.md](SCHEDULING.md) 第三节。

## 8. 回滚（门禁是旁路，退起来很轻）

```bash
# 1) 注释掉第 4 节那一行 cron（别删日志）
# 2) 日报链路立刻恢复原样，不需要重启任何服务
# 3) 若要撤掉脚本本身：git revert <本 PR 的 squash commit>
```

脚本产生的 `out-news/` 在 `.gitignore` 的 `out-*/` 里，不会进 git；要清理直接删目录即可。

## 9. 明确不做的事

- 不评新闻真假：没有权威源，评了就是编。
- 不生成日报正文：门禁只回答「发不发」，写稿仍由 qunmind 日报链路负责。
- 不改生产配置、不读密钥：本会话只交付脚本与本交接单。

## 附：真实数据实测长啥样（2026-11-26，本机）

用 qunmind 现有配置文件读真实公共源，跑完整条链：

```bash
QUNMIND_BIN=/path/to/qunmind/target/release/qunmind \
QUNMIND_CONFIG=/path/to/qunmind/config.toml \
bash scripts/news-check.sh --export --gate -o out-news-real
```

实测 61 条 / 9 来源，抓取成功率 100%，一致性错位 0，超 90 天 1/61 = 1.6%，`GATE=PASS`。
「说明不足」51/61 = 84%，**只提醒不拦**（拿它当门禁等于天天不发报）。

反向验证：同一批产物不改数据，只把时效阈值收紧到 1%（`GATE_MAX_LAG_RATIO=0.01`），
门禁立刻变 `GATE=BLOCK`、exit 1，并指出具体超标数。说明它真的在判数据，不是一律放行。

> `config.toml` 含凭据，实测时只传路径、未读取或打印内容。
