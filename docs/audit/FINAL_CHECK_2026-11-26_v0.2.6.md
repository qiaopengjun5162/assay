# v0.2.6 发布后终检（2026-11-26）

> 同一天还有一份 `FINAL_CHECK_2026-11-26.md`，那是 **v0.2.5**（前端友好度）的终检。
> 本文件只记 v0.2.6——数据源适配器那一版，两件事不要混。

## 结论

`v0.2.6` 已发布：10 资产齐、5 平台哈希全对且全 LF、包内二进制 `assay 0.2.6` 实跑通过。
旧路径两道门禁 500/500，**新功能 `--source news:` 也在发布出去的字节上跑通了**（第 ⑤ 步，
本次新增）。站点未重新部署——这一版没动前端，线上仍是 `bf622dc` 的构建。

这一版进包的两件事：

| PR | 内容 |
|---|---|
| #75 | `--source news:` 直读 qunmind 的 `PublicNewsItem` 原样数组（snake_case / 无 id / Option 字段） |
| #77 | 声明列改为实际列（`schema-*` 不再 100% 全中）、榜单标题不再写死「浙江」 |

## 一、发版链路时点

| 时刻（UTC） | 动作 | 结果 |
|---|---|---|
| 06:27 | 合并发版 PR #76 | main `daaf669` |
| 06:29 | main push 的 `ci` | success（点火前已确认，这是真门禁） |
| 06:30 | `gh release edit v0.2.6 --draft=false` 点火 | tag 指向 `daaf669` |
| 06:30 | `release` 与 `push` 两个 run 同时起 | 留 `release`，取消 `push` |
| 06:39 | 5 平台构建 + 上传 | 全 success，10 资产 |

## 二、资产与哈希

| 平台 | 包 | `.sha256` 行尾 | 哈希 | 包内二进制 |
|---|---|---|---|---|
| linux-amd64 | 3,826,872 B | LF | 一致 | `assay` |
| linux-arm64 | 3,487,712 B | LF | 一致 | `assay` |
| macos-amd64 | 3,495,968 B | LF | 一致 | `assay` |
| macos-arm64 | 3,025,408 B | LF | 一致 | `assay` |
| windows-amd64 | 3,032,576 B | LF | 一致 | `assay.exe` |

`draft=false`、`prerelease=false`、`assets=10`。

## 三、包内二进制实跑

```
macos-arm64: assay 0.2.6
verify-evidence: == 500 / 500 份一致
verify-reports : == 500 / 500 份一致
```

第 ⑤ 步（本次新增，`scripts/verify-release.sh`）：

```
OK   来源识别：platform='公众号日报（qunmind 条目）'
OK   条数：catalogTotal=4 items.jsonl=4
OK   一致性三项 0 命中
```

为什么加它：前四步只摸浙江那 500 份冻结快照，**对「这一版新增了什么」是瞎的**。
`news:` 适配器哪怕在包里是坏的，10 个资产、哈希、旧门禁照样全绿。

## 四、线上站点

`pages.yml` 最后一次是 2026-11-25 的 `bf622dc`，v0.2.6 没有触发新的部署——
符合预期：这一版只改 `crates/core`、`crates/cli`、`fixtures`、`scripts`、`docs`，
没碰 `crates/web/**` 与 `web/data.json`。线上是否需要改动判定见
`docs/RELEASE_CHECKLIST.md` 第 5 节。

## 五、本次发现的两个流程坑

1. **发版 PR 上的 `ci` 不是门禁**。release-please 用 `GITHUB_TOKEN` 建 PR，
   workflow 默认要人工批准 → 状态 `action_required`；不批准就合并，run 会在分支
   被删时取消，显示成 `failure` 且没有任何 job（`gh run view` 只说「workflow file
   issue」）。#76 排队 1 小时 7 分后就是这样。别据此判断发布坏了。
2. **点火必须在 main 的 `ci` 绿了之后**。草稿一旦发布，`latest` 立刻指向它；
   若 tag 提交本身构建不过（例如发版 PR 漏改 `Cargo.lock`，`--locked` 直接失败），
   得到的是「已发布 + 0 资产」的 release，比不发更糟。本次是 06:29 绿、06:30 点火。

两条都已写进 `docs/RELEASE_CHECKLIST.md` 第 3 节。

## 六、回滚

```bash
gh release delete v0.2.6
git push --delete origin v0.2.6   # 谨慎：版本号不建议重用
git revert daaf669                # 撤回版本 bump
```

回滚后 `latest` 退回 `v0.2.5`；要再发请走 `v0.2.7`，不要重用 `v0.2.6`。

## 七、遗留

- 发版 PR 的 CI 从未真正执行过（`action_required`）。要么合并前手动批准一次，
  要么接受「main push 的 ci + release.yml 的 verify」作为等价门禁——本次选后者，
  但这是**用时间换确定性**：出问题时已经是 tag 之后。
- `--source news:` 在生产里只跑过 fixture 与 59 条真实导出，尚未接调度。
  一条命令已备好：`just news-check`（见 `docs/REUSE.md`）。

## 相关

- `docs/RELEASE_CHECKLIST.md` — 发版与线上判定链
- `docs/REUSE.md` — 接真实业务的路径与口径
- `scripts/verify-release.sh` — 五步终检
- `scripts/news-check.sh` — 日报体检（生产路径）
