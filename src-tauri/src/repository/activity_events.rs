//! Repositorio de eventos de atividade (para a reconstrucao do dia — secao 14).

use sqlx::{Pool, Sqlite};

use crate::error::AppError;
use crate::models::ActivityEvent;

const COLUMNS: &str = "id, event_type, process_name, detected_at, metadata_json, \
     processed, created_at";

/// Lista os eventos detectados no intervalo [from, to), do mais antigo ao mais
/// recente (ordem cronologica da linha do tempo).
pub async fn list_between(
    pool: &Pool<Sqlite>,
    from: &str,
    to: &str,
) -> Result<Vec<ActivityEvent>, AppError> {
    let rows = sqlx::query_as::<_, ActivityEvent>(&format!(
        "SELECT {COLUMNS} FROM activity_events \
         WHERE detected_at >= ?1 AND detected_at < ?2 ORDER BY detected_at ASC"
    ))
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
