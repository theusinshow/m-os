//! Repositorio de sessoes de trabalho (time_entries).

use sqlx::{Pool, Sqlite};

use crate::error::AppError;
use crate::models::{BillingRow, TimeEntry, ValidEntryUpdate, ValidManualEntry};

use super::{new_id, now_iso};

pub const COLUMNS: &str = "id, project_id, started_at, ended_at, duration_seconds, \
     idle_seconds, description, activity_type, billable, hourly_rate_snapshot_cents, \
     source, created_at, updated_at, deleted_at";

/// Lista as sessoes mais recentes, da mais nova para a antiga. Por padrao
/// oculta as excluidas (soft delete); `include_deleted` as inclui (para
/// restauracao no historico).
///
/// `limit = None` traz o historico inteiro. Telas que somam valores (Painel e
/// Relatorio) precisam disso: com um teto, o total exibido fica silenciosamente
/// menor que a realidade assim que o historico o ultrapassa.
pub async fn list_recent(
    pool: &Pool<Sqlite>,
    limit: Option<i64>,
    include_deleted: bool,
) -> Result<Vec<TimeEntry>, AppError> {
    let filter = if include_deleted {
        ""
    } else {
        "WHERE deleted_at IS NULL"
    };
    // No SQLite, LIMIT negativo equivale a "sem limite".
    let effective_limit = limit.filter(|n| *n >= 0).unwrap_or(-1);
    let rows = sqlx::query_as::<_, TimeEntry>(&format!(
        "SELECT {COLUMNS} FROM time_entries {filter} ORDER BY started_at DESC LIMIT ?1"
    ))
    .bind(effective_limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Todas as sessoes nao excluidas, reduzidas aos campos usados no calculo de
/// cobranca. Base do total acumulado por projeto: le o historico inteiro, sem
/// o teto de paginacao das listagens de tela.
pub async fn billing_rows(pool: &Pool<Sqlite>) -> Result<Vec<BillingRow>, AppError> {
    let rows = sqlx::query_as::<_, BillingRow>(
        "SELECT project_id, duration_seconds, idle_seconds, billable, \
                hourly_rate_snapshot_cents \
         FROM time_entries WHERE deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Cria uma sessao manual (`source = manual`), preservando o valor/hora atual do
/// projeto como snapshot.
pub async fn create_manual(
    pool: &Pool<Sqlite>,
    input: ValidManualEntry,
) -> Result<TimeEntry, AppError> {
    let rate: i64 = sqlx::query_scalar("SELECT hourly_rate_cents FROM projects WHERE id = ?1")
        .bind(&input.project_id)
        .fetch_one(pool)
        .await?;
    let now = now_iso();
    let entry = sqlx::query_as::<_, TimeEntry>(&format!(
        "INSERT INTO time_entries \
         (id, project_id, started_at, ended_at, duration_seconds, idle_seconds, \
          description, activity_type, billable, hourly_rate_snapshot_cents, source, \
          created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12) \
         RETURNING {COLUMNS}"
    ))
    .bind(new_id())
    .bind(&input.project_id)
    .bind(&input.started_at)
    .bind(&input.ended_at)
    .bind(input.duration_seconds)
    .bind(input.idle_seconds)
    .bind(&input.description)
    .bind(&input.activity_type)
    .bind(input.billable)
    .bind(rate)
    .bind(&input.source)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(entry)
}

/// Edita uma sessao existente (recalcula a duracao a partir de inicio/fim).
/// Preserva `source` e o snapshot do valor/hora.
pub async fn update(
    pool: &Pool<Sqlite>,
    id: &str,
    input: ValidEntryUpdate,
) -> Result<TimeEntry, AppError> {
    let now = now_iso();
    let entry = sqlx::query_as::<_, TimeEntry>(&format!(
        "UPDATE time_entries SET \
         started_at = ?2, ended_at = ?3, duration_seconds = ?4, idle_seconds = ?5, \
         description = ?6, activity_type = ?7, billable = ?8, updated_at = ?9 \
         WHERE id = ?1 AND deleted_at IS NULL \
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&input.started_at)
    .bind(&input.ended_at)
    .bind(input.duration_seconds)
    .bind(input.idle_seconds)
    .bind(&input.description)
    .bind(&input.activity_type)
    .bind(input.billable)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(entry)
}

/// Exclusao reversivel (soft delete): marca `deleted_at`.
pub async fn soft_delete(pool: &Pool<Sqlite>, id: &str) -> Result<(), AppError> {
    let now = now_iso();
    sqlx::query("UPDATE time_entries SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1")
        .bind(id)
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(())
}

/// Restaura uma sessao excluida.
pub async fn restore(pool: &Pool<Sqlite>, id: &str) -> Result<TimeEntry, AppError> {
    let now = now_iso();
    let entry = sqlx::query_as::<_, TimeEntry>(&format!(
        "UPDATE time_entries SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1 \
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(entry)
}
