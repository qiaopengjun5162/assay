# 唯一入口：本地与 CI 跑同一套命令
set shell := ["bash", "-euo", "pipefail", "-c"]

# 产物目录。换一个就有干净的独立跑批：just out=out-smoke audit 8 200
out := "out-full"

default:
    @just --list

# 编译（core + cli）
build:
    cargo build --release -p dataproof-core -p dataproof

# 格式化（Rust + TOML）
fmt:
    cargo fmt --all
    taplo fmt

# 格式检查
fmt-check:
    cargo fmt --all -- --check
    taplo fmt --check

# 拼写检查
typos:
    typos

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

# 单元 + 集成测试
test:
    cargo nextest run -p dataproof-core -p dataproof --all-features

# 测试 + 覆盖率
coverage:
    cargo llvm-cov nextest -p dataproof-core -p dataproof --all-features --html

# 证明门禁：拒绝 sorry，内核必须 PASS
proofs file="fixtures/expected.lean":
    cargo run --release -q -p dataproof -- check-proofs {{file}}

# 证据门禁：逐份重算 SHA-256，改一个字节就红（跑入库的站点证据，CI 可用）
verify-evidence:
    cargo run --release -q -p dataproof -- verify-evidence web/data.json web/evidence

# 本地完整检查（提交前跑这个）
check: fmt-check clippy test deny verify-evidence proofs
    echo "全部通过"

# 1) 全量体检（真实网络，约 12 分钟；limit=0 表示全部）
audit threads="8" limit="0":
    cargo run --release -p dataproof -- --out {{out}} audit --threads {{threads}} --limit {{limit}}

# 2) 聚合榜单
report:
    cargo run --release -p dataproof -- --out {{out}} report

# 3) 解释层：模型写人话，数字必须过护栏
explain top="100" threads="4":
    cargo run --release -p dataproof -- --out {{out}} explain --top {{top}} --threads {{threads}}

# 4) 证据包：落盘接口原始响应并算哈希
evidence top="100" threads="4":
    cargo run --release -p dataproof -- --out {{out}} evidence --top {{top}} --threads {{threads}}

# 5) 生成 Lean 定理并跑内核
prove top="100":
    cargo run --release -p dataproof -- --out {{out}} prove --top {{top}}

# 6) 单数据集报告（前 N 名 → Markdown，随站点发布）
reports top="100" dest="web/reports":
    cargo run --release -p dataproof -- --out {{out}} reports --top {{top}} --dest {{dest}}

# 7) 导出静态站数据（报告目录由 dest 同级决定）
export limit="500" dest="web/data.json":
    cargo run --release -p dataproof -- --out {{out}} export --limit {{limit}} --dest {{dest}}

# 8) 构建 WASM 前端（trunk → crates/web/dist）
web:
    cd crates/web && trunk build --release

# 全流水线：体检 → 榜单 → 解释 → 证据 → 证明 → 报告 → 导出 → 前端
all: audit report explain evidence prove reports export web
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
    cargo run --release -p dataproof -- --out {{out}} tamper

# 本地预览（Rust 静态服务）
serve port="8080":
    cargo run --release -p dataproof -- serve --dir crates/web/dist --port {{port}}
