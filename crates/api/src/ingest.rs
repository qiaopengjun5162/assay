//! 把一次跑批的产物灌进查询库。
//!
//! **这不是 HTTP 写接口**，是运维/CI 用的 CLI 操作。服务本身仍然只读（见 `lib.rs`
//! 与 D20）——「谁能写库」和「谁能查询」是两件事，前者只需要一个人，后者才需要
//! 一个服务。
//!
//! 只写索引行（分数、滞后、空值率、分类…），**不写证据**。原始响应、SHA-256、
//! Lean 证明仍然是静态对象，库丢了照样能离线复核。
//!
//! 字段从 `assay_core::export::index_rows` 取——与静态 `index.json` 同一个来源，
//! 所以接口和静态站不会对同一条数据给出不同分数。

use assay_core::export;
use serde_json::Value;
use sqlx::PgPool;
use std::collections::HashSet;
use std::path::Path;

/// 浙江省平台的固定主键，与 migrations 里的 insert 对应。
const PLATFORM_ZJ: i16 = 1;

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("数据库错误：{0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    Core(#[from] assay_core::Error),
    #[error("{0}")]
    Bad(String),
}

pub struct IngestOptions<'a> {
    /// 跑批产物目录，如 `out-full`（需要 items.jsonl / summary.json，
    /// 有 catalog_snapshot.json / proofs.json 时会一并纳入）
    pub out_dir: &'a str,
    /// 体检基准日 `YYYY-MM-DD`。**必须显式给**，不用 `now()`：
    /// 同一次跑批今天灌和明天灌必须落到同一行（`run` 上是 unique 约束）。
    pub captured_at: &'a str,
    pub commit: Option<&'a str>,
}

#[derive(Debug)]
pub struct IngestOutcome {
    pub run_id: i64,
    pub datasets: usize,
    pub observations: usize,
    /// 被跳过的重复 id 与空 id 数量。真实数据目前是 0，但平台是公共接口，
    /// 哪天返回重复 id，这里必须先挡住——见下面 `dedup` 的注释。
    pub dropped: usize,
}

/// 一条 run 的元信息，来自 `summary.json`。整份也存进 `raw_summary`，
/// 将来新增指标不用改 schema。
struct RunMeta {
    audited: i32,
    with_preview: i32,
    raw: Value,
}

pub async fn ingest(pool: &PgPool, opts: &IngestOptions<'_>) -> Result<IngestOutcome, IngestError> {
    validate_date(opts.captured_at)?;

    let dir = Path::new(opts.out_dir);
    let items = dir.join("items.jsonl");
    if !items.is_file() {
        return Err(IngestError::Bad(format!(
            "找不到 {}，先跑一次 assay audit 或指定 --out",
            items.display()
        )));
    }

    let catalog = dir.join("catalog_snapshot.json");
    let proofs = dir.join("proofs.json");
    let str_of = |p: &Path| p.to_string_lossy().to_string();

    let rows = export::index_rows(
        &str_of(&items),
        catalog.is_file().then(|| str_of(&catalog)).as_deref(),
        proofs.is_file().then(|| str_of(&proofs)).as_deref(),
    )?;
    let total = rows.len();
    let rows = dedup(rows);
    let dropped = total - rows.len();
    if dropped > 0 {
        tracing::warn!(dropped, kept = rows.len(), "丢弃了重复或空 id 的行");
    }

    let meta = read_run_meta(&dir.join("summary.json"))?;
    if meta.audited as usize != total {
        // 不是致命错误（limit 跑批时 items 会少于 audited），但值得说一句：
        // 「跑批说体检了 N 条，却只灌进去 M 条」是最容易写错的地方。
        tracing::warn!(
            audited = meta.audited,
            ingested = total,
            "summary 里的 audited 与实际灌入行数不一致（limit 跑批属正常）"
        );
    }

    let payload =
        serde_json::to_string(&rows).map_err(|e| IngestError::Bad(format!("序列化失败：{e}")))?;

    let mut tx = pool.begin().await?;
    let run_id: i64 = sqlx::query_scalar(RUN_UPSERT)
        .bind(PLATFORM_ZJ)
        .bind(opts.captured_at)
        .bind(opts.commit)
        .bind(meta.audited)
        .bind(meta.with_preview)
        .bind(&meta.raw)
        .fetch_one(&mut *tx)
        .await?;

    let affected = sqlx::query(DATASET_AND_OBSERVATION_UPSERT)
        .bind(PLATFORM_ZJ)
        .bind(&payload)
        .bind(run_id)
        .execute(&mut *tx)
        .await?
        .rows_affected() as usize;

    tx.commit().await?;
    Ok(IngestOutcome {
        run_id,
        datasets: rows.len(),
        observations: affected,
        dropped,
    })
}

/// 去掉重复 id 与空 id。
///
/// 不是为了省空间，是为了不让整批失败：`ON CONFLICT DO UPDATE` 在**同一条命令**里
/// 碰到同一行两次会直接报 `cannot affect row a second time`，一条脏数据能让 2 万条
/// 全灌不进去。所以脏数据要在这里被挡住并计数，而不是让 Postgres 兜。
fn dedup(rows: Vec<Value>) -> Vec<Value> {
    let mut seen: HashSet<String> = HashSet::with_capacity(rows.len());
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let id = r["id"].as_str().unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        if seen.insert(id.to_string()) {
            out.push(r);
        }
    }
    out
}

