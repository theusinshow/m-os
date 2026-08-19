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

/// Tipo do Resource.
///
/// A Library filtra por estes cinco. A validacao de URL varia por tipo, e o
/// banco espelha exatamente estas regras numa CHECK (migrations 0007 e 0023).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Site,
    Library,
    Image,
    Note,
    /// Arquivo preservado dentro do M/OS.
    ///
    /// Nao tem `url`: o caminho do original mora na linha de ingestao que o
    /// criou, endereçado pelo hash do conteudo. Guardar o caminho aqui tambem
    /// criaria duas verdades sobre onde o arquivo esta, e um dia elas
    /// discordariam.
    File,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::Library => "library",
            Self::Image => "image",
            Self::Note => "note",
            Self::File => "file",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "site" => Ok(Self::Site),
            "library" => Ok(Self::Library),
            "image" => Ok(Self::Image),
            "note" => Ok(Self::Note),
            "file" => Ok(Self::File),
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

/// Um par significa: este Resource pertence a este contexto.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceWorkspace {
    pub resource_id: ResourceId,
    pub workspace_id: crate::WorkspaceId,
}

/// Um par significa: este Resource serve a este Project.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProject {
    pub resource_id: ResourceId,
    pub project_id: crate::ProjectId,
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
    /// Cria um Resource de qualquer tipo.
    ///
    /// A validacao de URL varia por tipo: Site e Library exigem http(s);
    /// Image aceita http(s) ou caminho local; Note e File nao tem URL — e por
    /// isso exigem titulo proprio, ja que nao ha URL para servir de fallback.
    pub fn create(
        kind: ResourceKind,
        title: &str,
        url: &str,
        note: &str,
        source_capture_id: Option<CaptureId>,
    ) -> Result<Self, CoreError> {
        let url = match kind {
            ResourceKind::Site | ResourceKind::Library => validate_resource_url(url)?,
            ResourceKind::Image => validate_image_location(url)?,
            ResourceKind::Note | ResourceKind::File => String::new(),
        };

        let title = match title.trim() {
            "" if url.is_empty() => {
                return Err(CoreError::new(
                    ErrorCode::InvalidInput,
                    "Um Resource sem URL precisa de titulo.",
                    false,
                ))
            }
            "" => url.clone(),
            value => value.to_owned(),
        };

        Ok(Self {
            id: ResourceId::new(),
            kind,
            title,
            url,
            note: note.trim().to_owned(),
            source_capture_id,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    /// Atalho historico: um link e um Site.
    pub fn create_link(
        title: &str,
        url: &str,
        note: &str,
        source_capture_id: Option<CaptureId>,
    ) -> Result<Self, CoreError> {
        Self::create(ResourceKind::Site, title, url, note, source_capture_id)
    }
}

fn validate_image_location(value: &str) -> Result<String, CoreError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Uma imagem precisa de um endereco ou caminho.",
            false,
        ));
    }
    Ok(value.to_owned())
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
    fn kinds_round_trip() {
        for kind in [
            ResourceKind::Site,
            ResourceKind::Library,
            ResourceKind::Image,
            ResourceKind::Note,
            ResourceKind::File,
        ] {
            assert_eq!(ResourceKind::parse(kind.as_str()).unwrap(), kind);
        }
        assert!(ResourceKind::parse("link").is_err());
    }

    /// O caminho do arquivo NAO mora no Resource: quem guarda e a ingestao.
    #[test]
    fn file_nao_tem_url_e_exige_titulo() {
        let resource =
            NewResource::create(ResourceKind::File, "memorial.pdf", "C:/x.pdf", "", None).unwrap();
        assert_eq!(resource.url, "");
        assert_eq!(resource.title, "memorial.pdf");
        assert!(NewResource::create(ResourceKind::File, " ", "", "", None).is_err());
    }

    #[test]
    fn site_and_library_require_http_url() {
        assert!(NewResource::create(ResourceKind::Site, "", "motion.dev", "", None).is_err());
        assert!(NewResource::create(ResourceKind::Library, "", "", "", None).is_err());
    }

    #[test]
    fn note_has_no_url_and_requires_a_title() {
        let resource =
            NewResource::create(ResourceKind::Note, "Ideia", "", "o motivo", None).unwrap();
        assert_eq!(resource.url, "");
        assert_eq!(resource.note, "o motivo");
        assert!(NewResource::create(ResourceKind::Note, "  ", "", "corpo", None).is_err());
    }

    #[test]
    fn image_accepts_local_path_or_url() {
        assert!(NewResource::create(
            ResourceKind::Image,
            "captura",
            "C:/imagens/hero.png",
            "",
            None
        )
        .is_ok());
        assert!(NewResource::create(
            ResourceKind::Image,
            "captura",
            "https://x.dev/a.png",
            "",
            None
        )
        .is_ok());
        assert!(NewResource::create(ResourceKind::Image, "captura", "", "", None).is_err());
    }

    #[test]
    fn link_requires_http_and_uses_url_as_fallback_title() {
        assert!(NewResource::create_link("", "motion.dev", "", None).is_err());
        let resource =
            NewResource::create_link("", "https://motion.dev", "Animacoes", None).unwrap();
        assert_eq!(resource.title, "https://motion.dev");
        assert_eq!(resource.kind, ResourceKind::Site);
    }
}
