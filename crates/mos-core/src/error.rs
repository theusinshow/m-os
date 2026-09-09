use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    NotFound,
    InvalidTransition,
    StorageBusy,
    StorageUnavailable,
    DataIntegrity,
    BackupInvalid,
    UnsupportedBackup,
    /// A sessao de um provedor externo caiu.
    ///
    /// Codigo proprio, e nao `StorageUnavailable`: a tela precisa distinguir
    /// "reconecte o Univirtus" de "algo deu errado". Tratar 401 como erro
    /// generico faria o sync parecer bug e esconderia a unica acao que resolve.
    ProviderUnauthorized,
    /// O lugar de destino ja esta ocupado.
    ///
    /// Codigo proprio, e nao `DataIntegrity`, porque a diferenca decide se vale
    /// TENTAR DE NOVO. Integridade quebrada e um defeito a investigar; um
    /// destino ocupado e um fato que nao muda com o tempo — nada que chegue
    /// depois libera a vaga.
    ///
    /// Quem mais usa isso e a varredura de reparo do sync: ela existe para
    /// materializar o que chegou fora de ordem, e sem este codigo ela tratava
    /// "o pai ainda nao veio" (tentar de novo amanha) e "ja existe uma linha
    /// equivalente aqui" (nunca vai dar certo) como a mesma coisa — e ficava
    /// retentando para sempre.
    Conflict,
    Io,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct CoreError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl CoreError {
    pub fn new(code: ErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }
}
