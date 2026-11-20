# 新队友第一天怎么加入

> 12-04 早上你才第一次看到这份代码。这份文件的目标是让你 **10 分钟内跑起来、当天能领一个不阻塞别人的活**。

## 这个项目在干什么

给政府开放数据做体检，并且让每一条数值结论都能被机器独立复核。

不是「数据质量打分网站」。核心差别在最后两个字。所有数据质量工具都能说「这列 40% 空」，
但它给的是**报告**；报告会被改、被转发十次之后失真。
我们把「第 3 列恰好 2 条空值」写成一条 Lean 4 定理，交给证明器内核穷举验证；
把接口原始响应逐字节存盘并算 SHA-256。改任何一位数字，内核立刻报错。

**证明证的是推导，哈希钉的是时点。** 这两条都没让模型参与。

已经做到的（冻结在 2026-11-14 的快照上）：
24,322 个数据集全量体检、391 个可证数据集 4,970 条定理内核 PASS、500 份证据包、500 份报告、静态站五视图上线。

## 先跑起来

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk cargo-nextest just --locked
just hooks          # 装 commit-msg 钩子
just web            # 构建前端（约 1 分钟）
just serve          # http://localhost:8080
```

打开浏览器应该看到榜单。点任意一条进详情，能看到六维、字段差异、AI 解读、证据下载、证明入口、报告下载。

> 代理：本机访问 crates.io 要 `export HTTPS_PROXY=http://127.0.0.1:7890`。
> `dist/` 不入库，所以**必须自己跑一次 `just web`**，否则 `just serve` 是空目录。

## 提交前必跑

```bash
just check        # fmt(Rust+TOML) + clippy + nextest + cargo-deny + 证明门禁
just clippy-wasm  # 前端也在门禁内
```

红了不要提交。不要为了让测试过而改断言。

## 代码地图（只列你会碰的）

| 你要改… | 去这里 |
|---|---|
| 前端页面 | `crates/web/src/app.rs` |
| 前端数据模型 | `crates/web/src/model.rs` |
| 单数据集报告 | `crates/core/src/single.rs` |
| 前端 JSON 导出 | `crates/core/src/export.rs` |
| 空值统计 | `crates/core/src/matrix.rs` |
| 命令入口 | `crates/cli/src/main.rs` |

## 绝不能破坏的四条

1. **口径只有一处**：`scoring::classify` / `scoring::score`，别处不许复制比较逻辑
2. **证明对象 = 存证对象**：`proof` 读 `evidence/<id>/preview.json`，不现场重抓
3. **哈希算原始字节**：走 `api::post_raw`，不许序列化后重算
4. **前端只读**：WASM 只 GET 静态文件

## 当天可以领的活（按上手难度）

**第一天就能做，不碰口径：**

- 前端：详情页「证明」和「报告」两块目前是并排两个按钮，改成一张「可复核材料」卡
- 前端：榜单加「按部门筛」的空态文案
- 文档：`docs/ARCHITECTURE.md` 第 5 节的不变量补一条你实际踩到的
- 测试：给 `crates/core/src/export.rs` 补集成测试（零覆盖，用 `tests/pipeline.rs` 的假平台）
- 测试：给 `crates/core/src/evidence.rs` 补集成测试（同上）

**熟悉之后：**

- 检查项 `checks.yaml` 声明化（见 `ROADMAP.md` 第 3 条）
- 第二平台接入（需先探明接口，`docs/HANDOFF.md` 待办 1 有三步法）

**不确定做什么就问，别自己发明口径。**

## 演示怎么演

`docs/PITCH.md` 是 6 分钟讲稿，`just tamper` 是全场最硬的一分钟——
改一位数字，Lean 内核当场报错、打印 `lean 退出码 1`；`just` 本身退出码 0（演示成功）。演之前预跑一次确认 Lean 在 PATH。

## 先读什么

按顺序，一共约 15 分钟：

1. `README.md` 的「边界」一节——先知道我们不声称什么
2. `CLAUDE.md` 的「硬约束」——11 条
3. `docs/ENGINEERING_LESSONS_ZH.md`——踩过的坑
4. `docs/DECISIONS.md` 的 D5（字段三类）和 D10（为什么不用模型抓页面）
