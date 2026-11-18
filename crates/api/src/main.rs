//! 查询服务的入口，同时带一个灌库子命令。
//!
//! 两个模式共用同一份 `DATABASE_URL` 与 migrations：
//!
//! ```bash
//! assay-api                                    # 起服务（等价于 assay-api serve）
//! assay-api serve                              # 同上
//! assay-api ingest --captured-at 2026-11-18    # 把 out-full 灌进库
//! ```
//!
//! 启动时自动跑 migrations——单实例、无状态、可随时重建，这是「零运维」在
//! 有后端之后仍然保留的那部分。**不要**把证据文件放进来，它们是静态对象。

use assay_api::ingest::{ingest, IngestOptions};
use assay_api::{app, connect, AppState};
use clap::{Parser, Subcommand};
use std::error::Error;

const DEFAULT_PORT: u16 = 8081;

#[derive(Parser)]
#[command(name = "assay-api", about = "数验查询服务（只读）与灌库工具")]
struct Args {
    /// 不带子命令时默认起服务，保持 `assay-api` 的老用法可用
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// 起查询服务
    Serve,
    /// 把一次跑批的产物灌进库（幂等，同一天重跑会覆盖）
    Ingest {
        /// 跑批产物目录（含 items.jsonl / summary.json）
        #[arg(long, default_value = "out-full")]
        out: String,
        /// 体检基准日 YYYY-MM-DD。必须显式给：同一次跑批今天灌和明天灌要落到同一行
        #[arg(long)]
        captured_at: String,
        /// 产生这批产物的 commit，便于回溯
        #[arg(long)]
        commit: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let args = Args::parse();
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "缺少 DATABASE_URL，例如 postgres://assay:assay@127.0.0.1:5432/assay")?;
    let pool = connect(&database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    match args.cmd {
        Some(Cmd::Ingest {
            out,
            captured_at,
            commit,
        }) => {
            let o = ingest(
                &pool,
                &IngestOptions {
                    out_dir: &out,
                    captured_at: &captured_at,
                    commit: commit.as_deref(),
                },
            )
            .await?;
            tracing::info!(
                run_id = o.run_id,
                datasets = o.datasets,
                observations = o.observations,
                dropped = o.dropped,
                "灌库完成"
            );
            println!(
                "灌库完成：run #{}, {} 条数据集 / {} 条观测（丢弃 {} 条重复或空 id）",
                o.run_id, o.datasets, o.observations, o.dropped
            );
        }
        Some(Cmd::Serve) | None => {
            tracing::info!("migrations 已应用");
            let port: u16 = match std::env::var("PORT") {
                Ok(v) => v.parse().map_err(|_| format!("PORT 不是合法端口：{v}"))?,
                Err(_) => DEFAULT_PORT,
            };
            let addr = format!("127.0.0.1:{port}");
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            tracing::info!(%addr, "assay-api 已启动");
            axum::serve(listener, app(AppState { pool }))
                .with_graceful_shutdown(shutdown())
                .await?;
        }
    }
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "assay_api=info,tower_http=info".into()),
        )
        .init();
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("收到中断信号，等待在途请求结束");
}
