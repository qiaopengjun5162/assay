//! assay CLI：体检、聚合、解释、证据、证明、导出、本地服务。
//!
//! 所有子命令共享 `--out` 目录约定（默认 `out-full`），产物与旧版脚本一一对应。

use assay_core::api::{CatalogSource, Client};
use assay_core::{
    aggregate, audit, checks, drift, evidence, explain, export, fixture_source::FixtureSource,
    history, pack, proof,
};
use clap::{Parser, Subcommand};
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

    /// 数据源：本地 JSON fixture 路径。省略则用内置浙江平台客户端。
    /// 给路径即走离线适配器（不触网），用于复用演示与接自有数据。
    #[arg(long, global = true)]
    source: Option<String>,

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
        /// 只发一次请求自检端点 / 模型 / key / 代理，不跑批
        #[arg(long)]
        probe: bool,
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
        /// 同时产出 index.json + shards/（全量，不受 --limit 影响）
        ///
        /// 单文件 data.json 是全量加载的，一周涨约 723KB。分层后列表只读索引，
        /// 详情按分片懒加载。两个产物并存，前端可分步迁移。
        #[arg(long)]
        split: bool,
        /// 分片数量（配合 --split）
        #[arg(long, default_value_t = 24)]
        shards: usize,
        /// 快照日期（YYYY-MM-DD）。默认从 `<out>/run.json` 推，没有就从证据时间推
        #[arg(long)]
        snapshot_date: Option<String>,
        /// 跑批历史日志（默认 history/summary.json）。只读，嵌进 data.json 的 history
        #[arg(long, default_value = "history/summary.json")]
        history: String,
    },
    /// 批量生成单数据集报告（前 N 名，Markdown，写进站点目录）
    Reports {
        #[arg(long, default_value_t = 100)]
        top: usize,
        /// 输出目录（默认 web/reports）；export 从这里判断哪些条有报告
        #[arg(long, default_value = "web/reports")]
        dest: String,
        /// 证据根目录：必须与入库 web/evidence 同一批，否则报告里的哈希全部对不上
        #[arg(long)]
        evidence_dir: Option<String>,
        /// 不把 AI 解读写进报告：报告只放可核对的事实，解读留在网页
        #[arg(long)]
        no_explain: bool,
    },
    /// 单数据集报告：渲染成 Markdown（可直接贴进评审流程）
    ReportOne {
        /// 数据集 ID
        id: String,
        /// 输出文件；省略则打印到标准输出
        #[arg(long)]
        to: Option<String>,
        /// 证据根目录，同上
        #[arg(long)]
        evidence_dir: Option<String>,
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
    /// 报告复核：报告里印的 SHA-256 必须等于证据文件重算的结果
    ///
    /// 报告和证据必须来自同一次抓取。用错 `--out` 补报告，会生成一批哈希指向
    /// 别处的报告——每份都看着完整，读者逐份核对却全部失败。
    VerifyReports {
        /// 索引文件：data.json（按 datasets[].evidence 判断哪些该有报告）
        index: String,
        /// 报告目录（`<safe_id>.md`）
        reports_dir: String,
        /// 证据根目录
        evidence_dir: String,
    },
    /// 篡改演示：把一条定理的计数 +1，确认内核会拒绝
    Tamper {
        /// 证明文件（默认读 --out 下的 TopProofs.lean）
        #[arg(long)]
        file: Option<String>,
    },
    /// 周度漂移对比：两次 summary.json 比指标，用退出码表达结论
    ///
    /// 退出码：0 正常 / 1 漂移（该开 PR）/ 2 跑批不可信（别更新基线）
    Drift {
        /// 基线文件（对照系）
        #[arg(long, default_value = "baseline/summary.json")]
        baseline: String,
        /// 本次的 summary.json；省略则用 --out 下的
        #[arg(long)]
        current: Option<String>,
        /// 判定为漂移时，把本轮指标写成新基线（随 PR 合并才生效）
        #[arg(long)]
        write_baseline: Option<String>,
        /// 同时写出 Markdown 报告（PR 正文用）
        #[arg(long)]
        md_out: Option<String>,
        /// 采集日期（YYYY-MM-DD），默认取当天
        #[arg(long)]
        captured_at: Option<String>,
        /// 采集时的提交号，写进基线供追溯
        #[arg(long)]
        commit: Option<String>,
        /// 比例类指标阈值（pp）
        #[arg(long)]
        ratio_pp: Option<f64>,
        /// 中位滞后天数阈值（天）
        #[arg(long)]
        lag_days: Option<i64>,
        /// 空值率中位数阈值（0~1）
        #[arg(long)]
        empty_rate: Option<f64>,
    },
    /// 打审计包：把一次体检的产物压成可交付的单文件
    ///
    /// 包内自带 VERIFY.md 与标准 shasum 清单，解压后一条系统命令验完全部文件，
    /// 不需要安装本工具。
    Pack {
        /// 输出文件；省略则 dist/assay-audit-<captured-at>.tar.gz
        #[arg(long)]
        dest: Option<String>,
        /// 体检基准日 YYYY-MM-DD（写进包内目录名与清单）
        #[arg(long)]
        captured_at: String,
        /// 产出这批数据的 commit，写进清单供追溯
        #[arg(long)]
        commit: Option<String>,
    },
    /// 跑批历史：把这一轮的指标追加进日志，攒成曲线
    ///
    /// 同一天重跑会**替换**当天那一条，不会在图上画两个点。
    /// 日期默认取 `<--current>` 旁边的 `run.json`（跑批开跑时间），
    /// 补跑旧快照时用 `--date` 指定，别用「今天」——图的横轴会全错。
    History {
        /// 这一轮的 summary.json；省略则用 --out 下的
        #[arg(long)]
        current: Option<String>,
        /// 历史日志（不存在就新建）
        #[arg(long, default_value = "history/summary.json")]
        log: String,
        /// 采集日期 YYYY-MM-DD；省略则从 run.json 推
        #[arg(long)]
        date: Option<String>,
    },
    /// 本地静态服务（替代 python -m http.server）
    Serve {
        #[arg(long, default_value = "web")]
        dir: String,
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// 列出全部检查项（人话 + 维度 + 阈值 + 权重），可附命中数
    ///
    /// 「你们凭什么说它有问题？」——答案就是这张表。不带 --items 只打定义；
    /// 带上跑批产物会多一列命中数，让「门槛多少、命中了多少」一起对得上。
    Checks {
        /// items.jsonl；省略则用 <--out>/items.jsonl（不存在则只打定义）
        #[arg(long)]
        items: Option<String>,
        /// 目录快照（逾期判定要用更新时间）；省略则用 <--out>/catalog_snapshot.json
        #[arg(long)]
        catalog: Option<String>,
        /// 只输出 JSON（便于喂给别的工具）
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("错误: {e}");
        std::process::exit(1);
    }
}

/// 按 --source 构造数据源：给了路径走离线 fixture，否则用内置浙江客户端。
///
/// 内核（audit/evidence/prove）只认 `&dyn CatalogSource`，所以这里是我们唯一
/// 需要知道「有哪几种数据源」的地方——换平台时只动这一处。
fn make_source(source: Option<&str>) -> assay_core::Result<Box<dyn CatalogSource>> {
    match source {
        Some(p) => Ok(Box::new(FixtureSource::open(Path::new(p))?)),
        None => Ok(Box::new(Client::new())),
    }
}

fn run(cli: Cli) -> assay_core::Result<()> {
    let out = cli.out;
    let source = cli.source;
    match cli.command {
        Command::Audit { threads, limit } => {
            let client = make_source(source.as_deref())?;
            let o = audit::audit(
                &audit::AuditOptions {
                    threads,
                    limit,
                    out_dir: out,
                },
                client.as_ref(),
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
        Command::Explain {
            top,
            threads,
            probe,
        } => {
            if probe {
                let r = explain::probe()?;
                println!("端点   {}", r.endpoint);
                println!("模型   {}", r.model);
                println!("协议   {}", r.protocol);
                println!("key    {}（来源：{}）", r.masked_key, r.source);
                println!("代理   {}", r.proxy.as_deref().unwrap_or("直连"));
                println!("响应   {:?}  {:.1}s", r.reply, r.latency_secs);
                return Ok(());
            }
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
                "解释层: {}/{} 条通过护栏，{:.1}s（模型 {}）",
                o.passed, o.total, o.elapsed_secs, o.model
            );
        }
        Command::Evidence { top, threads } => {
            let client = make_source(source.as_deref())?;
            let items = audit::read_items(&Path::new(&out).join("items.jsonl").to_string_lossy())?;
            let o = evidence::capture_top(&items, top, &out, client.as_ref(), threads)?;
            println!("证据包: {}/{} 条", o.captured, o.total);
        }
        Command::Prove { top } => {
            proof::require_lean()?;
            let client = make_source(source.as_deref())?;
            let items = audit::read_items(&Path::new(&out).join("items.jsonl").to_string_lossy())?;
            let o = proof::prove_top(&items, top, &out, client.as_ref())?;
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
        Command::Export {
            limit,
            dest,
            split,
            shards,
            snapshot_date,
            history,
        } => {
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
                snapshot_date: snapshot_date.as_deref(),
                history_path: Some(&history),
                shards: split.then_some(shards),
            })?;
            println!(
                "导出 {} 条（共 {}）：证明 {} / 证据 {} / 解读 {}",
                o.rows, o.total, o.proofs, o.evidence, o.explanations
            );
            if o.shards_written > 0 {
                println!(
                    "分层：index.json {} 条（全量）+ {} 个分片 → {}",
                    o.total,
                    o.shards_written,
                    Path::new(&dest)
                        .parent()
                        .unwrap_or(Path::new("."))
                        .display()
                );
            }
        }
        Command::Reports {
            top,
            dest,
            evidence_dir,
            no_explain,
        } => {
            let items = assay_core::audit::read_items(&format!("{out}/items.jsonl"))?;
            let explanations = if no_explain {
                std::collections::HashMap::new()
            } else {
                assay_core::single::load_explanations(&out)
            };
            let evidence_dir =
                std::path::PathBuf::from(evidence_dir.unwrap_or_else(|| format!("{out}/evidence")));
            let o = assay_core::single::render_top_with_evidence(
                &items,
                top,
                &evidence_dir,
                &dest,
                &explanations,
            )?;
            println!(
                "报告: {} 条 → {dest}（证据来自 {}）",
                o.written,
                evidence_dir.display()
            );
        }
        Command::ReportOne {
            id,
            to,
            evidence_dir,
        } => {
            let items = assay_core::audit::read_items(&format!("{out}/items.jsonl"))?;
            let d = items
                .iter()
                .find(|d| d.id == id)
                .ok_or_else(|| assay_core::Error::Other(format!("未找到数据集 {id}")))?;
            let evidence_dir =
                std::path::PathBuf::from(evidence_dir.unwrap_or_else(|| format!("{out}/evidence")));
            let (ev, rows) = assay_core::single::load_evidence_dir(&evidence_dir, &id);
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
        Command::VerifyReports {
            index,
            reports_dir,
            evidence_dir,
        } => {
            std::process::exit(verify_reports(&index, &reports_dir, &evidence_dir));
        }
        Command::Tamper { file } => {
            let path = file.unwrap_or_else(|| format!("{out}/TopProofs.lean"));
            tamper(&path)?;
        }
        Command::Drift {
            baseline,
            current,
            write_baseline,
            md_out,
            captured_at,
            commit,
            ratio_pp,
            lag_days,
            empty_rate,
        } => {
            let current = current.unwrap_or_else(|| format!("{out}/summary.json"));
            let mut th = drift::Thresholds::default();
            if let Some(v) = ratio_pp {
                th.ratio_pp = v;
            }
            if let Some(v) = lag_days {
                th.lag_days = v;
            }
            if let Some(v) = empty_rate {
                th.empty_rate = v;
            }

            let mut cur = match read_summary(&current) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("错误: {e}");
                    std::process::exit(2);
                }
            };
            // 采集日期和提交号写进基线，以后才能回答「这条基线是哪来的」。
            if cur.captured_at.is_none() {
                cur.captured_at = Some(captured_at.unwrap_or_else(today));
            }
            if cur.source_commit.is_none() {
                cur.source_commit = commit;
            }

            // 首跑：没有对照系就不判漂移，只落基线。
            // 不报 Healthy——「没比过」和「比过且正常」必须能区分开。
            if !Path::new(&baseline).exists() {
                println!("基线 {baseline} 不存在，首跑：跳过判定");
                match &write_baseline {
                    Some(dest) => match cur.to_baseline_text().and_then(|t| write_text(dest, &t)) {
                        Ok(()) => println!("已写入基线 {dest}"),
                        Err(e) => {
                            eprintln!("错误: {e}");
                            std::process::exit(2);
                        }
                    },
                    None => println!("提示：加 --write-baseline {baseline} 落一份基线"),
                }
                std::process::exit(0);
            }

            let base = match read_summary(&baseline) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("错误: {e}");
                    std::process::exit(2);
                }
            };
            let report = drift::compare(&base, &cur, &th);
            let md = report.to_markdown();

            if let Some(dest) = &md_out {
                if let Err(e) = write_text(dest, &md) {
                    eprintln!("错误: {e}");
                    std::process::exit(2);
                }
                println!("报告 → {dest}");
            }
            print!("{md}");

            match &report.verdict {
                drift::Verdict::Suspect { reason } => {
                    eprintln!("跑批不可信：{reason}");
                    eprintln!("基线未更新（{baseline}）——脏数据不能进对照系");
                }
                // 正常不动基线：基线代表「上次人工确认过的常态」。
                // 这样渐进漂移会累积到越过阈值才报，而不是每周都跟着数据走。
                drift::Verdict::Healthy => {
                    if write_baseline.is_some() {
                        println!("判定正常，基线不动（基线随漂移 PR 合并才前进）");
                    }
                }
                drift::Verdict::Drifted => {
                    if let Some(dest) = &write_baseline {
                        match cur.to_baseline_text().and_then(|t| write_text(dest, &t)) {
                            Ok(()) => println!("基线已更新 → {dest}（随 PR 合并才生效）"),
                            Err(e) => {
                                eprintln!("错误: {e}");
                                std::process::exit(2);
                            }
                        }
                    }
                }
            }
            std::process::exit(report.verdict.exit_code());
        }
        Command::History { current, log, date } => {
            let current = current.unwrap_or_else(|| format!("{out}/summary.json"));
            let summary = read_json_value(&current)
                .ok_or_else(|| assay_core::Error::Other(format!("缺少 {current}")))?;
            let date = match date {
                Some(d) => d,
                None => date_of_run(&current)?,
            };
            // 缺字段说明这份 summary 不是本工具的产物；不猜、不补 0。
            let point = history::Point::from_summary(&date, &summary).ok_or_else(|| {
                assay_core::Error::Other(format!(
                    "{current} 里缺指标或日期 {date} 不合法，不记这一轮"
                ))
            })?;
            let points = history::append(&log, point)?;
            let first = points.first().map(|p| p.date.as_str()).unwrap_or("-");
            let last = points.last().map(|p| p.date.as_str()).unwrap_or("-");
            println!("历史 {log}：{} 点（{first} → {last}）", points.len());
            if points.len() < 2 {
                println!("提示：曲线要两点以上才画得出来，等下一次周报");
            }
        }
        Command::Pack {
            dest,
            captured_at,
            commit,
        } => {
            let dest = dest.unwrap_or_else(|| format!("dist/assay-audit-{captured_at}.tar.gz"));
            let o = pack::pack(&pack::PackOptions {
                out_dir: &out,
                dest: &dest,
                captured_at: &captured_at,
                commit: commit.as_deref(),
            })?;
            println!("审计包 → {dest}");
            println!(
                "  {} 个文件 / {} 条证据；原始 {:.1} MB → {:.1} MB",
                o.files,
                o.evidence_entries,
                o.bytes_raw as f64 / 1_048_576.0,
                o.bytes_packed as f64 / 1_048_576.0
            );
            // 直接把验证命令打出来：交付时这一行就是要转给对方的话。
            println!("  验：tar xzf {dest} && cd assay-audit-{captured_at} && shasum -a 256 -c MANIFEST.sha256");
        }
        Command::Serve { dir, port } => serve(&dir, port)?,
        Command::Checks {
            items,
            catalog,
            json,
        } => {
            let items_path = items.unwrap_or_else(|| format!("{out}/items.jsonl"));
            let catalog_path = catalog.unwrap_or_else(|| format!("{out}/catalog_snapshot.json"));
            print_checks(&items_path, &catalog_path, json)?;
        }
    }
    Ok(())
}

