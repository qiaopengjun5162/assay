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

# 证据包：落盘接口原始响应并算哈希（确定性，不经模型）
evidence top="100":
    python3 scripts/evidence.py --items out-full/items.jsonl --out out-full --top {{top}}
    cp -R out-full/evidence web/evidence

# 榜单前 N 名生成 Lean 并校验（优先读存证快照）
prove top="100":
    python3 scripts/proof_top.py --items out-full/items.jsonl --out out-full --top {{top}} --evidence out-full/evidence.json
    ./scripts/check_proofs.sh out-full/TopProofs.lean
    cp out-full/TopProofs.lean web/TopProofs.lean

# 导出前端数据
web:
    python3 scripts/export_web.py --items out-full/items.jsonl --summary out-full/summary.json --proofs out-full/proofs.json --catalog out-full/catalog_snapshot.json --evidence out-full/evidence.json --out web/data.json --limit 500

# 篡改演示：改一位数字，让内核当场拒绝
tamper file="out-full/TopProofs.lean":
    ./scripts/tamper_demo.sh {{file}}

# 本地预览静态站
serve:
    python3 -m http.server -d web 8080
