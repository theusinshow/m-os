//! Tipo de erro unificado retornado ao frontend.
//!
//! Erros sao tratados explicitamente (secao 20): nada de `unwrap()` em caminhos
//! normais de producao. `AppError` serializa como string legivel para o JS.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("erro de banco de dados: {0}")]
    Database(String),

    #[error("entrada invalida: {0}")]
    Validation(String),

    #[error("registro nao encontrado: {0}")]
    NotFound(String),

    #[error("conflito: {0}")]
    Conflict(String),

    #[error("erro de arquivo: {0}")]
    Io(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("registro inexistente".to_string()),
            other => AppError::Database(other.to_string()),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