/// 打检查项清单：定义 + 本批命中数。
///
/// 命中数按整批算，不受 `export --limit` 影响——「2,791 条里 163 条」这种句子
/// 两边的数必须来自同一个分母，否则又是一处「看起来对、对不上」。
fn print_checks(items_path: &str, catalog_path: &str, as_json: bool) -> assay_core::Result<()> {
    let items: Vec<assay_core::Dataset> = if Path::new(items_path).is_file() {
        audit::read_items(items_path)?
    } else {
        Vec::new()
    };
    let update: std::collections::HashMap<String, String> = if Path::new(catalog_path).is_file() {
        aggregate::load_catalog(catalog_path)?
            .into_iter()
            .filter_map(|r| r.update_time.map(|t| (r.id, t)))
            .collect()
    } else {
        Default::default()
    };

    let tally = checks::tally(
        items
            .iter()
            .map(|d| checks::Facts::with_catalog(d, update.get(&d.id).map(|s| s.as_str()))),
    );
    let total = items.len();

    if as_json {
        let rows: Vec<serde_json::Value> = tally
            .iter()
            .map(|(c, hits)| {
                serde_json::json!({
                    "id": c.id,
                    "title": c.title,
                    "dim": c.dim,
                    "weight": c.weight,
                    "note": c.note,
                    "hits": hits,
                    "of": total,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    if total == 0 {
        println!(
            "检查项 {} 条（没找到 {items_path}，只打定义；跑一次 audit 或传 --items 才有命中数）",
            tally.len()
        );
    } else {
        println!("检查项 {} 条，本批 {total} 条数据的命中数：", tally.len());
    }
    println!();
    println!(
        "{} {} {} {} 人话",
        pad("id", 20),
        pad("维度", 8),
        pad("命中", 12),
        pad("权重", 5)
    );
    for (c, hits) in &tally {
        let hit = if total == 0 {
            "—".to_string()
        } else {
            format!("{hits}/{total}")
        };
        let w = if c.weight.fract() == 0.0 {
            format!("{:.0}", c.weight)
        } else {
            format!("{}", c.weight)
        };
        println!(
            "{} {} {} {} {}",
            pad(c.id, 20),
            pad(c.dim, 8),
            pad(&hit, 12),
            pad(&w, 5),
            c.title
        );
    }
    println!();
    println!("权重 0 = 只做标注、不影响排序；阈值与权重全部在 crates/core/src/checks.rs，");
    println!("改那里就是改结论，站点上的清单与这里同源。");
    Ok(())
}

/// 终端表格按显示宽度对齐：中文占两格，按字节或字符数都会被拉歪。
fn pad(s: &str, w: usize) -> String {
    let mut out = s.to_string();
    for _ in width(s)..w {
        out.push(' ');
    }
    out
}

fn width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let cp = c as u32;
            let wide = matches!(cp,
                0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3
                | 0xF900..=0xFAFF | 0xFE30..=0xFE6F
                | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6
                | 0x20000..=0x2FFFD);
            if wide {
                2
            } else {
                1
            }
        })
        .sum()
}

/// 读 summary.json。基线既可能是纯指标切片，也可能是完整跑批快照，
/// 两种形状都接受（形状判断在 `drift::Summary::from_value` 里）。
fn read_summary(path: &str) -> assay_core::Result<drift::Summary> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| assay_core::Error::Other(format!("读不到 {path}：{e}")))?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| assay_core::Error::Other(format!("{path} 不是合法 JSON：{e}")))?;
    drift::Summary::from_value(v)
}

