# 唯一入口：本地与 CI 跑同一套命令
set shell := ["bash", "-euo", "pipefail", "-c"]

# 产物目录。换一个就有干净的独立跑批：just out=out-smoke audit 8 200
out := "out-full"

default:
    @just --list

# 编译（core + cli）
build:
    cargo build --release -p assay-core -p assay

# 格式化（Rust + TOML）
fmt:
    cargo fmt --all
    taplo fmt

# 格式检查
fmt-check:
    cargo fmt --all -- --check
    taplo fmt --check

# 拼写检查（typos-cli）
typos:
    @command -v typos >/dev/null || { echo "未安装 typos：brew install typos-cli（或 cargo install typos-cli）"; exit 1; }
    typos

# 第二平台探针：深圳的目录能不能枚举、接口要什么授权（只读，不需账号）
probe-sz key="":
    ./scripts/sz-probe.sh {{key}}

# 依赖审计（许可证 + 漏洞 + 供应链）
deny:
    cargo deny check

# 静态分析（core/cli/web，零容忍）
clippy:
    cargo clippy --all-targets --all-features --tests --benches -- -D warnings

# 生成 CHANGELOG（git-cliff 从 Conventional Commits 派生）
changelog:
    git cliff -o CHANGELOG.md

# 文档 lint
lint-docs:
    ./scripts/lint-docs.sh

# 文档 lint（只查改动）
lint-docs-changed:
    ./scripts/lint-docs.sh --changed

# 安装仓库 git 钩子（Conventional Commits）
hooks:
    ./scripts/install-git-hooks.sh

# 前端静态分析（wasm 目标）
clippy-wasm:
    cd crates/web && cargo clippy --target wasm32-unknown-unknown -- -D warnings

# 单元 + 集成测试（含 web 的复核命令纯函数；它 native 可跑，不必等 wasm）
# 必须与 CI 的 `rust` job 同集合：漏掉 `-p assay-api` 时本地 123 绿、CI 132 绿，
# 「本地过了」就成了假信号（api 的 9 条曾经只在 CI 跑）。
test:
    cargo nextest run -p assay-core -p assay -p assay-api -p assay-web --all-features

# 测试 + 覆盖率（带地板；CI 的 coverage job 跑同一条命令）
coverage:
    ./scripts/coverage.sh

# 覆盖率 + HTML 报告（本地看哪个分支没测到）
coverage-html:
    ./scripts/coverage.sh --html

# 证明门禁：拒绝 sorry，内核必须 PASS
proofs file="fixtures/expected.lean":
    cargo run --release -q -p assay -- check-proofs {{file}}

# 证据门禁：逐份重算 SHA-256，改一个字节就红（跑入库的站点证据，CI 可用）
verify-evidence:
    cargo run --release -q -p assay -- verify-evidence web/data.json web/evidence

# 报告门禁：报告里印的 SHA-256 必须等于证据文件重算结果（同一次抓取）
# 用错 --out 补报告会生成一批哈希指向别处的报告，每份看着完整、读者逐份核对全失败
verify-reports:
    cargo run --release -q -p assay -- verify-reports web/data.json web/reports web/evidence

# 门禁负向自测：故意破坏产物，确认每道门禁真的会红（全在临时副本上做）
gate-negative:
    ./scripts/gate-negative.sh

# 发布终检：把 tag 的全部资产下载回来验签、解压、跑本机二进制（省略 tag 取 latest）
verify-release tag="":
    ./scripts/verify-release.sh {{tag}}

# 线上终检：验 github.io 活站是不是仓库最新版、抽一条证据重算哈希、资源是否可达
# 不进 check：依赖外网访问 github.io，交给交付前手动跑（可传自定义域名验镜像）
verify-live base="":
    ./scripts/verify-live.sh {{base}}

# 线上终检的负向自测：本地搭假站点，证明每处破坏都会让 verify-live 变红
# 纯本地、不访问外网；跟 gate-negative 同理——验工具本身有没有约束力
verify-live-negative:
    bash scripts/verify-live-negative.sh

# 复用演示：用公众号日报形状的 fixture 走同一套内核，证明换数据源零改动
# 不进 check：它是能力演示，不是门禁；产物写临时目录，不碰冻结快照。
reuse-demo:
    bash scripts/reuse-demo.sh

# 复用演示（新闻源）：直接吃 qunmind 的 PublicNewsItem 形状（snake_case / 无 id），
# id 由 assay 派生，上游不用改结构。不进 check：能力演示，产物写临时目录。
reuse-news:
    bash scripts/reuse-news.sh

# 公众号日报体检（生产路径）：读真实导出、出人话摘要，查不出问题也算成功。
# items 省略时读 out-news/items.json；`just news-check --export` 会先调 qunmind 导出。
news-check *ARGS:
    bash scripts/news-check.sh {{ARGS}}

# 本地完整检查（提交前跑这个）
check: fmt-check clippy typos test deny verify-evidence verify-reports proofs gate-negative
    echo "全部通过"

# 无障碍复查（axe-core + WCAG 2.1 A/AA，五个视图）
# 不进 check：CI 的 rust job 没有浏览器，装 Chromium 会拖慢整条流水线。
# 交付前手动跑；改前端结构（地标 / 标题层级 / 表单控件）之后必跑。
a11y:
    bash scripts/a11y.sh

