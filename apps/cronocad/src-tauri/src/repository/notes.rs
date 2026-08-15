//! Repositorio de anotacoes e pendencias por projeto.
//!
//! `set_notes` grava o texto livre na coluna `projects.notes` (1-para-1).
//! As pendencias vivem em `project_todos` e usam **hard delete**: nao sao
//! registro de tempo nem geram cobranca, entao nao ha soft delete aqui.

use sqlx::{Pool, Sqlite};

use crate::error::AppError;
use crate::models::{Project, ProjectTodo};

use super::{new_id, now_iso};

const PROJECT_COLUMNS: &str = "id, client_id, name, code, description, hourly_rate_cents, \
     budget_minutes, status, color, created_at, updated_at, archived_at, notes";

const TODO_COLUMNS: &str = "id, project_id, text, done, done_at, created_at, updated_at";

/// Normaliza o texto de uma pendencia; rejeita vazio/em branco.
fn clean_text(text: &str) -> Result<String, AppError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "a pendencia precisa de um texto".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Grava o bloco de anotacoes do projeto. Texto vazio/em branco vira `NULL`.
pub async fn set_notes(
    pool: &Pool<Sqlite>,
    project_id: &str,
    notes: Option<String>,
) -> Result<Project, AppError> {
    let value = notes
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    let now = now_iso();
    let project = sqlx::query_as::<_, Project>(&format!(
        "UPDATE projects SET notes = ?2, updated_at = ?3 WHERE id = ?1 \
         RETURNING {PROJECT_COLUMNS}"
    ))
    .bind(project_id)
    .bind(&value)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(project)
}

/// Lista todas as pendencias (abertas e concluidas). O Painel filtra as abertas.
pub async fn list_todos(pool: &Pool<Sqlite>) -> Result<Vec<ProjectTodo>, AppError> {
    let rows = sqlx::query_as::<_, ProjectTodo>(&format!(
        "SELECT {TODO_COLUMNS} FROM project_todos ORDER BY created_at"
    ))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Cria uma pendencia aberta no projeto.
pub async fn create_todo(
    pool: &Pool<Sqlite>,
    project_id: &str,
    text: &str,
) -> Result<ProjectTodo, AppError> {
    let text = clean_text(text)?;
    let id = new_id();
    let now = now_iso();
    let todo = sqlx::query_as::<_, ProjectTodo>(&format!(
        "INSERT INTO project_todos (id, project_id, text, done, done_at, created_at, updated_at) \
         VALUES (?1, ?2, ?3, 0, NULL, ?4, ?4) \
         RETURNING {TODO_COLUMNS}"
    ))
    .bind(&id)
    .bind(project_id)
    .bind(&text)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(todo)
}

/// Marca/desmarca uma pendencia, mantendo `done_at` coerente.
pub async fn set_todo_done(
    pool: &Pool<Sqlite>,
    id: &str,
    done: bool,
) -> Result<ProjectTodo, AppError> {
    let now = now_iso();
    let todo = sqlx::query_as::<_, ProjectTodo>(&format!(
        "UPDATE project_todos SET \
         done = ?2, \
         done_at = CASE WHEN ?2 = 1 THEN ?3 ELSE NULL END, \
         updated_at = ?3 \
         WHERE id = ?1 \
         RETURNING {TODO_COLUMNS}"
    ))
    .bind(id)
    .bind(done)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(todo)
}

/// Corrige o texto de uma pendencia.
pub async fn update_todo_text(
    pool: &Pool<Sqlite>,
    id: &str,
    text: &str,
) -> Result<ProjectTodo, AppError> {
    let text = clean_text(text)?;
    let now = now_iso();
    let todo = sqlx::query_as::<_, ProjectTodo>(&format!(
        "UPDATE project_todos SET text = ?2, updated_at = ?3 WHERE id = ?1 \
         RETURNING {TODO_COLUMNS}"
    ))
    .bind(id)
    .bind(&text)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(todo)
}

/// Remove a pendencia definitivamente.
pub async fn delete_todo(pool: &Pool<Sqlite>, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM project_todos WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
