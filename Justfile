# 唯一入口：本地与 CI 跑同一套命令
set shell := ["bash", "-euo", "pipefail", "-c"]

# 产物目录。换一个就有干净的独立跑批：just out=out-smoke audit 8 200
out := "out-full"

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

# 1) 全量体检（真实网络，约 12 分钟；limit=0 表示全部）
audit threads="8" limit="0":
    cd rust-core && cargo run --release --bin full_audit -- --threads {{threads}} --limit {{limit}} --out ../{{out}}

# 2) 聚合榜单（产出 summary.json / leaderboard.md / leaderboard_dept.md）
report:
    python3 scripts/aggregate.py --items {{out}}/items.jsonl --catalog {{out}}/catalog_snapshot.json --out {{out}}

# 3) 证据包：落盘接口原始响应并算哈希（确定性，不经模型）
evidence top="100":
    python3 scripts/evidence.py --items {{out}}/items.jsonl --out {{out}} --top {{top}}

# 4) 榜单前 N 名生成 Lean 并校验（优先读存证快照）
prove top="100":
    python3 scripts/proof_top.py --items {{out}}/items.jsonl --out {{out}} --top {{top}} --evidence {{out}}/evidence.json
    ./scripts/check_proofs.sh {{out}}/TopProofs.lean

# 5) 发布到静态站（数据 + 证明 + 证据一次到位）
web:
    cp {{out}}/TopProofs.lean web/TopProofs.lean
    rm -rf web/evidence
    cp -R {{out}}/evidence web/evidence
    python3 scripts/export_web.py --items {{out}}/items.jsonl --summary {{out}}/summary.json --proofs {{out}}/proofs.json --catalog {{out}}/catalog_snapshot.json --evidence {{out}}/evidence.json --out web/data.json --limit 500

# 全流水线：体检 → 榜单 → 证据 → 证明 → 静态站
all: audit report evidence prove web
    echo "完成：just serve 预览"

# 小样本端到端自检（200 条，不覆盖正式产物）
smoke:
    just out=out-smoke audit 8 200
    just out=out-smoke report
    just out=out-smoke evidence 5
    just out=out-smoke prove 5
    echo "smoke 通过：out-smoke/ 下有 summary、证据、证明"

# 篡改演示：改一位数字，让内核当场拒绝
tamper file="out-full/TopProofs.lean":
    ./scripts/tamper_demo.sh {{file}}

# 本地预览静态站
serve:
    python3 -m http.server -d web 8080
