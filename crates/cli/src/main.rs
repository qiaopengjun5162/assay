//! assay CLI：体检、聚合、解释、证据、证明、导出、本地服务。
//!
//! 所有子命令共享 `--out` 目录约定（默认 `out-full`），产物与旧版脚本一一对应。

use clap::{Parser, Subcommand};
use assay_core::api::Client;
use assay_core::{aggregate, audit, evidence, explain, export, proof};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(
    name = "assay",
    version,
    about = "给公共开放数据做体检，并让结论可被机器复核"
)]
struct Cli {
    /// 产物目录
    #[arg(long, global = true, default_value = "out-full")]
    out: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 全量体检：目录 → 详情 → 样例，逐条落 items.jsonl
    Audit {
        #[arg(long, default_value_t = 8)]
        threads: usize,
        #[arg(long, default_value_t = 0)]
        limit: usize,
    },
    /// 聚合榜单，产出 summary.json / leaderboard.md
    Report,
    /// 解释层：模型写人话，数字必须过护栏
    Explain {
        #[arg(long, default_value_t = 100)]
        top: usize,
        #[arg(long, default_value_t = 4)]
        threads: usize,
    },
    /// 证据包：落盘接口原始响应并算 SHA-256
    Evidence {
        #[arg(long, default_value_t = 100)]
        top: usize,
        #[arg(long, default_value_t = 4)]
        threads: usize,
    },
    /// 生成 Lean 定理并跑内核校验
    Prove {
        #[arg(long, default_value_t = 100)]
        top: usize,
    },
    /// 导出静态站数据
    Export {
        #[arg(long, default_value_t = 500)]
        limit: usize,
        /// 输出文件（默认 web/data.json）
        #[arg(long, default_value = "web/data.json")]
        dest: String,
    },
    /// 批量生成单数据集报告（前 N 名，Markdown，写进站点目录）
    Reports {
        #[arg(long, default_value_t = 100)]
        top: usize,
        /// 输出目录（默认 web/reports）；export 从这里判断哪些条有报告
        #[arg(long, default_value = "web/reports")]
        dest: String,
    },
    /// 单数据集报告：渲染成 Markdown（可直接贴进评审流程）
    ReportOne {
        /// 数据集 ID
        id: String,
        /// 输出文件；省略则打印到标准输出
        #[arg(long)]
        to: Option<String>,
    },

    /// 证明门禁：拒绝 sorry/admit，只接受 Lean 内核 PASS
    CheckProofs {
        #[arg(required = true)]
        files: Vec<String>,
    },

