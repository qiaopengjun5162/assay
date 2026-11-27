# v0.2.7 发布后终检（2026-11-27）

> 这是内核修复那一版（`catalog_all` 缺页不再静默吞掉）的终检。
> 前一天的 `FINAL_CHECK_2026-11-26_v0.2.6.md` 记的是数据源适配器那一版，不要混。

## 结论

`v0.2.7` 已发布：10 资产齐、5 平台哈希全对且全 LF、包内二进制 `assay 0.2.7` 实跑通过；
`verify-evidence` / `verify-reports` 均 500/500；第 ⑤ 步 `--source news:` 在**发布出去的
字节**上三项全 OK。仓库源码、manifest、tag 三者同为 `ba73cae` / `0.2.7`。

这一版进包的关键改动：

| PR | 内容 |
|---|---|
| #87 | `Client::catalog_all` 缺页不再静默吞掉——抽 `assemble_catalog_pages` 纯函数，缺页即报具体页号并中止（不变量 8：不可信跑批不往下游走） |
| #80 / #82 / #84 / #85 | 脚本侧：一条命令的日报体检、发报门禁三档、除零修复、导出前置检查（不在发布资产里，但随版本记录） |

站点**未重新部署**：这一版没动前端，线上仍是 `bf622dc` 的冻结构建，`verify-live.sh`
已验证逐字节一致（见 2026-11-27 工作记录）。

## 一、发版链路时点（UTC）

| 时刻 | 动作 | 结果 |
|---|---|---|
| 02:25 前 | release-please 更新 PR #78，纳入 #87 内核修复 | CHANGELOG 确认含 `c4f6a2d` |
| 02:26 | 合并发版 PR #78（squash） | main `ba73cae` |
| ~02:30 | `main` 的 `ci` | success（**点火前确认，这是真门禁**） |
| ~02:31 | `gh release edit v0.2.7 --draft=false` 点火 | tag 指向 `ba73cae` |
| ~02:31 | `release` 与 `push` 两个 run 同时起 | 留 `release`（45928862293），取消 `push`（45928862059） |
| ~02:37 | 5 平台构建 + 上传 | 全 success，10 资产 |
| 02:45 | `verify-release.sh v0.2.7` 终检 | 全过 |

注：发版 PR #78 上的 `ci` 显示 `action_required`/无 job 是常规的（release-please 用
`GITHUB_TOKEN` 建 PR，workflow 需人工批准），不是门禁；真门禁是 main push 的 `ci`
加 `release.yml` 的 verify job。

## 二、资产与哈希

| 平台 | 包大小（B） | `.sha256` 行尾 | 哈希 | 包内二进制 |
|---|---|---|---|---|
| linux-amd64 | 1,711,263 | LF | 一致 | `assay` (3,832,360 B) |
| linux-arm64 | 1,633,262 | LF | 一致 | `assay` (3,487,784 B) |
| macos-amd64 | 1,632,047 | LF | 一致 | `assay` (3,499,968 B) |
| macos-arm64 | 1,490,867 | LF | 一致 | `assay` (3,025,440 B) |
| windows-amd64 | 1,547,848 | LF | 一致 | `assay.exe` (3,033,600 B) |

10 个资产 = 5 包 + 5 份 `.sha256`，`draft=false pre=false`。

## 三、本机二进制实跑（macos-arm64）

```
macos-arm64: assay 0.2.7
verify-evidence: == 500 / 500 份一致
verify-reports:  == 500 / 500 份一致
```

第 ⑤ 步（`--source news:fixtures/qunmind-items.json` → `audit` → `checks`）：

```
OK  来源识别：platform='公众号日报（qunmind 条目）'
OK  条数：catalogTotal=4 items.jsonl=4
OK  一致性三项 0 命中
```

## 四、假发版 PR

发版后 release-please 又开了 #88（`release 0.2.8`）。按 `docs/RELEASE_CHECKLIST.md` §2
的**内容判据**核对：0.2.8 段共 75 条，引用的 71 个 commit 全部已在 main CHANGELOG 里
归属更早版本 → 假发版，已关闭并留 comment。行数判据在这里不适用（bot 单轮也可能只带
1 条，#71 就误判过）。`latest` 保持 v0.2.7。

## 五、文档口径同步

发版后发现 `HANDOFF.md` / `PROGRESS.md` / `PLAN.md` / `ROADMAP.md` 的「当前 latest」
还停在 `v0.2.3`（v0.2.4~v0.2.6 发版都没同步），`RELEASE_CHECKLIST.md` 停在 `v0.2.6`。
本次一并改到 `v0.2.7`；顺带把 ROADMAP/HANDOFF 里「假发版按增量行数辨别」的旧口径
改成「看条目是否归属历史版本」（#73 已修清单，但这两处漏了）。

## 六、回滚

删 tag `v0.2.7` 并把 release 标回 draft 即可；代码不用回退（#87 修复是对的，
回滚代码会重新引入静默缺页）。脚本/文档改动可单独 `git revert`。
