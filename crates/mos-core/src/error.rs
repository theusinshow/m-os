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