    /// 证据复核：逐份重算 SHA-256，和索引里的值比对
    VerifyEvidence {
        /// 索引文件：data.json（datasets[].evidence）或 evidence.json（items[]）
        index: String,
        /// 证据根目录
        evidence_dir: String,
    },
    /// 篡改演示：把一条定理的计数 +1，确认内核会拒绝
    Tamper {
        /// 证明文件（默认读 --out 下的 TopProofs.lean）
        #[arg(long)]
        file: Option<String>,
    },
    /// 本地静态服务（替代 python -m http.server）
    Serve {
        #[arg(long, default_value = "web")]
        dir: String,
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("错误: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> assay_core::Result<()> {
    let out = cli.out;
    match cli.command {
        Command::Audit { threads, limit } => {
            let client = Client::new();
            let o = audit::audit(
                &audit::AuditOptions {
                    threads,
                    limit,
                    out_dir: out,
                },
                &client,
            )?;
            println!(
                "audited={} failed={} elapsed={:.1}s",
                o.audited, o.failed, o.elapsed_secs
            );
        }
        Command::Report => {
            let summary = aggregate::aggregate(
                &Path::new(&out).join("items.jsonl").to_string_lossy(),
                &Path::new(&out)
                    .join("catalog_snapshot.json")
                    .to_string_lossy(),
                &out,
            )?;
            for k in ["audited", "withPreview", "schemaMismatch", "lagOver90Days"] {
                println!("{k}={}", summary[k]);
            }
        }
        Command::Explain { top, threads } => {
            let o = explain::explain(&explain::ExplainOptions {
                items_path: &Path::new(&out).join("items.jsonl").to_string_lossy(),
                catalog_path: &Path::new(&out)
                    .join("catalog_snapshot.json")
                    .to_string_lossy(),
                out_dir: &out,
                top,
                threads,
            })?;
            println!(
                "解释层: {}/{} 条通过护栏，{:.1}s",
                o.passed, o.total, o.elapsed_secs
            );
        }
        Command::Evidence { top, threads } => {
            let client = Client::new();
            let items = audit::read_items(&Path::new(&out).join("items.jsonl").to_string_lossy())?;
            let o = evidence::capture_top(&items, top, &out, &client, threads)?;
            println!("证据包: {}/{} 条", o.captured, o.total);
        }
        Command::Prove { top } => {
            proof::require_lean()?;
            let client = Client::new();
            let items = audit::read_items(&Path::new(&out).join("items.jsonl").to_string_lossy())?;
            let o = proof::prove_top(&items, top, &out, &client)?;
            println!(
                "定理 {} 条 / {} 个数据集，lean 校验: {}",
                o.theorems,
                o.entries,
                if o.passed { "PASS" } else { "FAIL" }
            );
            if !o.passed {
                eprintln!("{}", o.lean_stdout);
                std::process::exit(1);
            }
        }
        Command::Export { limit, dest } => {
            let reports_dir = Path::new(&dest).parent().map(|p| p.join("reports"));
            let reports_dir = reports_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());
            let o = export::export(&export::ExportOptions {
                items_path: &Path::new(&out).join("items.jsonl").to_string_lossy(),
                summary_path: &Path::new(&out).join("summary.json").to_string_lossy(),
                proofs_path: Some(&Path::new(&out).join("proofs.json").to_string_lossy()),
                catalog_path: Some(
                    &Path::new(&out)
                        .join("catalog_snapshot.json")
                        .to_string_lossy(),
                ),
                evidence_path: Some(&Path::new(&out).join("evidence.json").to_string_lossy()),
                explanations_path: Some(
                    &Path::new(&out).join("explanations.json").to_string_lossy(),
                ),
                reports_dir: reports_dir.as_deref(),
                out_path: &dest,
                limit,
            })?;
            println!(
                "导出 {} 条（共 {}）：证明 {} / 证据 {} / 解读 {}",
                o.rows, o.total, o.proofs, o.evidence, o.explanations
            );
        }
        Command::Reports { top, dest } => {
            let items = assay_core::audit::read_items(&format!("{out}/items.jsonl"))?;
            let explanations = assay_core::single::load_explanations(&out);
            let o = assay_core::single::render_top(&items, top, &out, &dest, &explanations)?;
            println!("报告: {} 条 → {dest}", o.written);
        }
        Command::ReportOne { id, to } => {
            let items = assay_core::audit::read_items(&format!("{out}/items.jsonl"))?;
            let d = items
                .iter()
                .find(|d| d.id == id)
                .ok_or_else(|| assay_core::Error::Other(format!("未找到数据集 {id}")))?;
            let (ev, rows) = assay_core::single::load_evidence(&out, &id);
            let explain = assay_core::single::load_explanations(&out);
            let md = assay_core::single::render(
                d,
                d.lag_days,
                ev.as_ref(),
                rows.as_deref(),
                explain.get(&id).map(String::as_str),
            );
            match to {
                Some(path) => {
                    std::fs::write(&path, &md)?;
                    println!("已写出 {path}（{} 字节）", md.len());
                }
                None => print!("{md}"),
            }
        }
        Command::CheckProofs { files } => {
            std::process::exit(check_proofs(&files));
        }
        Command::VerifyEvidence {
            index,
            evidence_dir,
        } => {
            std::process::exit(verify_evidence(&index, &evidence_dir));
        }
        Command::Tamper { file } => {
            let path = file.unwrap_or_else(|| format!("{out}/TopProofs.lean"));
            tamper(&path)?;
        }
        Command::Serve { dir, port } => serve(&dir, port)?,
    }
    Ok(())
}

/// 证据复核：逻辑在 `evidence::verify_index`，这里只打结果与给退出码。
/// 索引里一条都没识别出来也算失败——空跑不能当通过。
fn verify_evidence(index: &str, evidence_dir: &str) -> i32 {
    let report = match assay_core::evidence::verify_index(
        std::path::Path::new(index),
        std::path::Path::new(evidence_dir),
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("FAIL 复核不了 {index}：{e}");
            return 1;
        }
    };
    for c in &report.checks {
        if c.mismatched {
            println!("   FAIL {} 缺文件或哈希不符", c.name);
        } else {
            println!("   OK   {}", c.name);
        }
    }
    println!("== {} / {} 份一致", report.matched, report.listed);
    if report.passed() {
        0
    } else {
        1
    }
}

