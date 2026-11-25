# v0.2.5 发布后完整终检

复核时间：2026-11-26 01:44–02:05（CST）
复核对象：`main` @ `95d1b6a`，latest release `v0.2.5`（tag 指向 `09d6074`）
产站：<https://qiaopengjun5162.github.io/assay/>
性质：**只读复核**。未改源码、未改统计口径、未改任何产物。本报告即是归档记录。

## 结论

三项复核全部通过，无遗留项。

| 复核项 | 结果 |
|---|---|
| 本地工程门禁 `just check` + `just lint-docs` | 通过（EXIT=0） |
| 发布终检 `verify-release.sh v0.2.5` | 通过（EXIT=0，10 资产齐） |
| 线上站点产物哈希 + 浏览器交互 | 通过（wasm 哈希与 CI 产物一致；12/12 断言过） |

## 一、本地工程门禁

在 `main` @ `95d1b6a` 上重跑，命令与真实输出：

```bash
just check      # EXIT=0
just lint-docs  # EXIT=0
```

| 门禁 | 实测 |
|---|---|
| 证据门禁 `verify-evidence` | `== 500 / 500 份一致` |
| 报告门禁 `verify-reports` | `== 500 / 500 份一致` |
| 证明内核 `check-proofs` | `PASS 内核校验通过`（`fixtures/expected.lean`） |
| 门禁负向自测 | **11 通过 / 0 失败** |
| 文档 lint | 26 files / **0 issues** |

负向自测逐项确认「每道门禁都真的会红」：

| 断言 | 期望 | 实测 |
|---|---|---|
| evidence-baseline / reports-baseline / proofs-baseline | rc=0 | rc=0 PASS |
| evidence-byte（篡改一个字节） | rc=1 | rc=1 PASS |
| evidence-missing（删一份证据） | rc=1 | rc=1 PASS |
| report-sha（改报告里的哈希） | rc=1 | rc=1 PASS |
| proofs-sorry（证明里塞 sorry） | rc=1 | rc=1 PASS |
| tamper-demo | rc=0 | rc=0 PASS |
| drift-healthy | rc=0 | rc=0 PASS |
| drift-drifted | rc=1 | rc=1 PASS |
| drift-suspect | rc=2 | rc=2 PASS |

## 二、发布终检

`bash scripts/verify-release.sh v0.2.5`，EXIT=0。

| 项 | 结果 |
|---|---|
| 已发布状态 | draft/prerelease = false |
| 资产数 | **10 / 10** |
| 5 平台 sha256 | 全部一致，LF 行尾 |
| 包内二进制 | 每包恰一个 |

解压后二进制（字节）：

| 平台 | 二进制 | 大小 |
|---|---|---|
| linux-amd64 | `assay` | 3,795,792 |
| linux-arm64 | `assay` | 3,418,048 |
| macos-amd64 | `assay` | 3,468,264 |
| macos-arm64 | `assay` | 3,005,696 |
| windows-amd64 | `assay.exe` | 3,000,320 |

本机（macos-arm64）实跑复核：

```text
macos-arm64: assay 0.2.5
verify-evidence: == 500 / 500 份一致
verify-reports:  == 500 / 500 份一致
```

## 三、线上站点

### 3.1 产物一致性（与 CI artifact 比，不与本地 dist 比）

本地 `dist/` 不入库，CI 从源码重建，文件名 hash 与本地不同是正常的。判据是
「线上产物 == 部署它的那次 CI artifact」。

| 项 | 值 |
|---|---|
| 线上 wasm | `assay-web-c9bba65f7fe19c6a_bg.wasm` |
| 大小 | 1,784,015 字节 |
| magic | `0061736d`（合法 wasm） |
| 线上 sha256 | `2b2a801cf6720fea0d2a67820c7d6fee5f90f2a899408ad199c814e9f72e6350` |
| CI artifact sha256 | `2b2a801cf6720fea0d2a67820c7d6fee5f90f2a899408ad199c814e9f72e6350` |
| 结论 | **完全一致** |

线上 `index.html` 引用的资源逐个探测，全部 200：

