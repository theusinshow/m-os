//! Repositorio de programas monitorados.

use sqlx::{Pool, Sqlite};

use crate::error::AppError;
use crate::models::{MonitoredApp, ValidMonitoredApp};

use super::{new_id, now_iso};

const COLUMNS: &str = "id, display_name, process_name, enabled, remind_on_open, \
     remind_on_close, created_at, updated_at";

/// Lista todos os programas monitorados, ordenados por nome de exibicao.
pub async fn list(pool: &Pool<Sqlite>) -> Result<Vec<MonitoredApp>, AppError> {
    let rows = sqlx::query_as::<_, MonitoredApp>(&format!(
        "SELECT {COLUMNS} FROM monitored_apps ORDER BY display_name COLLATE NOCASE"
    ))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Lista apenas os programas habilitados (usado pelo servico de monitoramento).
pub async fn list_enabled(pool: &Pool<Sqlite>) -> Result<Vec<MonitoredApp>, AppError> {
    let rows = sqlx::query_as::<_, MonitoredApp>(&format!(
        "SELECT {COLUMNS} FROM monitored_apps WHERE enabled = 1"
    ))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Cria um programa monitorado. Falha (Conflict) se o executavel ja existir.
pub async fn create(
    pool: &Pool<Sqlite>,
    input: ValidMonitoredApp,
) -> Result<MonitoredApp, AppError> {
    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM monitored_apps WHERE process_name = ?1")
            .bind(&input.process_name)
            .fetch_optional(pool)
            .await?;
    if exists.is_some() {
        return Err(AppError::Conflict(format!(
            "o executavel '{}' ja esta na lista",
            input.process_name
        )));
    }

    let now = now_iso();
    let app = sqlx::query_as::<_, MonitoredApp>(&format!(
        "INSERT INTO monitored_apps \
         (id, display_name, process_name, enabled, remind_on_open, remind_on_close, \
          created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7) \
         RETURNING {COLUMNS}"
    ))
    .bind(new_id())
    .bind(&input.display_name)
    .bind(&input.process_name)
    .bind(input.enabled)
    .bind(input.remind_on_open)
    .bind(input.remind_on_close)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(app)
}

/// Atualiza um programa monitorado existente.
pub async fn update(
    pool: &Pool<Sqlite>,
    id: &str,
    input: ValidMonitoredApp,
) -> Result<MonitoredApp, AppError> {
    let now = now_iso();
    let app = sqlx::query_as::<_, MonitoredApp>(&format!(
        "UPDATE monitored_apps SET \
         display_name = ?2, process_name = ?3, enabled = ?4, remind_on_open = ?5, \
         remind_on_close = ?6, updated_at = ?7 \
         WHERE id = ?1 \
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&input.display_name)
    .bind(&input.process_name)
    .bind(input.enabled)
    .bind(input.remind_on_open)
    .bind(input.remind_on_close)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(app)
}

/// Remove um programa monitorado.
pub async fn delete(pool: &Pool<Sqlite>, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM monitored_apps WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
