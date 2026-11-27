# 发版检查清单

> 目标不是「无人值守自动发版」，而是**任何一次发版都能被核对**：版本号从哪来、
> 资产是谁编的、下载下来能不能跑、失败时怎么退。配置见
> `.release-please-config.json`、`.release-please-manifest.json` 与两条 workflow。

## 0. 版本号口径

当前已发布 `v0.2.7`（`latest`，10 资产），源码与 manifest 都是 `0.2.7`；
`v0.2.1` 是首个正式 release（首次发版踩过 Windows CRLF 坑，资产已人工补齐）。
ROADMAP 早期写的 `v0.1.0` 是作废的旧计划，**不要再按它打 tag**。

release-please 从 `bootstrap-sha`（`6532b3a`，全 Rust 工作区那一版）之后扫描
Conventional Commits，自己算出下一个版本号并开 PR。发布以**那个 PR 写版本号为准**，
不手工挑一个顺眼的数字。

用的是 `release-type: simple`，版本源是单独的 `.release-please-version`（只有一行数字）。
**不能把 `version-file` 指到 `Cargo.toml`**：simple 策略用 DefaultUpdater 更新版本文件，
会把整个文件替换成一行，实测发版 PR 差点把 workspace 清单清空（PR #37 已关）。
根 `Cargo.toml` 与 ROADMAP 的版本行都带 `x-release-please-version` 标记，由 `generic`
extra-files 只改那一行；`Cargo.lock` 里 4 个 assay 包靠 `toml` extra-files 同步。
rust 策略处理不了虚拟 workspace + `version.workspace = true`，原因见 D36。

仓库设置：`Settings → Actions → General → Workflow permissions` 要勾上
**Allow GitHub Actions to create and approve pull requests**，否则 release-please
会建好分支却在开 PR 时报 “GitHub Actions is not permitted to create or approve
pull requests”。已用 API 打开（`can_approve_pull_request_reviews=true`），
默认的 workflow 权限仍保持 `read`，写权限由 workflow 自己声明。
核对命令：

```bash
gh api repos/qiaopengjun5162/assay/actions/permissions/workflow \
  --jq '{default_workflow_permissions, can_approve_pull_request_reviews}'
```

`release-please` 配置的 `"draft": true`：发版 PR 合并后先生成**草稿 release**，
草稿不计入 `latest`，`releases/latest/download/...` 在构建期间仍指向上一版可用资产。

## 1. 发版前

```bash
just check              # fmt / clippy / nextest / deny / typos / 证据+报告+证明门禁 / 门禁负向自测
just clippy-wasm
just lint-docs
./scripts/coverage.sh   # 两个地板：74（core+cli）/ 72（全量）
```

另外确认：

- 线上五视图可访问，`just a11y` 五个视图 `violations=0`；
- `verify-evidence` 与 `verify-reports` 都是 500/500；
- 没有 key、没有 `out-full/` 大文件进 git。

## 2. 发版 PR

release-please 在每次推 `main` 后自动开/更新 `chore(main): release x.y.z` PR。
合并前先看它改了什么：

```bash
gh pr diff <num> --name-only
```

期望只有 `.release-please-manifest.json`、`.release-please-version`、`CHANGELOG.md`、
`Cargo.toml`、`Cargo.lock`、`ROADMAP.md`。**`Cargo.toml` 只能改一行**：出现
`@@ -1,32 +1 @@` 这种整文件替换就是配错了 `version-file`，立刻关 PR。
出现 `src/` 也停下——这不是纯发版 PR。`Cargo.lock` 必须跟着改：漏了它，CI 的
`--locked` 立刻红。

**假发版 PR**：合并发版 PR 之后、把草稿发布出去之前，release-please 找不到锚点，
有时会开出一个内容重复的 PR。判据是**新增的 CHANGELOG 条目是否已经在历史版本里出现过**，
而不是行数——行数只是表象：

```bash
gh pr diff <num> -- CHANGELOG.md | grep '^+'   # 看新增条目
# 逐条拿 PR 号或 commit 到已有 CHANGELOG 里 grep，命中即重复
grep -n '#<号>\|<commit前7位>' CHANGELOG.md
```

