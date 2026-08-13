use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{CoreError, ErrorCode};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CaptureId(Uuid);

impl CaptureId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| CoreError::new(ErrorCode::InvalidInput, "Capture ID invalido.", false))
    }
}

impl Default for CaptureId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CaptureId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureSource {
    Home,
    QuickCapture,
}

impl CaptureSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::QuickCapture => "quick_capture",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "home" => Ok(Self::Home),
            "quick_capture" => Ok(Self::QuickCapture),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Origem de Capture desconhecida.",
                false,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingState {
    Inbox,
    Processed,
}

impl ProcessingState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Processed => "processed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "inbox" => Ok(Self::Inbox),
            "processed" => Ok(Self::Processed),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de processamento desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Active,
    Archived,
    Trashed,
}

impl LifecycleState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Trashed => "trashed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            "trashed" => Ok(Self::Trashed),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de lifecycle desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub id: CaptureId,
    pub content: String,
    pub source: CaptureSource,
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub processing_state: ProcessingState,
    pub lifecycle_state: LifecycleState,
}

#[derive(Clone, Debug)]
pub struct NewCapture {
    pub id: CaptureId,
    pub content: String,
    pub source: CaptureSource,
    pub captured_at: OffsetDateTime,
}

impl NewCapture {
    pub fn create(content: &str, source: CaptureSource) -> Result<Self, CoreError> {
        let content = content.trim();
        if content.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "A captura nao pode estar vazia.",
                false,
            ));
        }

        Ok(Self {
            id: CaptureId::new(),
            content: content.to_owned(),
            source,
            captured_at: OffsetDateTime::now_utc(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_requires_content() {
        let error = NewCapture::create("  ", CaptureSource::Home).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn capture_starts_with_global_uuid_v7() {
        let capture = NewCapture::create("Uma ideia", CaptureSource::Home).unwrap();
        assert_eq!(capture.id.0.get_version_num(), 7);
    }
}
