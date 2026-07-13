//! Repositorio de clientes.

use sqlx::{Pool, Sqlite};

use crate::error::AppError;
use crate::models::{Client, ValidClient};

use super::{new_id, now_iso};

const COLUMNS: &str = "id, name, company_name, email, phone, notes, \
     created_at, updated_at, archived_at";

/// Lista clientes ordenados por nome. Por padrao oculta os arquivados.
pub async fn list(pool: &Pool<Sqlite>, include_archived: bool) -> Result<Vec<Client>, AppError> {
    let sql = format!(
        "SELECT {COLUMNS} FROM clients {} ORDER BY name COLLATE NOCASE",
        if include_archived {
            ""
        } else {
            "WHERE archived_at IS NULL"
        }
    );
    let rows = sqlx::query_as::<_, Client>(&sql).fetch_all(pool).await?;
    Ok(rows)
}

/// Busca um cliente por id.
pub async fn get(pool: &Pool<Sqlite>, id: &str) -> Result<Client, AppError> {
    let client = sqlx::query_as::<_, Client>(&format!("SELECT {COLUMNS} FROM clients WHERE id = ?1"))
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(client)
}

/// Cria um cliente e retorna o registro persistido.
pub async fn create(pool: &Pool<Sqlite>, input: ValidClient) -> Result<Client, AppError> {
    let id = new_id();
    let now = now_iso();
    let client = sqlx::query_as::<_, Client>(&format!(
        "INSERT INTO clients \
         (id, name, company_name, email, phone, notes, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7) \
         RETURNING {COLUMNS}"
    ))
    .bind(&id)
    .bind(&input.name)
    .bind(&input.company_name)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.notes)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(client)
}

/// Atualiza os dados de um cliente existente.
pub async fn update(pool: &Pool<Sqlite>, id: &str, input: ValidClient) -> Result<Client, AppError> {
    let now = now_iso();
    let client = sqlx::query_as::<_, Client>(&format!(
        "UPDATE clients SET \
         name = ?2, company_name = ?3, email = ?4, phone = ?5, notes = ?6, updated_at = ?7 \
         WHERE id = ?1 \
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&input.name)
    .bind(&input.company_name)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.notes)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(client)
}

/// Arquiva um cliente (soft): define `archived_at`. Preserva o historico.
pub async fn archive(pool: &Pool<Sqlite>, id: &str) -> Result<Client, AppError> {
    let now = now_iso();
    let client = sqlx::query_as::<_, Client>(&format!(
        "UPDATE clients SET archived_at = ?2, updated_at = ?2 WHERE id = ?1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&now)
    .fetch_one(pool)
    .await?;
    Ok(client)
}
