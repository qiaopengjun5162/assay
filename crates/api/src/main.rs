//! 只读查询服务的入口。
//!
//! 启动时自动跑 migrations——单实例、无状态、可随时重建，这是「零运维」在
//! 有后端之后仍然保留的那部分。**不要**把证据文件放进来，它们是静态对象。

use assay_api::{app, connect, AppState};
use std::error::Error;

const DEFAULT_PORT: u16 = 8081;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "assay_api=info,tower_http=info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "缺少 DATABASE_URL，例如 postgres://assay:assay@127.0.0.1:5432/assay")?;
    let port: u16 = match std::env::var("PORT") {
        Ok(v) => v.parse().map_err(|_| format!("PORT 不是合法端口：{v}"))?,
        Err(_) => DEFAULT_PORT,
    };

    let pool = connect(&database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("migrations 已应用");

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "assay-api 已启动");
    axum::serve(listener, app(AppState { pool }))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("收到中断信号，等待在途请求结束");
}
