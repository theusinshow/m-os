//! Repositorio de configuracoes (tabela de linha unica, id = 1).

use sqlx::{Pool, Sqlite};

use crate::error::AppError;
use crate::models::Settings;

const COLUMNS: &str = "idle_detection_enabled, idle_threshold_minutes, \
     process_monitoring_enabled, process_check_interval_seconds, \
     remind_when_monitored_app_opens, remind_when_monitored_app_closes, \
     rounding_enabled, rounding_interval_minutes, rounding_mode, \
     start_with_windows, minimize_to_tray, close_to_tray, currency, locale, \
     issuer_name, issuer_document, issuer_contact";

/// Le a linha unica de configuracoes.
pub async fn get(pool: &Pool<Sqlite>) -> Result<Settings, AppError> {
    let settings = sqlx::query_as::<_, Settings>(&format!(
        "SELECT {COLUMNS} FROM settings WHERE id = 1"
    ))
    .fetch_one(pool)
    .await?;
    Ok(settings)
}

/// Atualiza todas as configuracoes (linha unica) e retorna o estado salvo.
pub async fn update(pool: &Pool<Sqlite>, s: Settings) -> Result<Settings, AppError> {
    sqlx::query(
        "UPDATE settings SET \
         idle_detection_enabled = ?1, idle_threshold_minutes = ?2, \
         process_monitoring_enabled = ?3, process_check_interval_seconds = ?4, \
         remind_when_monitored_app_opens = ?5, remind_when_monitored_app_closes = ?6, \
         rounding_enabled = ?7, rounding_interval_minutes = ?8, rounding_mode = ?9, \
         start_with_windows = ?10, minimize_to_tray = ?11, close_to_tray = ?12, \
         currency = ?13, locale = ?14, \
         issuer_name = ?15, issuer_document = ?16, issuer_contact = ?17 \
         WHERE id = 1",
    )
    .bind(s.idle_detection_enabled)
    .bind(s.idle_threshold_minutes)
    .bind(s.process_monitoring_enabled)
    .bind(s.process_check_interval_seconds)
    .bind(s.remind_when_monitored_app_opens)
    .bind(s.remind_when_monitored_app_closes)
    .bind(s.rounding_enabled)
    .bind(s.rounding_interval_minutes)
    .bind(&s.rounding_mode)
    .bind(s.start_with_windows)
    .bind(s.minimize_to_tray)
    .bind(s.close_to_tray)
    .bind(&s.currency)
    .bind(&s.locale)
    .bind(&s.issuer_name)
    .bind(&s.issuer_document)
    .bind(&s.issuer_contact)
    .execute(pool)
    .await?;
    get(pool).await
}
