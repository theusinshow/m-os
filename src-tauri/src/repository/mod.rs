//! Camada de repositorio: consultas `sqlx` especificas e tipadas.
//!
//! Todas as funcoes recebem `&Pool<Sqlite>`, o que permite testa-las contra um
//! banco em memoria (ver testes de persistencia). Nenhuma consulta e montada
//! por concatenacao insegura: valores sempre via `bind` (secao 19/20).

pub mod activity_events;
pub mod clients;
pub mod monitored_apps;
pub mod projects;
pub mod settings;
pub mod time_entries;
pub mod timer;

#[cfg(test)]
mod tests;

use chrono::SecondsFormat;
use sqlx::{Executor, Sqlite};

use crate::error::AppError;

/// Gera um novo id UUID v4 em texto.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Timestamp atual em ISO 8601 UTC com sufixo "Z" (ex.: 2026-07-11T13:34:00Z).
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Converte um timestamp ISO 8601 em epoch (segundos). Base para calcular
/// duracoes no backend (secao 9), independente de fuso.
pub fn epoch_of(iso: &str) -> Result<i64, AppError> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|dt| dt.timestamp())
        .map_err(|e| AppError::Database(format!("timestamp invalido '{iso}': {e}")))
}

/// Registra um evento de atividade (secao 8). Aceita pool ou transacao.
pub async fn record_event<'e, E>(
    executor: E,
    event_type: &str,
    process_name: Option<&str>,
    metadata_json: Option<&str>,
) -> Result<(), AppError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let now = now_iso();
    sqlx::query(
        "INSERT INTO activity_events \
         (id, event_type, process_name, detected_at, metadata_json, processed, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?4)",
    )
    .bind(new_id())
    .bind(event_type)
    .bind(process_name)
    .bind(now)
    .bind(metadata_json)
    .execute(executor)
    .await?;
    Ok(())
}
