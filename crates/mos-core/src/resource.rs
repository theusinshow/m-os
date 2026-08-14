use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{CaptureId, CoreError, ErrorCode, LifecycleState};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceId(Uuid);

impl ResourceId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| CoreError::new(ErrorCode::InvalidInput, "Resource ID invalido.", false))
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ResourceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Link,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Link => "link",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "link" => Ok(Self::Link),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo de Resource desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub title: String,
    pub url: String,
    pub note: String,
    pub source_capture_id: Option<CaptureId>,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
pub struct NewResource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub title: String,
    pub url: String,
    pub note: String,
    pub source_capture_id: Option<CaptureId>,
    pub created_at: OffsetDateTime,
}

impl NewResource {
    pub fn create_link(
        title: &str,
        url: &str,
        note: &str,
        source_capture_id: Option<CaptureId>,
    ) -> Result<Self, CoreError> {
        let url = validate_resource_url(url)?;
        let title = match title.trim() {
            "" => url.clone(),
            value => value.to_owned(),
        };
        Ok(Self {
            id: ResourceId::new(),
            kind: ResourceKind::Link,
            title,
            url,
            note: note.trim().to_owned(),
            source_capture_id,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

pub fn validate_resource_url(url: &str) -> Result<String, CoreError> {
    let url = url.trim();
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(url.to_owned())
    } else {
        Err(CoreError::new(
            ErrorCode::InvalidInput,
            "A URL do Resource deve comecar com http:// ou https://.",
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_requires_http_and_uses_url_as_fallback_title() {
        assert!(NewResource::create_link("", "motion.dev", "", None).is_err());
        let resource =
            NewResource::create_link("", "https://motion.dev", "Animacoes", None).unwrap();
        assert_eq!(resource.title, "https://motion.dev");
        assert_eq!(resource.kind, ResourceKind::Link);
    }
}
