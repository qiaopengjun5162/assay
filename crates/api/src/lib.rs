//! 查询后端（只读）。
//!
//! 边界：**这里不存证据**。原始响应、SHA-256、Lean 证明都是静态对象，
//! 浏览器/买方可以直接下载、离线校验。这个服务只回答「哪一次体检、每条多少分、
//! 随时间怎么变」——也就是静态站装不下的那部分（见 D20）。
//!
//! 所以对外只有四个 GET，没有写接口、没有账号、没有任务队列。
//! 加 HTTP 写接口之前先想清楚：写进来的东西要不要也进证据链？如果要，先设计签名。
//!
//! 唯一的写路径是 [`ingest`]：把一次跑批的产物灌进库。它是**运维/CI 用的 CLI**，
//! 不监听端口、不接受网络请求——「谁能写库」只需要一个人，「谁能查询」才需要一个服务。

pub mod ingest;
pub mod query;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use query::DatasetQuery;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool};
use std::time::Duration;

/// 浙江省平台的固定主键，与 migrations 里的 insert 对应。
const PLATFORM_ZJ: i16 = 1;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("数据库错误：{0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    NotFound(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (code, msg) = match &self {
            ApiError::Db(e) => {
                // 不外泄 SQL 细节：错误里可能带表名和参数
                tracing::error!(error = %e, "数据库查询失败");
                (StatusCode::INTERNAL_SERVER_ERROR, "内部错误".to_string())
            }
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
        };
        (code, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/runs", get(list_runs))
        .route("/v1/datasets", get(list_datasets))
        .route("/v1/datasets/{id}/history", get(dataset_history))
        // 每个请求记一条日志（方法/路径/状态/耗时）。线上排障时，
        // 「哪个查询慢」比「服务挂了」更需要现场证据。
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

#[derive(Serialize)]
struct Health {
    ok: bool,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct RunRow {
    id: i64,
    captured_at: String,
    audited: i32,
    with_preview: i32,
    commit_sha: Option<String>,
}

async fn list_runs(State(st): State<AppState>) -> Result<Json<Vec<RunRow>>, ApiError> {
    let rows = sqlx::query_as::<_, RunRow>(
        "select id, captured_at, audited, with_preview, commit_sha
           from run where platform_id = $1
          order by captured_at desc limit 52",
    )
    .bind(PLATFORM_ZJ)
    .fetch_all(&st.pool)
    .await?;
    Ok(Json(rows))
}

/// 榜单行。**字段集与 `web/index.json` 的单行完全一致**——前端从读静态文件换成
/// 调接口时，改的只是 fetch 那一行，`Row` 结构体不用动。
#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DatasetRow {
    /// 平台原始 id（不是库里的自增主键）
    pub id: String,
    pub name: String,
    pub dept: String,
    pub update_cycle: String,
    pub lag_days: Option<i32>,
    pub stopped: bool,
    pub declared_cols: i32,
    pub sample_cols: i32,
    pub empty_rate: f32,
    pub all_empty_cols: i32,
    pub schema_kind: String,
    pub score: f32,
    pub has_proof: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    run_id: i64,
    captured_at: String,
    total: i64,
    page: u32,
    size: u32,
    items: Vec<DatasetRow>,
}

async fn list_datasets(
    State(st): State<AppState>,
    Query(raw): Query<DatasetQuery>,
) -> Result<Json<Page>, ApiError> {
    let run = match raw.run {
        Some(id) => latest_run_date(&st.pool, id).await?,
        None => {
            let id = latest_run_id(&st.pool).await?;
            latest_run_date(&st.pool, id).await?
        }
    };
    let q = raw.normalize();

    let where_sql = query::Normalized::where_clause();

    // 计数与取页用同一段 where，避免「总数和实际行对不上」这种翻页鬼故事
    let count_sql = format!(
        "select count(*) from observation o join dataset d on d.id = o.dataset_id where {where_sql}"
    );
    let total: i64 = sqlx::query_scalar(&count_sql)
        .bind(run.0)
        .bind(q.q.as_deref())
        .bind(q.dept.as_deref())
        .bind(q.stopped)
        .bind(q.mismatch)
        .bind(q.min_empty)
        .fetch_one(&st.pool)
        .await?;

    let sql = format!(
        "select d.ext_id as id, d.name, coalesce(d.dept, '') as dept,
                o.update_cycle, o.lag_days, o.stopped, o.declared_cols,
                o.sample_cols, o.empty_rate, o.all_empty_cols,
                o.schema_kind, o.score, o.has_proof
           from observation o
           join dataset d on d.id = o.dataset_id
          where {where_sql}
          order by {}
          limit $7 offset $8",
        q.sort.order_by()
    );
    let items = sqlx::query_as::<_, DatasetRow>(&sql)
        .bind(run.0)
        .bind(q.q.as_deref())
        .bind(q.dept.as_deref())
        .bind(q.stopped)
        .bind(q.mismatch)
        .bind(q.min_empty)
        .bind(i64::from(q.size))
        .bind(q.offset())
        .fetch_all(&st.pool)
        .await?;

    Ok(Json(Page {
        run_id: run.0,
        captured_at: run.1,
        total,
        page: q.page,
        size: q.size,
        items,
    }))
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct HistoryPoint {
    captured_at: String,
    score: f32,
    lag_days: Option<i32>,
    empty_rate: f32,
    stopped: bool,
}

/// 单条数据集的时间序列。**这是静态托管做不到的那件事**：一周一个点，
/// 想查哪条就查哪条，不需要为每个数据集预生成一份文件。
async fn dataset_history(
    State(st): State<AppState>,
    Path(ext_id): Path<String>,
) -> Result<Json<Vec<HistoryPoint>>, ApiError> {
    let rows = sqlx::query_as::<_, HistoryPoint>(
        "select r.captured_at, o.score, o.lag_days, o.empty_rate, o.stopped
           from observation o
           join run r on r.id = o.run_id
           join dataset d on d.id = o.dataset_id
          where d.platform_id = $1 and d.ext_id = $2
          order by r.captured_at",
    )
    .bind(PLATFORM_ZJ)
    .bind(&ext_id)
    .fetch_all(&st.pool)
    .await?;
    if rows.is_empty() {
        return Err(ApiError::NotFound(format!(
            "没有数据集 {ext_id} 的体检记录"
        )));
    }
    Ok(Json(rows))
}

async fn latest_run_id(pool: &PgPool) -> Result<i64, ApiError> {
    // 按 captured_at 取最近一次，**不用 max(id)**：`run.id` 是 serial，
    // 而 upsert 冲突（同一天重灌）也会消耗序列，所以 id 顺序不等于日期顺序。
    // 重灌一次旧日期就会让 max(id) 指到旧数据上，接口默认返回的榜单会悄悄退版本。
    // captured_at 是 ISO 文本，字符串排序即时间排序。
    let id: Option<i64> = sqlx::query_scalar(
        "select id from run where platform_id = $1 order by captured_at desc limit 1",
    )
    .bind(PLATFORM_ZJ)
    // 必须 fetch_optional：fetch_one 在空表时返回 RowNotFound，
    // 会被映射成 500「内部错误」，而这里该给的是「还没跑过体检」。
    .fetch_optional(pool)
    .await?;
    id.ok_or_else(|| ApiError::NotFound("还没有任何体检记录，先跑一次 assay audit".into()))
}

async fn latest_run_date(pool: &PgPool, run_id: i64) -> Result<(i64, String), ApiError> {
    let row: Option<(i64, String)> =
        sqlx::query_as("select id, captured_at from run where id = $1 and platform_id = $2")
            .bind(run_id)
            .bind(PLATFORM_ZJ)
            .fetch_optional(pool)
            .await?;
    row.ok_or_else(|| ApiError::NotFound(format!("没有 id={run_id} 的体检记录")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_row_serializes_to_index_json_shape() {
        // 前端 `Row` 反序列化靠这些名字；改名会让前端静默丢字段
        let r = DatasetRow {
            id: "HuZoVx_QGmxtyEgbHu_Ulw==".into(),
            name: "少数民族行政村信息".into(),
            dept: "省民宗委".into(),
            update_cycle: "-1".into(),
            lag_days: Some(522),
            stopped: true,
            declared_cols: 4,
            sample_cols: 3,
            empty_rate: 0.6667,
            all_empty_cols: 2,
            schema_kind: "列数不一致".into(),
            score: 9.0,
            has_proof: true,
        };
        let v = serde_json::to_value(&r).unwrap();
        for key in [
            "id",
            "name",
            "dept",
            "updateCycle",
            "lagDays",
            "stopped",
            "declaredCols",
            "sampleCols",
            "emptyRate",
            "allEmptyCols",
            "schemaKind",
            "score",
            "hasProof",
        ] {
            assert!(v.get(key).is_some(), "缺字段 {key}，前端会静默拿到默认值");
        }
    }
}