2026-11-25 实测：#71 是 v0.2.5 发布后 2 分钟开的 `release 0.2.6`，
CHANGELOG 唯一新增条目 `#56` 其实已在 v0.2.4 发布过（重复计入）。它只增了 1 行，
用「个位数行数是真」的旧判据会误判为真——**本质判据是内容重复，不是行数**。
关掉即可，**不要合并**；关之前确认 `gh release view` 的 latest 正确、CHANGELOG
里该条目已有归属版本。

## 3. 点火（关键，容易漏）

默认 `GITHUB_TOKEN` 创建的 tag **不会**触发其它 workflow。所以发版 PR 合并后：
`push: tags` 不会自动跑，`release.yml` 也不会自动出资产。必须人工发布草稿：

```bash
gh release list --limit 3                  # 确认 vX.Y.Z 以 Draft 出现
gh release edit vX.Y.Z --draft=false       # 用你自己的 token 建 tag，于是点火
git ls-remote --tags origin | grep vX.Y.Z  # 确认 tag 存在且指向 main
gh workflow run release.yml --ref vX.Y.Z   # 兜底点火（幂等，见下）
```

这一步可能同时触发两个 run（`event=release` 与 `event=push`）。两者都会完整构建
5 个平台，留一个、取消另一个：

```bash
gh run list --workflow release.yml --limit 6 \
  --json databaseId,status,event,headBranch --jq '.[] | "\(.databaseId) \(.status) \(.event)"'
gh run cancel <要取消的 id>
gh run watch <保留的 id> --exit-status
```

若给仓库配了 `RELEASE_PAT` secret，release-please 用 PAT 建 tag，就会自动触发；
此时上面的手工点火可省。PAT 的代价是权限更大，别为了省两条命令盲目开。

### 点火前必须看一眼 main 的 ci

草稿一旦发布，`latest` 立刻指向它——`releases/latest/download/...` 从此给用户
推这一版。如果这个 tag 提交本身是坏的（比如发版 PR 漏改 `Cargo.lock`，
构建用 `--locked` 直接失败），状态会是「已发布的 release + 0 资产」，
比不发更糟。所以点火前先确认：

```bash
gh run list --branch main --limit 4 \
  --json workflowName,headSha,conclusion --jq '.[] | "\(.workflowName) \(.headSha[0:7]) \(.conclusion)"'
# 期望看到 headline 是发版提交、workflow=ci、conclusion=success
```

**别拿发版 PR 上的 ci 当门禁**。release-please 用 `GITHUB_TOKEN` 建 PR，
GitHub 对这类 PR 的 workflow 默认要求人工批准，状态是 `action_required`；
没批准就合并，run 会在分支被删时取消，显示成 `failure`（且没有任何 job）。
看着吓人，其实什么都没跑。2026-11-26 的 #76 就是这样：排队 1 小时 7 分后
被执行「取消」，`gh run view` 只给一句「workflow file issue」。

真正的门禁有三道，顺序是：main push 的 `ci` → 点火后 `release.yml` 的
`verify · 发版提交必须过门禁`（fmt / clippy / nextest / 两份产物门禁）→
`scripts/verify-release.sh` 拿发布出去的字节重跑一遍。

## 4. 终检（必须是 10 个资产）

5 平台 ×（二进制包 + `.sha256`）：`linux-amd64`、`linux-arm64`、`macos-amd64`、
`macos-arm64`、`windows-amd64`。

```bash
gh release view vX.Y.Z \
  --json tagName,isDraft,isPrerelease,assets \
  --jq '{tag:.tagName,draft:.isDraft,pre:.isPrerelease,n:(.assets|length)}'
```

期望 `draft=false`、`pre=false`、`n=10`。workflow 里已经用 `shasum -a 256 -c`
逐个自检过；下载后仍要在自己机器上再验一次。**别只验一个平台就算完**——旧清单
只给了 macos-arm64 一条命令，其余 9 个靠 workflow 自检那句话带过，等于没验。
一条命令跑完整终检：

