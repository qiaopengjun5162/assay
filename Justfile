# 唯一入口：本地与 CI 跑同一套命令
set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# 格式化
fmt:
    cd rust-core && cargo fmt --all

# 格式检查（CI 用）
fmt-check:
    cd rust-core && cargo fmt --all -- --check

# 静态分析（零容忍）
clippy:
    cd rust-core && cargo clippy --all-targets --all-features --tests --benches -- -D warnings

# 单元 + 黄金文件测试
test:
    cd rust-core && cargo test --all-features

# 证明门禁：拒绝 sorry，内核必须 PASS
proofs file="rust-core/fixtures/expected.lean":
    ./scripts/check_proofs.sh {{file}}

# 本地完整检查（提交前跑这个）
check: fmt-check clippy test proofs
    python3 -m py_compile scripts/*.py
    echo "全部通过"

# 全量体检（真实网络，约 12 分钟）
audit threads="8":
    cd rust-core && cargo run --release --bin full_audit -- --threads {{threads}} --out ../out-full

# 聚合榜单
report:
    python3 scripts/aggregate.py --items out-full/items.jsonl --catalog out-full/catalog_snapshot.json --out out-full

# 榜单前 N 名生成 Lean 并校验
prove top="20":
    python3 scripts/proof_top.py --items out-full/items.jsonl --out out-full --top {{top}}
    ./scripts/check_proofs.sh out-full/TopProofs.lean

# 导出前端数据
web:
    python3 scripts/export_web.py --items out-full/items.jsonl --summary out-full/summary.json --proofs out-full/proofs.json --out web/data.json --limit 500

# 本地预览静态站
serve:
    python3 -m http.server -d web 8080