| 资源 | HTTP | 大小 |
|---|---|---|
| `assay-web-febfb46d92aa0e8a.js` | 200 | 40,538 |
| `assay-web-c9bba65f7fe19c6a_bg.wasm` | 200 | 1,784,015 |
| `style-8c8ce6ef381c8843.css` | 200 | 19,998 |
| `favicon.svg` | 200 | 351 |
| `data.json` | 200 | 1,114,348 |

### 3.2 在线浏览器（Playwright，真实 Chromium）

12 项断言全部通过：

| # | 断言 | 实测 |
|---|---|---|
| 1 | 顶栏入口 | 榜单 → 部门 → 证明 → 判定规则（4 个） |
| 2 | 首屏验证 CTA | 「现场验一条」存在 |
| 3 | 榜单行数 | 500 |
| 4 | 搜索计数 | 搜「票务」→「筛出 5 条（榜上共 500 条）」+ 清除筛选 |
| 5 | 部门索引 | 2,500 张卡 |
| 6 | 证明索引 | 391 行（= 冻结产物 proof 非空条数） |
| 7 | 详情页跳转按钮 | 「↓ 跳到证据，当场重算」 |
| 8 | 浏览器重算 SHA-256 | **一致**（`0edd12c6f7caca63e42073f4a3d57d95b6e1f53c389403e1b943172e5ef041d6`） |
| 9 | 移动端 390px 首页溢出 | 0px |
| 10 | 移动端 390px 部门溢出 | 0px |
| 11 | 移动端 390px 证明溢出 | 0px |
| 12 | 移动端 390px 详情溢出 | 0px |

## 四、冻结数字口径

引 `README.md`，本次复核未改变任何数字：

| 口径 | 值 |
|---|---|
| 纳入体检的数据集 | **24,322**（全量枚举，不是抽样） |
| 榜单 / 证据 / 报告 / AI 解读 | **500** |
| 可证数据集 / Lean 定理 | **391 / 4,970** |
| 检查项 | **10** |
| 精度 | **样例级**：表头 + 5 行样例，不是全量 2.26 亿行 |

`证明 391 < 榜单 500` 不是漏做：差的 109 个是平台只给表头、没有数据行，
没有可证的命题。`proof ⊆ evidence` 关系有 `just check` 内的门禁复核。

## 五、本次复核排除的两个假警报

两个都是**验证方法出错**，不是产品缺陷；记下来避免以后重复误判。

### 5.1 线上 wasm 下载到 9,379 字节的「Page not found」

第一次下载线上 wasm 只拿到 9,379 字节，内容是 GitHub Pages 的 404 HTML。
原因是代理瞬时故障。加**重试 + 校验 wasm magic（`0061736d`）**后拿到
1,784,015 字节，sha256 与 CI artifact 一致。

判据：下载二进制后先校验 magic / 大小，再比哈希；不要凭一次短超时下载的结果下结论。

### 5.2 证明索引断言 `links=0`

首版断言用 `a[href*='#/proof/']` 匹配，得到 0。核对源码后确认：证明索引
复用 `row_view`，链接指向 `#/detail/<id>`，本来就没有 `#/proof/` 链接——
是断言写错。改为数 `.board .row` 并断言等于 391 后通过。

判据：UI 断言失败时先看渲染代码，再判断是产品问题还是断言问题。

## 六、边界

- **样例级**：结论来自表头 + 5 行样例，不回答分布、趋势、全量统计。
- **不评准确性**：assay 只回答「说的和给的是否一致」，不判断数据内容真伪。
- **证明对象是快照**：Lean 证的是已落盘并算过哈希的 `preview.json` 里的计数，
  不是现场重抓的新数据。

## 七、回滚

若需回退 `v0.2.5`：

```bash
gh release delete v0.2.5        # 删 release
git push --delete origin v0.2.5 # 删远端 tag（谨慎：版本号不建议重用）
git revert 09d6074              # 撤回版本 bump
```

`v0.2.5` 删除后不建议重新使用（避免下游缓存混版本）；要再发走 `v0.2.6`。

## 相关

- `docs/RELEASE_CHECKLIST.md` — 发版与线上判定链
- `docs/HANDOFF.md` — 交接与待办
- `scripts/verify-release.sh` — 发布终检脚本
- `scripts/verify-live.sh` — 线上产物核对脚本
