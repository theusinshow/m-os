//! Orquestracao do cronometro: aplica a regra no repositorio, emite o evento
//! `timer-state-changed` e atualiza a bandeja. E o ponto unico usado tanto
//! pelos comandos Tauri quanto pelas acoes da bandeja, evitando duplicacao.

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_sql::DbInstances;

use crate::database;
use crate::error::AppError;
use crate::models::{ActiveTimer, TimeEntry, ValidStart};
use crate::repository;
use crate::tray;

async fn pool(app: &AppHandle) -> Result<sqlx::Pool<sqlx::Sqlite>, AppError> {
    let db = app.state::<DbInstances>();
    database::pool(&db).await
}

/// Retorna o cronometro ativo, se houver (sem efeitos colaterais).
pub async fn get_active(app: &AppHandle) -> Result<Option<ActiveTimer>, AppError> {
    let pool = pool(app).await?;
    repository::timer::active(&pool).await
}

pub async fn start(app: &AppHandle, input: ValidStart) -> Result<ActiveTimer, AppError> {
    let pool = pool(app).await?;
    let timer = repository::timer::start(&pool, input).await?;
    after_change(app, &pool).await;
    Ok(timer)
}

pub async fn pause(app: &AppHandle) -> Result<ActiveTimer, AppError> {
    let pool = pool(app).await?;
    let timer = repository::timer::pause(&pool).await?;
    after_change(app, &pool).await;
    Ok(timer)
}

pub async fn resume(app: &AppHandle) -> Result<ActiveTimer, AppError> {
    let pool = pool(app).await?;
    let timer = repository::timer::resume(&pool).await?;
    after_change(app, &pool).await;
    Ok(timer)
}

pub async fn stop(app: &AppHandle) -> Result<TimeEntry, AppError> {
    let pool = pool(app).await?;
    let entry = repository::timer::stop(&pool).await?;
    after_change(app, &pool).await;
    Ok(entry)
}

pub async fn discard(app: &AppHandle) -> Result<(), AppError> {
    let pool = pool(app).await?;
    repository::timer::discard(&pool).await?;
    after_change(app, &pool).await;
    Ok(())
}

/// Sincroniza bandeja e frontend com o estado atual do banco. Chamado no
/// startup (recuperacao) e apos cada transicao.
pub async fn refresh(app: &AppHandle) {
    if let Ok(pool) = pool(app).await {
        after_change(app, &pool).await;
    }
}

async fn after_change(app: &AppHandle, pool: &sqlx::Pool<sqlx::Sqlite>) {
    let active = repository::timer::active(pool).await.ok().flatten();

    let project_name = match &active {
        Some(t) => repository::projects::get(pool, &t.project_id)
            .await
            .ok()
            .map(|p| p.name),
        None => None,
    };

    tray::update_state(
        app,
        active
            .as_ref()
            .map(|t| (t.status.clone(), project_name.clone().unwrap_or_default())),
    );

    let payload = json!({
        "active": active.is_some(),
        "projectId": active.as_ref().map(|t| t.project_id.clone()),
        "status": active.as_ref().map(|t| t.status.clone()),
        "activityType": active.as_ref().map(|t| t.activity_type.clone()),
    });
    let _ = app.emit("timer-state-changed", payload);
}