fn write_text(dest: &str, text: &str) -> assay_core::Result<()> {
    // 报告目录可能还没建（CI 里第一次跑），这里顺手补上，
    // 免得 workflow 依赖「目录恰好被提交过」这种隐式前提。
    if let Some(dir) = Path::new(dest).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| {
                assay_core::Error::Other(format!("建目录 {} 失败：{e}", dir.display()))
            })?;
        }
    }
    std::fs::write(dest, text).map_err(|e| assay_core::Error::Other(format!("写不到 {dest}：{e}")))
}

/// 当天日期（UTC+8）。只用于基线元数据，不参与任何评分口径。
fn today() -> String {
    let s = assay_core::util::now_iso8601();
    s.get(..10).unwrap_or(&s).to_string()
}

fn read_json_value(path: &str) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// 这一轮跑批的日期：优先 `<summary 同目录>/run.json` 的 `capturedAt`。
///
/// 不用「今天」当默认值：补跑旧快照时「今天」会把横轴写成错的日期，
/// 而且这个错误没有任何报错——图上只是多了一个不该有的点。
fn date_of_run(current: &str) -> assay_core::Result<String> {
    let dir = Path::new(current).parent().unwrap_or(Path::new("."));
    let run = dir.join("run.json");
    let ts = read_json_value(&run.to_string_lossy())
        .and_then(|v| v["capturedAt"].as_str().map(str::to_string));
    match ts.as_deref().and_then(|s| s.get(..10)).map(str::to_string) {
        Some(d) if assay_core::history::is_iso_date(&d) => Ok(d),
        _ => Err(assay_core::Error::Other(format!(
            "推不出这一轮的日期：{} 里没有合法的 capturedAt。\
             补跑旧快照时用 --date YYYY-MM-DD 指定，不要用「今天」。",
            run.display()
        ))),
    }
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

/// 报告复核：逻辑在 `single::verify_reports`，这里只打结果与给退出码。
///
/// 成功的**不逐条打**——500 份全 OK 的日志会淹掉真正的失败。
/// 失败逐条打，带具体是哪个哈希对不上。一份都没复核也算失败：空跑不能当通过。
fn verify_reports(index: &str, reports_dir: &str, evidence_dir: &str) -> i32 {
    let checks = match assay_core::single::verify_reports(
        std::path::Path::new(index),
        std::path::Path::new(reports_dir),
        std::path::Path::new(evidence_dir),
    ) {
        Ok(c) => c,
        Err(e) => {
            println!("FAIL 复核不了 {index}：{e}");
            return 1;
        }
    };
    if checks.is_empty() {
        println!("FAIL 一份报告都没复核：{index} 里没有带证据的数据集？");
        return 1;
    }
    let mut bad = 0usize;
    for c in &checks {
        if let Some(p) = &c.problem {
            println!("   FAIL {}：{p}", c.name);
            bad += 1;
        }
    }
    println!("== {} / {} 份一致", checks.len() - bad, checks.len());
    i32::from(bad > 0)
}

/// 篡改演示。内核拒绝是预期结果：返回 Ok 表示「证明确实有约束力」。/// 证明门禁：等价于原 ci/check_proofs.sh，但用 Rust 实现。
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
    // 屏幕上的第一行要短：演示时投影不会给长路径留位置，但也不能说假话——
    // 括号里标明这是临时副本，原文件没动。
    println!(
        "篡改: 空值数 {old} → {new}（临时副本 {}，原文件未动）",
        tmp.file_name().unwrap_or_default().to_string_lossy()
    );
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
    // code() 为 None 表示被信号杀掉，不能打印成 `Some(1)` —— 那是给调试看的，不是给人看的。
    match res.status.code() {
        Some(code) => println!("\n内核拒绝（预期）—— lean 退出码 {code}"),
        None => println!("\n内核拒绝（预期）—— lean 被信号终止"),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 中文是双宽字符。按字节或字符数补齐都会把表拉歪，
    /// 而「表格不齐」这种东西没有测试会红，只能靠肉眼看出来。
    #[test]
    fn width_counts_cjk_as_two_cells() {
        assert_eq!(width("abc"), 3);
        assert_eq!(width("维度"), 4);
        assert_eq!(width("命中 12"), 7);
        assert_eq!(width(""), 0);
    }

    #[test]
    fn pad_fills_by_display_width_without_truncating() {
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("维度", 4), "维度");
        assert_eq!(pad("维度", 6), "维度  ");
        // 超宽不截断：少一列比少一个字好查
        assert_eq!(pad("abc", 2), "abc");
    }

    fn temp_fixture(tag: &str, body: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "assay-cli-source-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&p, body).unwrap();
        p
    }

    /// --source 是赛后接真实业务的唯一入口，但 CLI 层此前零测试：
    /// core 单测全绿也发现不了这里漏接 fixture 分支（reuse-demo 不在 CI 里）。
    #[test]
    fn make_source_switches_between_builtin_and_fixture() {
        let builtin = make_source(None).expect("默认数据源应能构造");
        assert_eq!(builtin.platform(), "浙江省公共数据开放平台");

        let p = temp_fixture(
            "ok",
            r#"{"platform":"测试源","declaredColumns":["a"],"items":[{"id":"x","a":"v"}]}"#,
        );
        let fixture = make_source(Some(p.to_str().unwrap())).expect("合法 fixture 应能加载");
        assert_eq!(
            fixture.platform(),
            "测试源",
            "--source 必须切到 fixture 适配器"
        );
        std::fs::remove_file(&p).unwrap();
    }

    /// 坏 fixture 的错误要说清是哪一类问题，别让用户对着 "IO 失败" 猜。
    #[test]
    fn make_source_reports_bad_fixture_with_context() {
        // 用 err().expect() 而非 unwrap_err()：后者要求 Ok 类型（Box<dyn CatalogSource>）
        // 实现 Debug，而 trait 对象没有——这是测试里最常见的假编译错误。
        let missing = make_source(Some("/nonexistent/assay-fixture.json"))
            .err()
            .expect("不存在的文件必须报错");
        assert!(
            missing.to_string().contains("读取 fixture"),
            "缺文件应提示是读取 fixture 失败: {missing}"
        );

        let p = temp_fixture("bad", r#"{"platform": 不是合法 JSON"#);
        let broken = make_source(Some(p.to_str().unwrap()))
            .err()
            .expect("坏 JSON 必须报错");
        assert!(
            broken.to_string().contains("不是合法 JSON"),
            "坏 JSON 应提示解析失败: {broken}"
        );
        std::fs::remove_file(&p).unwrap();
    }
}