/// 篡改演示。内核拒绝是预期结果：返回 Ok 表示「证明确实有约束力」。
/// 证明门禁：等价于原 ci/check_proofs.sh，但用 Rust 实现。
/// 规则：拒绝 sorry/admit；只有 lean 退出码 0 才算 PASS。
fn check_proofs(files: &[String]) -> i32 {
    let mut fail = 0;
    for f in files {
        println!("== {f}");
        let text = match std::fs::read_to_string(f) {
            Ok(t) => t,
            Err(_) => {
                println!("   FAIL 文件不存在");
                fail = 1;
                continue;
            }
        };
        let forbidden = text
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|w| w == "sorry" || w == "admit");
        if forbidden {
            println!("   FAIL 含 sorry/admit，拒绝接受");
            fail = 1;
            continue;
        }
        match std::process::Command::new("lean").arg(f).output() {
            Ok(o) if o.status.success() => println!("   PASS 内核校验通过"),
            Ok(o) => {
                println!("   FAIL 内核拒绝:");
                let mut msg = String::from_utf8_lossy(&o.stdout).to_string();
                msg.push_str(&String::from_utf8_lossy(&o.stderr));
                for line in msg.lines().take(20) {
                    println!("{line}");
                }
                fail = 1;
            }
            Err(_) => {
                println!("   FAIL 找不到 lean 可执行文件");
                fail = 1;
            }
        }
    }
    fail
}

fn tamper(path: &str) -> assay_core::Result<()> {
    use std::io::Write;
    let text = std::fs::read_to_string(path)?;
    let mut changed: Option<(String, usize, usize)> = None;
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start();
        if changed.is_none()
            && trimmed.starts_with("theorem empty_")
            && trimmed.ends_with(":= by decide")
        {
            if let Some(eq) = trimmed.find(" = ") {
                let rest = &trimmed[eq + 3..];
                if let Some(end) = rest.find(" := by decide") {
                    let old: usize = rest[..end].trim().parse().unwrap_or(0);
                    let new_line = format!(
                        "{}{}{}{}",
                        &line[..line.find(trimmed).unwrap_or(0)],
                        &trimmed[..eq + 3],
                        old + 1,
                        &rest[end..]
                    );
                    changed = Some((trimmed.to_string(), old, old + 1));
                    out.push_str(&new_line);
                    out.push('\n');
                    continue;
                }
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    let Some((_t, old, new)) = changed else {
        return Err(assay_core::Error::Other("没找到可篡改的定理".into()));
    };
    let tmp = std::env::temp_dir().join(format!("assay-tamper-{}.lean", std::process::id()));
    std::fs::File::create(&tmp)?.write_all(out.as_bytes())?;
    println!("篡改: 空值数 {old} → {new}（{}）", tmp.display());
    let res = std::process::Command::new("lean").arg(&tmp).output()?;
    let stdout = String::from_utf8_lossy(&res.stdout);
    let stderr = String::from_utf8_lossy(&res.stderr);
    for l in stdout.lines().chain(stderr.lines()).take(12) {
        println!("{l}");
    }
    let _ = std::fs::remove_file(&tmp);
    if res.status.success() {
        return Err(assay_core::Error::Other(
            "篡改后仍然通过，证明没有约束力".into(),
        ));
    }
    println!("\n内核拒绝（预期）—— 退出码 {:?}", res.status.code());
    Ok(())
}

/// 极简静态服务器：只读文件、固定 MIME、拒绝路径穿越。
fn serve(dir: &str, port: u16) -> assay_core::Result<()> {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let root = std::fs::canonicalize(dir)?;
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!("serving {} on http://127.0.0.1:{port}", root.display());
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        if n == 0 {
            continue;
        }
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("/");
        let path = path.split('?').next().unwrap_or("/");
        let mut rel = path.trim_start_matches('/').to_string();
        if rel.is_empty() {
            rel = "index.html".into();
        }
        let mut target = std::fs::canonicalize(root.join(&rel)).ok();
        // 目录请求回落到 index.html（GitHub Pages 也是这个行为）
        if let Some(p) = target.clone() {
            if p.is_dir() {
                target = std::fs::canonicalize(p.join("index.html")).ok();
            }
        }
        let (status, body, mime) = match target {
            Some(p) if p.starts_with(&root) && p.is_file() => {
                ("200 OK", std::fs::read(&p).unwrap_or_default(), mime_of(&p))
            }
            _ => ("404 Not Found", b"not found".to_vec(), "text/plain"),
        };
        let header = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(&body);
    }
    Ok(())
}

fn mime_of(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "md" | "markdown" => "text/markdown; charset=utf-8",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "lean" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
