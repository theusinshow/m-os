//! Repositorio do cronometro (active_timer) e transicoes.
//!
//! Regras centrais (secao 9). O estado e persistido imediatamente a cada
//! transicao e a duracao e recalculada a partir de timestamps (via
//! `domain::timer`, testado). Ao encerrar, cria-se um `time_entry` preservando
//! o valor/hora atual do projeto como snapshot, dentro de uma transacao.

use sqlx::{Pool, Sqlite};

use crate::domain::timer::{elapsed_seconds, TimerSnapshot, TimerStatus};
use crate::error::AppError;
use crate::models::{ActiveTimer, TimeEntry, ValidStart};

use super::{epoch_of, new_id, now_iso, record_event, time_entries};

const COLUMNS: &str = "id, project_id, started_at, last_resumed_at, accumulated_seconds, \
     idle_seconds, status, description, activity_type, created_at, updated_at";

/// Retorna o cronometro ativo, se houver.
pub async fn active(pool: &Pool<Sqlite>) -> Result<Option<ActiveTimer>, AppError> {
    let row = sqlx::query_as::<_, ActiveTimer>(&format!(
        "SELECT {COLUMNS} FROM active_timer WHERE singleton = 1"
    ))
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Constroi um snapshot de dominio a partir do estado persistido.
fn snapshot(timer: &ActiveTimer) -> Result<TimerSnapshot, AppError> {
    Ok(TimerSnapshot {
        status: if timer.status == "running" {
            TimerStatus::Running
        } else {
            TimerStatus::Paused
        },
        accumulated_seconds: timer.accumulated_seconds,
        last_resumed_epoch: epoch_of(&timer.last_resumed_at)?,
    })
}

/// Inicia um cronometro. Falha (Conflict) se ja existir um ativo — a decisao de
/// encerrar/pausar o anterior cabe ao usuario (nunca inicia silenciosamente).
pub async fn start(pool: &Pool<Sqlite>, input: ValidStart) -> Result<ActiveTimer, AppError> {
    if active(pool).await?.is_some() {
        return Err(AppError::Conflict(
            "ja existe um cronometro ativo; encerre ou pause antes de iniciar outro".into(),
        ));
    }
    let now = now_iso();
    let timer = sqlx::query_as::<_, ActiveTimer>(&format!(
        "INSERT INTO active_timer \
         (id, singleton, project_id, started_at, last_resumed_at, accumulated_seconds, \
          status, description, activity_type, created_at, updated_at) \
         VALUES (?1, 1, ?2, ?3, ?3, 0, 'running', ?4, ?5, ?3, ?3) \
         RETURNING {COLUMNS}"
    ))
    .bind(new_id())
    .bind(&input.project_id)
    .bind(&now)
    .bind(&input.description)
    .bind(&input.activity_type)
    .fetch_one(pool)
    .await?;
    record_event(pool, "timer_started", None, None).await?;
    Ok(timer)
}

/// Pausa: soma o tempo desde `last_resumed_at` ao acumulado e persiste.
pub async fn pause(pool: &Pool<Sqlite>) -> Result<ActiveTimer, AppError> {
    let timer = active(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("nenhum cronometro ativo".into()))?;
    if timer.status == "paused" {
        return Ok(timer);
    }
    let now_epoch = chrono::Utc::now().timestamp();
    let new_accumulated = elapsed_seconds(&snapshot(&timer)?, now_epoch);
    let now = now_iso();
    let updated = sqlx::query_as::<_, ActiveTimer>(&format!(
        "UPDATE active_timer SET accumulated_seconds = ?1, status = 'paused', updated_at = ?2 \
         WHERE singleton = 1 RETURNING {COLUMNS}"
    ))
    .bind(new_accumulated)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    record_event(pool, "timer_paused", None, None).await?;
    Ok(updated)
}

/// Continua: define novo `last_resumed_at` e volta a `running`.
pub async fn resume(pool: &Pool<Sqlite>) -> Result<ActiveTimer, AppError> {
    let timer = active(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("nenhum cronometro ativo".into()))?;
    if timer.status == "running" {
        return Ok(timer);
    }
    let now = now_iso();
    let updated = sqlx::query_as::<_, ActiveTimer>(&format!(
        "UPDATE active_timer SET last_resumed_at = ?1, status = 'running', updated_at = ?1 \
         WHERE singleton = 1 RETURNING {COLUMNS}"
    ))
    .bind(&now)
    .fetch_one(pool)
    .await?;
    record_event(pool, "timer_resumed", None, None).await?;
    Ok(updated)
}

/// Encerra: calcula a duracao final, cria um `time_entry` com snapshot do
/// valor/hora e remove o `active_timer` — tudo em uma transacao.
pub async fn stop(pool: &Pool<Sqlite>) -> Result<TimeEntry, AppError> {
    let timer = active(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("nenhum cronometro ativo".into()))?;

    let now_epoch = chrono::Utc::now().timestamp();
    let duration = elapsed_seconds(&snapshot(&timer)?, now_epoch);
    let now = now_iso();

    let rate: i64 = sqlx::query_scalar("SELECT hourly_rate_cents FROM projects WHERE id = ?1")
        .bind(&timer.project_id)
        .fetch_one(pool)
        .await?;

    // O tempo inativo descontado durante a sessao segue para a sessao gravada,
    // sem nunca exceder a duracao bruta.
    let idle = timer.idle_seconds.clamp(0, duration);

    let mut tx = pool.begin().await?;
    let entry = sqlx::query_as::<_, TimeEntry>(&format!(
        "INSERT INTO time_entries \
         (id, project_id, started_at, ended_at, duration_seconds, idle_seconds, \
          description, activity_type, billable, hourly_rate_snapshot_cents, source, \
          created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, 'timer', ?4, ?4) \
         RETURNING {}",
        time_entries::COLUMNS
    ))
    .bind(new_id())
    .bind(&timer.project_id)
    .bind(&timer.started_at)
    .bind(&now)
    .bind(duration)
    .bind(idle)
    .bind(&timer.description)
    .bind(&timer.activity_type)
    .bind(rate)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM active_timer WHERE singleton = 1")
        .execute(&mut *tx)
        .await?;
    record_event(&mut *tx, "timer_stopped", None, None).await?;
    tx.commit().await?;

    Ok(entry)
}

/// Acrescenta tempo inativo (segundos) ao cronometro ativo, por decisao do
/// usuario apos um periodo de inatividade (secao 11). Nao altera a duracao bruta;
/// apenas o inativo, aplicado ao gerar a sessao e nos relatorios.
pub async fn add_idle(pool: &Pool<Sqlite>, seconds: i64) -> Result<ActiveTimer, AppError> {
    if seconds < 0 {
        return Err(AppError::Validation(
            "o tempo a descontar nao pode ser negativo".into(),
        ));
    }
    let now = now_iso();
    let updated = sqlx::query_as::<_, ActiveTimer>(&format!(
        "UPDATE active_timer SET idle_seconds = idle_seconds + ?1, updated_at = ?2 \
         WHERE singleton = 1 RETURNING {COLUMNS}"
    ))
    .bind(seconds)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(updated)
}

/// Descarta o cronometro ativo sem criar sessao (usado na recuperacao quando o
/// usuario opta por nao registrar o periodo). Registra o evento com metadado.
pub async fn discard(pool: &Pool<Sqlite>) -> Result<(), AppError> {
    let affected = sqlx::query("DELETE FROM active_timer WHERE singleton = 1")
        .execute(pool)
        .await?
        .rows_affected();
    if affected > 0 {
        record_event(pool, "timer_stopped", None, Some("{\"discarded\":true}")).await?;
    }
    Ok(())
}