```bash
bash scripts/verify-release.sh vX.Y.Z   # 或 just verify-release vX.Y.Z
# 国内网络下载失败时：export HTTPS_PROXY=http://127.0.0.1:7890
```

它做五件事：① 核对 10 个资产齐、非草稿；② 全部下载回来逐份 `shasum -c`，
并单独查 `.sha256` 行尾是不是 LF（CRLF 会让 shasum 误报）；③ 逐包解压，
确认有且只有预期的那个二进制；④ 用匹配本机架构的那个二进制，在仓库根目录
跑 `verify-evidence` 与 `verify-reports`——验的是**用户拿到的那个字节**还能不能
复核 500 份证据与报告；⑤ 从 v0.2.6 起，再用这个二进制跑一遍
`--source news:fixtures/qunmind-items.json` → `audit` → `checks`。

第 ⑤ 步是补一个漏：前四步只摸浙江那 500 份冻结快照，**对「这一版新增了什么」是瞎的**。
v0.2.6 新增的 `news:` 适配器恰恰是用户下载后最想跑的功能，旧资产全对也可能功能是坏的。
它只断言三件不怕重构的事：`run.json.platform` 认出来源（含 `qunmind`，不强绑全串）、
`catalogTotal` 与 `items.jsonl` 行数一致（派生 id 没丢条）、一致性三项 0 命中
（若哪天又变成 4/4 全中，说明尺子退回了「拿 Option 当承诺」的老路，见 #77）。

2026-11-21 对 v0.2.3 实测（发版后第一次真跑）：10 资产齐、5 份哈希全对且全 LF、
5 个包内二进制齐全、macos-arm64 实跑 `assay 0.2.3` 两条门禁均 500/500。
2026-11-26 对 v0.2.6：同上全过，另加第 ⑤ 步 4 条上游原样条目、一致性 0 命中。
2026-11-27 对 v0.2.7：同上全过（含内核 `catalog_all` 缺页修复随包发布），第 ⑤ 步三项 OK。

下载要**串行 + 重试三次**，两个极端都不能走：并发 10 个 curl 会在代理下撞
HTTP/2 framing 错误静默丢文件；强制 `--http1.1` 则同一个包 60 秒才下到一半
（默认 HTTP/2 26 秒下完）。`gh api` 取元数据也要重试——实测返回过 EOF，
没重试会让脚本在网络最抖的那一刻误报「release 有问题」。

只想快速看一个平台时，手工路径仍然是：

```bash
curl -sSLO https://github.com/qiaopengjun5162/assay/releases/download/vX.Y.Z/assay-macos-arm64.tar.gz
curl -sSLO https://github.com/qiaopengjun5162/assay/releases/download/vX.Y.Z/assay-macos-arm64.tar.gz.sha256
shasum -a 256 -c assay-macos-arm64.tar.gz.sha256
tar xzf assay-macos-arm64.tar.gz
./assay --version
```

最后两条门禁要在仓库根目录跑，用的是入库的静态产物。

## 5. 确认站点真的更新了（别用 grep 判版本）

release 发完不代表网页也换了。`pages.yml` 是另一条独立流程：
推 `main` 且改到 `crates/web/**`、`web/data.json` 等路径才触发，
在同一次 push 里先 build 再 deploy。判**线上是否更新**只有一条可靠链路：

```bash
# 1) pages 工作流成功，且部署的 commit 就是这次前端改动所在的 commit
gh run list --workflow pages.yml --limit 3 \
  --json databaseId,status,conclusion,headBranch
gh run view <run_id> --log | grep pages_build_version   # 应为目标 commit sha

# 2) 下载那次构建的 artifact，拿到它的 index.html 和 wasm
gh api repos/qiaopengjun5162/assay/actions/runs/<run_id>/artifacts \
  --jq '.artifacts[] | "\(.id) \(.name)"'
# artifact 是 tar 套 zip：gh api .../actions/artifacts/<id>/zip 解出 artifact.tar，
# 再 tar -xf 才见到 index.html 与 *.wasm

# 3) 线上 index 与 artifact index 逐字节比，再比 wasm 的 sha256
curl -sS "https://qiaopengjun5162.github.io/assay/index.html?cb=$(date +%s)" -o /tmp/live-index.html
diff /tmp/live-index.html <解压目录>/index.html        # 无差异即线上=本次构建
shasum -a 256 /tmp/live.wasm <解压目录>/*_bg.wasm        # 两个哈希必须相同
```