# 1) 全量体检（真实网络，约 12 分钟；limit=0 表示全部）
audit threads="8" limit="0":
    cargo run --release -p assay -- --out {{out}} audit --threads {{threads}} --limit {{limit}}

# 2) 聚合榜单
report:
    cargo run --release -p assay -- --out {{out}} report

# 2b) 跑批历史：把这一轮的指标追加进 history/summary.json（同日重跑会替换，不重复画点）
#     日期取 {{out}}/run.json；没有 run.json（如补跑旧快照）就显式传：
#       just history 2026-11-14
history date="":
    cargo run --release -p assay -- --out {{out}} history {{ if date != "" { "--date " + date } else { "" } }}

# 检查项清单：阈值 / 权重 / 本批命中数（尺子本身也公开）
checks:
    cargo run --release -p assay -- --out {{out}} checks

# 周度漂移对比：拿基线比本轮 summary.json（0 正常 / 1 漂移 / 2 跑批不可信）
drift:
    cargo run --release -p assay -- drift --current {{out}}/summary.json

# 审计包：打成可交付单文件。包内自带 VERIFY.md 与 shasum 清单，
# 对方解压后一条 `shasum -a 256 -c MANIFEST.sha256` 就能验，不需要装我们任何东西。
# 用法：just pack 2026-11-18  （可选第二个参数传 commit）
pack date commit="":
    cargo run --release -p assay -- --out {{out}} pack --captured-at {{date}} {{ if commit != "" { "--commit " + commit } else { "" } }}

# 周度巡检：全量体检 + 漂移判定（只预览，不写基线；写基线由 CI 的 PR 完成）
audit-weekly: (audit "8" "0")
    cargo run --release -p assay -- drift --current {{out}}/summary.json || true

# 3) 解释层：模型写人话，数字必须过护栏
explain top="500" threads="4":
    cargo run --release -p assay -- --out {{out}} explain --top {{top}} --threads {{threads}}

# 3b) 只验配置：端点 / 模型 / key / 代理，一条命令
probe:
    cargo run --release -p assay -- --out {{out}} explain --probe

# 4) 证据包：落盘接口原始响应并算哈希
evidence top="100" threads="4":
    cargo run --release -p assay -- --out {{out}} evidence --top {{top}} --threads {{threads}}

# 5) 生成 Lean 定理并跑内核
prove top="100":
    cargo run --release -p assay -- --out {{out}} prove --top {{top}}

# 6) 单数据集报告（前 N 名 → Markdown，随站点发布）
reports top="100" dest="web/reports":
    # 证据必须取入库的 web/evidence：out-full/evidence 只有 11-14 的 100 份，
    # 用它重生成会让 500 份报告全丢哈希段。verify-reports 会在门禁里拦住。
    cargo run --release -p assay -- --out {{out}} reports --top {{top}} --dest {{dest}} --evidence-dir web/evidence --no-explain

# 7) 导出静态站数据（history 会被嵌进 data.json，前端据此画曲线）
export limit="500" dest="web/data.json" history="history/summary.json":
    cargo run --release -p assay -- --out {{out}} export --limit {{limit}} --dest {{dest}} --history {{history}}

# 7b) 数据分层：data.json 之外再产 index.json（全量）+ shards/（详情分片）
#     实测全量：index 7.3MB / shards 12MB（24 片，每片约 500KB）
#     —— 解决「初次加载全量详情」，但解决不了「历史累积进 git」，那是后端的事
export-split limit="500" dest="web/data.json" shards="24":
    cargo run --release -p assay -- --out {{out}} export --limit {{limit}} --dest {{dest}} --split --shards {{shards}}

# 8) 构建 WASM 前端（trunk → crates/web/dist）
web:
    cd crates/web && trunk build --release

# 9) 查询后端（只读）。先起库：docker compose up -d db
#    证据仍是静态对象，这个服务只存索引，可随时清库重建。
api:
    cargo run -p assay-api

# 10) 把一次跑批灌进查询库（幂等，同一天重跑会覆盖）
#     用法：just api-ingest 2026-11-18          默认读 out-full
#           just api-ingest 2026-11-18 out-full
api-ingest date out="out-full":
    cargo run -p assay-api -- ingest --out {{out}} --captured-at {{date}}

# 查询后端真库端到端：自带临时 Postgres（独立端口，不动 5432），
# 灌库 → 起服务 → 验 12 项端点 → 自动清理。需要本机有 postgres 工具与 release 构建。
api-e2e:
    bash scripts/api-e2e.sh

# 全流水线：体检 → 榜单 → 历史 → 解释 → 证据 → 证明 → 报告 → 导出 → 前端
all: audit report history explain evidence prove reports export web
    echo "完成：just serve 预览"

# 小样本端到端自检（不覆盖正式产物）
smoke:
    just out=out-smoke audit 8 200
    just out=out-smoke report
    just out=out-smoke explain 3 2
    just out=out-smoke evidence 5 2
    just out=out-smoke prove 5
    just out=out-smoke reports 5 out-smoke/reports
    just out=out-smoke export 50 out-smoke/data.json
    echo "smoke 通过：out-smoke/ 下有榜单、解读、证据、证明、报告、前端数据"

# 篡改演示：改一位数字，让内核当场拒绝
tamper:
    cargo run --release -p assay -- --out {{out}} tamper

# 本地预览（Rust 静态服务）
serve port="8080":
    cargo run --release -p assay -- serve --dir crates/web/dist --port {{port}}
