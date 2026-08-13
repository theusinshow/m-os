use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{CoreError, ErrorCode, LifecycleState};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppId(Uuid);

impl AppId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| CoreError::new(ErrorCode::InvalidInput, "App ID invalido.", false))
    }
}

impl Default for AppId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppLaunchKind {
    Url,
    Path,
}

impl AppLaunchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Url => "url",
            Self::Path => "path",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "url" => Ok(Self::Url),
            "path" => Ok(Self::Path),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo de abertura de App desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredApp {
    pub id: AppId,
    pub name: String,
    pub description: String,
    pub launch_kind: Option<AppLaunchKind>,
    pub launch_target: Option<String>,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_opened_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug)]
pub struct NewRegisteredApp {
    pub id: AppId,
    pub name: String,
    pub description: String,
    pub launch_kind: Option<AppLaunchKind>,
    pub launch_target: Option<String>,
    pub created_at: OffsetDateTime,
}

impl NewRegisteredApp {
    pub fn create(
        name: &str,
        description: &str,
        launch_kind: Option<AppLaunchKind>,
        launch_target: Option<&str>,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            id: AppId::new(),
            name: required(name, "O nome do App nao pode estar vazio.")?,
            description: description.trim().to_owned(),
            launch_kind,
            launch_target: validate_launch_target(launch_kind, launch_target)?,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

pub fn validate_launch_target(
    launch_kind: Option<AppLaunchKind>,
    launch_target: Option<&str>,
) -> Result<Option<String>, CoreError> {
    let launch_target = launch_target
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (launch_kind, launch_target) {
        (None, None) => Ok(None),
        (Some(AppLaunchKind::Url), Some(target)) => {
            if target.starts_with("https://") || target.starts_with("http://") {
                Ok(Some(target.to_owned()))
            } else {
                Err(CoreError::new(
                    ErrorCode::InvalidInput,
                    "URL de App deve comecar com http:// ou https://.",
                    false,
                ))
            }
        }
        (Some(AppLaunchKind::Path), Some(target)) => Ok(Some(target.to_owned())),
        (Some(_), None) => Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Informe o alvo de abertura do App.",
            false,
        )),
        (None, Some(_)) => Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Informe o tipo do alvo de abertura do App.",
            false,
        )),
    }
}

fn required(value: &str, message: &str) -> Result<String, CoreError> {
    let value = value.trim();
    if value.is_empty() {
        Err(CoreError::new(ErrorCode::InvalidInput, message, false))
    } else {
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_requires_name() {
        assert!(NewRegisteredApp::create(" ", "", None, None).is_err());
    }

    #[test]
    fn url_target_requires_http_scheme() {
        assert!(
            NewRegisteredApp::create("Figma", "", Some(AppLaunchKind::Url), Some("figma.com"))
                .is_err()
        );
        assert!(NewRegisteredApp::create(
            "Figma",
            "",
            Some(AppLaunchKind::Url),
            Some("https://figma.com"),
        )
        .is_ok());
    }
}
