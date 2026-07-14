//! Repositorio de projetos.

use sqlx::{Pool, Sqlite};

use crate::error::AppError;
use crate::models::{Project, ValidProject};

use super::{new_id, now_iso};

const COLUMNS: &str = "id, client_id, name, code, description, hourly_rate_cents, \
     budget_minutes, status, color, created_at, updated_at, archived_at, notes";

/// Lista projetos ordenados por nome. Por padrao oculta os arquivados.
pub async fn list(pool: &Pool<Sqlite>, include_archived: bool) -> Result<Vec<Project>, AppError> {
    let sql = format!(
        "SELECT {COLUMNS} FROM projects {} ORDER BY name COLLATE NOCASE",
        if include_archived {
            ""
        } else {
            "WHERE status != 'archived'"
        }
    );
    let rows = sqlx::query_as::<_, Project>(&sql).fetch_all(pool).await?;
    Ok(rows)
}

/// Busca um projeto por id.
pub async fn get(pool: &Pool<Sqlite>, id: &str) -> Result<Project, AppError> {
    let project =
        sqlx::query_as::<_, Project>(&format!("SELECT {COLUMNS} FROM projects WHERE id = ?1"))
            .bind(id)
            .fetch_one(pool)
            .await?;
    Ok(project)
}

/// Cria um projeto (status inicial `active`) e retorna o registro persistido.
pub async fn create(pool: &Pool<Sqlite>, input: ValidProject) -> Result<Project, AppError> {
    let id = new_id();
    let now = now_iso();
    let project = sqlx::query_as::<_, Project>(&format!(
        "INSERT INTO projects \
         (id, client_id, name, code, description, hourly_rate_cents, budget_minutes, \
          status, color, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?9, ?9) \
         RETURNING {COLUMNS}"
    ))
    .bind(&id)
    .bind(&input.client_id)
    .bind(&input.name)
    .bind(&input.code)
    .bind(&input.description)
    .bind(input.hourly_rate_cents)
    .bind(input.budget_minutes)
    .bind(&input.color)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(project)
}

/// Atualiza os dados de um projeto (nao altera o status).
pub async fn update(
    pool: &Pool<Sqlite>,
    id: &str,
    input: ValidProject,
) -> Result<Project, AppError> {
    let now = now_iso();
    let project = sqlx::query_as::<_, Project>(&format!(
        "UPDATE projects SET \
         client_id = ?2, name = ?3, code = ?4, description = ?5, hourly_rate_cents = ?6, \
         budget_minutes = ?7, color = ?8, updated_at = ?9 \
         WHERE id = ?1 \
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&input.client_id)
    .bind(&input.name)
    .bind(&input.code)
    .bind(&input.description)
    .bind(input.hourly_rate_cents)
    .bind(input.budget_minutes)
    .bind(&input.color)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(project)
}

/// Altera o status do projeto (validado previamente). Ajusta `archived_at`
/// coerentemente ao entrar/sair do status `archived`.
pub async fn set_status(pool: &Pool<Sqlite>, id: &str, status: &str) -> Result<Project, AppError> {
    let now = now_iso();
    let project = sqlx::query_as::<_, Project>(&format!(
        "UPDATE projects SET \
         status = ?2, \
         archived_at = CASE WHEN ?2 = 'archived' THEN ?3 ELSE NULL END, \
         updated_at = ?3 \
         WHERE id = ?1 \
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(status)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(project)
}