三个会让人误判「站点没更新」的坑，都踩过：

- **中文文案在 `.wasm` 里，不在 `.js` 胶水层**。grep 新按钮文案要查
  `*_bg.wasm`，grep `.js` 必然 0 命中——别据此判定线上还是旧版。
- **下 wasm 别用短超时**。`curl --max-time 15` 下 1.7MB wasm 会中途截断，
  看起来像「新文案命中 0」。用代理 `curl -sL --max-time 120` 下完并**比对
  sha256**，不能只看文件非空或 grep 结果。
- **缓存响应头会误导**。`age`、`x-cache` 只能辅助看缓存，不能当结论：
  曾见 `last-modified` 对得上部署时刻、`age=0`、`x-cache=MISS`，看起来
  「不是缓存问题」，其实 wasm 早就更新了，是我下载截断造成的假警报。
  只认三者齐：`last-modified` 对得上 pages 部署时刻、index 与 artifact
  一致、wasm 哈希一致。

## 6. 构建失败怎么办

先分清是哪一种，处置完全相反：

**构建失败**（`gh run view <run> --json jobs` 里有 build 是 `failure`）：资产本身是坏的。
撤掉空壳 `gh release delete vX.Y.Z --yes --cleanup-tag`，修好根因后用**新版本号**重发，
不要给同一个 tag 重传资产——那会让 tag 与代码版本对不上。

**资产自检失败**（build 全绿、末尾 release job 红在「资产自检」这步，日志是
`shasum: ... : FAILED open or read`）：资产本身是好的，只是某个 `.sha256` 行尾不对。
首次发版踩过：Windows 的 `.sha256` 被 PowerShell `Out-File` 写成 CRLF，`shasum -c`
把行尾的 `\r` 当文件名。临时处置：从 artifact 下载 10 个文件，`perl -pi -e 's/\r\n/\n/g'`
修正后本地 5 个 sha256 全过，再手动 `gh release upload`；workflow 已改用
`WriteAllText` + LF 根治（见 D36 第 7 条）。

**上传失败**（build 全绿、只有末尾 release job 红，日志是
`Resource not accessible by integration`）：资产是好的，只是 workflow 的 token
无权更新 bot 建的那个 release。从这次 run 的 artifact 手动补：

```bash
gh api repos/qiaopengjun5162/assay/actions/runs/<run_id>/artifacts \
  --jq '.artifacts[] | "\(.id) \(.name)"'
```

下载用 `gh run download <run_id> -D <dir>` 一次拿全 5 个 artifact，再把文件平铺。
上传**逐个单独一条命令**：`gh release upload` 传大包走代理会被 SIGTERM（实测），
更稳的是 `curl --proxy http://127.0.0.1:7890 --data-binary @<file>` 直接 POST 到
release 的 `upload_url?name=<file>`（带 `Authorization: Bearer <token>`）；
单个包超时返回 408 就重试那一个，其余不受影响。
用 `shasum -a 256 -c` 自检后再上传（若修过 CRLF，用修过的那份）。
若仍报权限错，说明该 release 归 bot 所有——配 `RELEASE_PAT` 后重跑末段，
或由仓库管理员手工上传。

## 7. 不做的

- 不在 PR CI 里访问政府网站（D6）；发布构建只用冻结产物。
- 不把 key、token 或 `out-full/` 打进资产包。
- 不为了让 `latest` 好看而先发布后构建：空壳 release 会让所有下载链接 404。
- 不在脏工作区上跑 `release-please release-pr --local --local-path .`：它会
  `git checkout main && git reset --hard origin/main`，把未提交改动清掉。
  要验证配置，先提交到分支再用 `--target-branch <分支>` 干跑。
