//! 数证 DataProof 的核心库。
//!
//! 分层约定：
//! - **纯函数**（`scoring` / `guard` / `matrix` / `lean`）不碰网络和文件，CI 里可全速跑；
//! - **IO 层**（`api` / `aggregate` / `evidence` / `proof` / `export`）显式接收路径与客户端，
//!   便于用固定快照做对比测试；
//! - **解释层**（`explain`）只产出文字，数字由 `guard` 把关，模型不参与取数。

pub mod aggregate;
pub mod api;
pub mod audit;
pub mod config;
pub mod evidence;
pub mod explain;
pub mod export;
pub mod guard;
pub mod lean;
pub mod matrix;
pub mod model;
pub mod proof;
pub mod scoring;
pub mod util;

pub use model::{CatalogRecord, Dataset};

/// 统一错误类型：给 CLI 用，库内部尽量返回具体错误。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("网络请求失败: {0}")]
    Http(String),
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
