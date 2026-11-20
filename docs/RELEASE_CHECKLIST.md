# 发版检查清单

> 目标不是「无人值守自动发版」，而是**任何一次发版都能被核对**：版本号从哪来、
> 资产是谁编的、下载下来能不能跑、失败时怎么退。配置见
> `.release-please-config.json`、`.release-please-manifest.json` 与两条 workflow。

## 0. 版本号口径

仓库当前源码版本是 `0.2.0`（`Cargo.toml` 的 `[workspace.package]`），
manifest 里写的也是 `0.2.0`。ROADMAP 早期写的 `v0.1.0` 是旧计划，**不要再按它打 tag**。

release-please 从 `bootstrap-sha`（`6532b3a`，全 Rust 工作区那一版）之后扫描
Conventional Commits，自己算出下一个版本号并开 PR。发布以**那个 PR 写版本号为准**，
不手工挑一个顺眼的数字。

`release-please` 配置的 `"draft": true`：发版 PR 合并后先生成**草稿 release**，
草稿不计入 `latest`，`releases/latest/download/...` 在构建期间仍指向上一版可用资产。

## 1. 发版前

```bash
just check              # fmt / clippy / nextest / deny / typos / 证明门禁
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

期望只有 `.release-please-manifest.json`、`CHANGELOG.md`、`Cargo.toml`、
`ROADMAP.md`（带 `x-release-please-version` 标记那一行）。出现 `src/` 就停下——
这不是纯发版 PR。

**假发版 PR**：合并发版 PR 之后、把草稿发布出去之前，release-please 找不到锚点，
有时会开出一个 CHANGELOG 上百行的 PR。判据是增量行数：个位数是真，几十上百行是假，
关掉即可，**不要合并**。

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

## 4. 终检（必须是 10 个资产）

5 平台 ×（二进制包 + `.sha256`）：`linux-amd64`、`linux-arm64`、`macos-amd64`、
`macos-arm64`、`windows-amd64`。

```bash
gh release view vX.Y.Z \
  --json tagName,isDraft,isPrerelease,assets \
  --jq '{tag:.tagName,draft:.isDraft,pre:.isPrerelease,n:(.assets|length)}'
```

期望 `draft=false`、`pre=false`、`n=10`。workflow 里已经用 `shasum -a 256 -c`
逐个自检过；下载后仍要在自己机器上再验一次：

```bash
curl -sSLO https://github.com/qiaopengjun5162/assay/releases/download/vX.Y.Z/assay-macos-arm64.tar.gz
curl -sSLO https://github.com/qiaopengjun5162/assay/releases/download/vX.Y.Z/assay-macos-arm64.tar.gz.sha256
shasum -a 256 -c assay-macos-arm64.tar.gz.sha256
tar xzf assay-macos-arm64.tar.gz
./assay --version
./assay verify-evidence web/data.json web/evidence
./assay verify-reports web/data.json web/reports web/evidence
```

最后两条要在仓库根目录跑，用的是入库的静态产物——验的是**用户拿到的那个二进制**
还能不能复核 500 份证据与报告。

## 5. 构建失败怎么办

先分清是哪一种，处置完全相反：

**构建失败**（`gh run view <run> --json jobs` 里有 build 是 `failure`）：资产本身是坏的。
撤掉空壳 `gh release delete vX.Y.Z --yes --cleanup-tag`，修好根因后用**新版本号**重发，
不要给同一个 tag 重传资产——那会让 tag 与代码版本对不上。

**上传失败**（build 全绿、只有末尾 release job 红，日志是
`Resource not accessible by integration`）：资产是好的，只是 workflow 的 token
无权更新 bot 建的那个 release。从这次 run 的 artifact 手动补：

```bash
gh api repos/qiaopengjun5162/assay/actions/runs/<run_id>/artifacts \
  --jq '.artifacts[] | "\(.id) \(.name)"'
```

逐个下载（带认证头；每个单独一条命令，连着下容易被 SIGTERM），解压得到 10 个文件，
用 `shasum -a 256 -c` 自检后再 `gh release upload vX.Y.Z <files...>`。
若仍报权限错，说明该 release 归 bot 所有——配 `RELEASE_PAT` 后重跑末段，
或由仓库管理员手工上传。

## 6. 不做的

- 不在 PR CI 里访问政府网站（D6）；发布构建只用冻结产物。
- 不把 key、token 或 `out-full/` 打进资产包。
- 不为了让 `latest` 好看而先发布后构建：空壳 release 会让所有下载链接 404。