/// 只做形状校验，不做日历合法性：`run.captured_at` 上有 check 约束，
/// 真写错会被数据库拒掉。这里提前拦一道是为了给出「你传的是 2026/11/18」
/// 这种能直接改的报错，而不是一个 SQLSTATE。
fn validate_date(s: &str) -> Result<(), IngestError> {
    let bytes = s.as_bytes();
    let ok = bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
    if !ok {
        return Err(IngestError::Bad(format!(
            "--captured-at 要 YYYY-MM-DD（零填充），收到：{s}"
        )));
    }
    Ok(())
}

fn read_run_meta(summary_path: &Path) -> Result<RunMeta, IngestError> {
    let raw: Value = serde_json::from_slice(
        &std::fs::read(summary_path)
            .map_err(|e| IngestError::Bad(format!("读不到 {}：{e}", summary_path.display())))?,
    )
    .map_err(|e| IngestError::Bad(format!("{} 不是合法 JSON：{e}", summary_path.display())))?;
    let pick = |k: &str| -> Result<i32, IngestError> {
        raw[k]
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| IngestError::Bad(format!("summary.json 缺整数字段 {k}")))
    };
    Ok(RunMeta {
        audited: pick("audited")?,
        with_preview: pick("withPreview")?,
        raw,
    })
}

/// 同一天重跑要覆盖，不能插两条。`raw_summary` 整份替换：口径变了也该跟着变。
const RUN_UPSERT: &str = "
insert into run (platform_id, captured_at, commit_sha, audited, with_preview, raw_summary)
values ($1, $2, $3, $4, $5, $6)
on conflict (platform_id, captured_at) do update
   set commit_sha   = excluded.commit_sha,
       audited      = excluded.audited,
       with_preview = excluded.with_preview,
       raw_summary  = excluded.raw_summary
returning id
";

/// dataset 与 observation 一条语句写完。
///
/// 三个刻意的点：
///
/// 1. **整个批次一次传 JSON**，不逐条往返。2.4 万条 × 2 次 = 4.8 万次往返在本地
///    都要几十秒，跨网络更是没法看。`jsonb_to_recordset` 让一次 SQL 处理整批。
/// 2. **recordset 的列名直接是 JSON 键名**（`"lagDays"` 这类要加引号）。这样
///    Rust 侧不用把 camelCase 再映射一遍——少一层映射就少一处会漂移的地方。
/// 3. **CTE 里 upsert dataset 再 join 回 observation**，避免在 Rust 侧维护
///    `ext_id → 自增 id` 的字典（那需要在内存里放 2.4 万个键，且要处理并发插入）。
const DATASET_AND_OBSERVATION_UPSERT: &str = r#"
with up as (
    insert into dataset (platform_id, ext_id, name, dept)
    select $1, u.id, u.name, coalesce(u.dept, '')
      from jsonb_to_recordset($2::jsonb) as u(id text, name text, dept text)
    on conflict (platform_id, ext_id) do update
       set name = excluded.name,
           dept = excluded.dept
    returning id, ext_id
)
insert into observation (
    run_id, dataset_id, score, lag_days, update_cycle, empty_rate,
    declared_cols, sample_cols, all_empty_cols, schema_kind, stopped, has_proof
)
select $3, up.id, u.score, u."lagDays", coalesce(u."updateCycle", ''),
       u."emptyRate", u."declaredCols", u."sampleCols", u."allEmptyCols",
       u."schemaKind", u.stopped, u."hasProof"
  from jsonb_to_recordset($2::jsonb) as u(
         id text, name text, dept text, "updateCycle" text,
         "lagDays" int, "emptyRate" real, "declaredCols" int,
         "sampleCols" int, "allEmptyCols" int, "schemaKind" text,
         stopped bool, "hasProof" bool, score real)
  join up on up.ext_id = u.id
on conflict (run_id, dataset_id) do update
   set score          = excluded.score,
       lag_days       = excluded.lag_days,
       update_cycle   = excluded.update_cycle,
       empty_rate     = excluded.empty_rate,
       declared_cols  = excluded.declared_cols,
       sample_cols    = excluded.sample_cols,
       all_empty_cols = excluded.all_empty_cols,
       schema_kind    = excluded.schema_kind,
       stopped        = excluded.stopped,
       has_proof      = excluded.has_proof
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_dates_pass_and_obvious_typos_fail() {
        assert!(validate_date("2026-11-18").is_ok());
        for bad in ["2026/11/18", "2026-1-8", "20261118", "2026-11-18 ", ""] {
            assert!(validate_date(bad).is_err(), "本该拒绝：{bad:?}");
        }
    }

    #[test]
    fn dedup_keeps_first_occurrence_and_drops_empty_ids() {
        let rows = vec![
            json!({"id": "a", "score": 1.0}),
            json!({"id": "b", "score": 2.0}),
            json!({"id": "a", "score": 9.0}),
            json!({"id": "", "score": 3.0}),
        ];
        let out = dedup(rows);
        assert_eq!(out.len(), 2, "重复与空 id 都要被丢掉");
        assert_eq!(out[0]["score"], 1.0, "保留首次出现的那条");
        assert_eq!(out[1]["id"], "b");
    }
}
