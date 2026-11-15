# 唯一入口：本地与 CI 跑同一套命令
set shell := ["bash", "-euo", "pipefail", "-c"]

# 产物目录。换一个就有干净的独立跑批：just out=out-smoke audit 8 200
out := "out-full"

default:
    @just --list

# 编译（core + cli）
build:
    cargo build --release -p dataproof-core -p dataproof

# 格式化
fmt:
    cargo fmt --all

# 格式检查
fmt-check:
    cargo fmt --all -- --check

# 静态分析（core/cli，零容忍）
clippy:
    cargo clippy -p dataproof-core -p dataproof --all-targets -- -D warnings

# 前端静态分析（wasm 目标）
clippy-wasm:
    cd crates/web && cargo clippy --target wasm32-unknown-unknown -- -D warnings

# 单元 + 集成测试
test:
    cargo test -p dataproof-core -p dataproof

# 证明门禁：拒绝 sorry，内核必须 PASS
proofs file="fixtures/expected.lean":
    ./ci/check_proofs.sh {{file}}

# 本地完整检查（提交前跑这个）
check: fmt-check clippy test proofs
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

# 6) 导出静态站数据
export limit="500":
    cargo run --release -p dataproof -- --out {{out}} export --limit {{limit}}

# 7) 构建 WASM 前端（trunk → crates/web/dist）
web:
    cd crates/web && trunk build --release

# 全流水线：体检 → 榜单 → 解释 → 证据 → 证明 → 导出 → 前端
all: audit report explain evidence prove export web
    echo "完成：just serve 预览"

# 小样本端到端自检（不覆盖正式产物）
smoke:
    just out=out-smoke audit 8 200
    just out=out-smoke report
    just out=out-smoke explain 3 2
    just out=out-smoke evidence 5 2
    just out=out-smoke prove 5
    just out=out-smoke export 50 --dest out-smoke/data.json
    echo "smoke 通过：out-smoke/ 下有榜单、解读、证据、证明、前端数据"

# 篡改演示：改一位数字，让内核当场拒绝
tamper:
    cargo run --release -p dataproof -- --out {{out}} tamper

# 本地预览（Rust 静态服务）
serve port="8080":
    cargo run --release -p dataproof -- serve --dir crates/web/dist --port {{port}}
