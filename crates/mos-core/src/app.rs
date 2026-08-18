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

/// Capacidades declaradas de um App.
///
/// Agrupadas de proposito: quatro `bool` soltos numa assinatura sao quatro
/// oportunidades de trocar dois de lugar e o compilador nao reclamar.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCapabilities {
    pub can_open: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub can_automate: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredApp {
    pub id: AppId,
    pub name: String,
    pub description: String,
    pub source_url: Option<String>,
    pub launch_kind: Option<AppLaunchKind>,
    pub launch_target: Option<String>,
    /// Capacidades declaradas. Capacidade nao declarada e capacidade que o
    /// Hermes nao tenta usar — sao contrato, nao rotulo.
    pub can_open: bool,
    pub can_read: bool,
    pub can_write: bool,
    pub can_automate: bool,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_opened_at: Option<OffsetDateTime>,
}

/// Extrai o host de uma URL sem depender de um parser completo: aqui so
/// interessa comparar dominios, e arrastar o crate `url` para dentro do
/// nucleo por causa disso seria pagar caro por pouco.
fn host_of(url: &str) -> Option<&str> {
    let sem_esquema = url.split_once("://").map(|(_, resto)| resto).unwrap_or(url);
    let autoridade = sem_esquema.split(['/', '?', '#']).next()?;
    let sem_credencial = autoridade
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(autoridade);
    let host = sem_credencial
        .split_once(':')
        .map(|(host, _)| host)
        .unwrap_or(sem_credencial);

    (!host.is_empty()).then_some(host)
}

/// O App do Registry que aponta para `host`, se houver algum.
///
/// Existe porque uma acao entre apps precisa achar QUAL registro concede a
/// permissao, e nenhuma das respostas obvias serve: o `id` e um UUID sorteado
/// no cadastro, diferente em cada instalacao, e o `name` e um campo que o
/// usuario pode reescrever a qualquer momento. O alvo, nao — se o App aponta
/// para o mesmo host que a acao vai escrever, e esse o registro que fala por
/// ele. Amarrar a permissao ao alvo tambem faz a coisa certa quando o usuario
/// tem dois cadastros do mesmo app: qualquer um deles com a capacidade
/// marcada e um consentimento dado para aquele destino.
pub fn app_targeting_host<'a>(apps: &'a [RegisteredApp], host: &str) -> Option<&'a RegisteredApp> {
    apps.iter().find(|app| {
        [app.launch_target.as_deref(), app.source_url.as_deref()]
            .into_iter()
            .flatten()
            .filter_map(host_of)
            .any(|encontrado| encontrado.eq_ignore_ascii_case(host))
    })
}

#[derive(Clone, Debug)]
pub struct NewRegisteredApp {
    pub id: AppId,
    pub name: String,
    pub description: String,
    pub source_url: Option<String>,
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
        Self::create_with_source(name, description, None, launch_kind, launch_target)
    }

    pub fn create_with_source(
        name: &str,
        description: &str,
        source_url: Option<&str>,
        launch_kind: Option<AppLaunchKind>,
        launch_target: Option<&str>,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            id: AppId::new(),
            name: required(name, "O nome do App nao pode estar vazio.")?,
            description: description.trim().to_owned(),
            source_url: validate_source_url(source_url)?,
            launch_kind,
            launch_target: validate_launch_target(launch_kind, launch_target)?,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_url: String,
    pub launch_kind: Option<AppLaunchKind>,
    pub launch_target: Option<String>,
}

impl AppCatalogEntry {
    pub fn into_new_app(self) -> Result<NewRegisteredApp, CoreError> {
        NewRegisteredApp::create_with_source(
            &self.name,
            &self.description,
            Some(&self.source_url),
            self.launch_kind,
            self.launch_target.as_deref(),
        )
    }
}

pub fn app_catalog() -> Vec<AppCatalogEntry> {
    vec![
        AppCatalogEntry {
            id: "cronocad".into(),
            name: "CronoCAD".into(),
            description: "Rastreador de horas local-first para projetos CAD.".into(),
            source_url: "https://github.com/theusinshow/cronocad".into(),
            launch_kind: None,
            launch_target: None,
        },
        AppCatalogEntry {
            id: "m-finance".into(),
            name: "M Finance".into(),
            description: "Cockpit pessoal de contas, vencimentos e faturas.".into(),
            source_url: "https://github.com/theusinshow/m-finance".into(),
            launch_kind: Some(AppLaunchKind::Url),
            launch_target: Some("https://m-finance-silk.vercel.app".into()),
        },
        AppCatalogEntry {
            id: "coded-atlas".into(),
            name: "Coded Atlas".into(),
            description: "Catalogo visual de projetos e gerador de assets de portfolio.".into(),
            source_url: "https://github.com/theusinshow/coded-atlas".into(),
            launch_kind: Some(AppLaunchKind::Url),
            launch_target: Some("https://coded-atlas.vercel.app".into()),
        },
    ]
}

pub fn validate_source_url(source_url: Option<&str>) -> Result<Option<String>, CoreError> {
    let source_url = source_url.map(str::trim).filter(|value| !value.is_empty());
    match source_url {
        Some(url) if url.starts_with("https://") || url.starts_with("http://") => {
            Ok(Some(url.to_owned()))
        }
        Some(_) => Err(CoreError::new(
            ErrorCode::InvalidInput,
            "A origem do App deve comecar com http:// ou https://.",
            false,
        )),
        None => Ok(None),
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

    #[test]
    fn source_url_is_optional_and_requires_http() {
        assert!(NewRegisteredApp::create_with_source(
            "CronoCAD",
            "",
            Some("github.com/theusinshow/cronocad"),
            None,
            None,
        )
        .is_err());
        assert!(NewRegisteredApp::create_with_source(
            "CronoCAD",
            "",
            Some("https://github.com/theusinshow/cronocad"),
            None,
            None,
        )
        .is_ok());
    }

    #[test]
    fn catalog_keeps_source_and_launch_separate() {
        let catalog = app_catalog();
        assert_eq!(catalog.len(), 3);
        let cronocad = catalog.iter().find(|entry| entry.id == "cronocad").unwrap();
        assert!(cronocad.source_url.ends_with("/cronocad"));
        assert!(cronocad.launch_target.is_none());
    }

    /// Um `RegisteredApp` cru, do jeito que sai do banco. Os construtores
    /// validam entrada; aqui o que interessa e a leitura.
    fn app_registrado(nome: &str, alvo: Option<&str>, origem: Option<&str>, escreve: bool) -> RegisteredApp {
        RegisteredApp {
            id: AppId::new(),
            name: nome.to_owned(),
            description: String::new(),
            source_url: origem.map(str::to_owned),
            launch_kind: alvo.map(|_| AppLaunchKind::Url),
            launch_target: alvo.map(str::to_owned),
            can_open: true,
            can_read: false,
            can_write: escreve,
            can_automate: false,
            lifecycle_state: LifecycleState::Active,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            last_opened_at: None,
        }
    }

    #[test]
    fn the_host_comes_out_of_every_url_shape_we_store() {
        assert_eq!(host_of("https://m-finance-silk.vercel.app/"), Some("m-finance-silk.vercel.app"));
        assert_eq!(host_of("https://m-finance-silk.vercel.app"), Some("m-finance-silk.vercel.app"));
        assert_eq!(host_of("https://m-finance-silk.vercel.app/app/bills?m=8"), Some("m-finance-silk.vercel.app"));
        assert_eq!(host_of("http://localhost:3010/api"), Some("localhost"));
        assert_eq!(host_of("https://user:senha@exemplo.com/x"), Some("exemplo.com"));
        assert_eq!(host_of("m-finance-silk.vercel.app/x"), Some("m-finance-silk.vercel.app"));
        assert_eq!(host_of(""), None);
        assert_eq!(host_of("https://"), None);
    }

    #[test]
    fn the_app_is_found_by_where_it_points() {
        let apps = vec![
            app_registrado("NexoDoc", Some("https://nexodoc.app"), None, true),
            app_registrado("M-Finance", Some("https://m-finance-silk.vercel.app/"), None, true),
        ];

        let achado = app_targeting_host(&apps, "m-finance-silk.vercel.app").expect("achou o App");
        assert_eq!(achado.name, "M-Finance");
        assert!(achado.can_write);
    }

    /// O nome e do usuario: ele escreve "M Finance", "M-Finance" ou "Grana" e
    /// nenhum desses e problema nosso enquanto o alvo continuar o mesmo.
    #[test]
    fn the_name_does_not_matter_only_the_target() {
        let apps = vec![app_registrado("Grana", Some("https://m-finance-silk.vercel.app/"), None, true)];
        assert!(app_targeting_host(&apps, "m-finance-silk.vercel.app").is_some());
    }

    #[test]
    fn the_source_url_also_counts_when_there_is_no_launch_target() {
        let apps = vec![app_registrado("M-Finance", None, Some("https://m-finance-silk.vercel.app"), true)];
        assert!(app_targeting_host(&apps, "m-finance-silk.vercel.app").is_some());
    }

    #[test]
    fn the_host_comparison_ignores_case() {
        let apps = vec![app_registrado("M-Finance", Some("https://M-Finance-Silk.Vercel.App/"), None, true)];
        assert!(app_targeting_host(&apps, "m-finance-silk.vercel.app").is_some());
    }

    /// Um host parecido nao pode herdar a permissao de outro — e o caso que
    /// transformaria um dominio vizinho em porta aberta.
    #[test]
    fn a_lookalike_host_is_not_a_match() {
        let apps = vec![
            app_registrado("Falso", Some("https://m-finance-silk.vercel.app.evil.com/"), None, true),
            app_registrado("Outro", Some("https://not-m-finance-silk.vercel.app/"), None, true),
        ];
        assert!(app_targeting_host(&apps, "m-finance-silk.vercel.app").is_none());
    }

    #[test]
    fn no_registered_app_for_the_host_means_no_match() {
        let apps = vec![app_registrado("NexoDoc", Some("https://nexodoc.app"), None, true)];
        assert!(app_targeting_host(&apps, "m-finance-silk.vercel.app").is_none());
        assert!(app_targeting_host(&[], "m-finance-silk.vercel.app").is_none());
    }

    /// Achar o App e conceder a permissao sao coisas diferentes: quem procura
    /// devolve o registro, e quem decide le o `can_write` dele.
    #[test]
    fn finding_the_app_is_not_the_same_as_granting_it() {
        let apps = vec![app_registrado("M-Finance", Some("https://m-finance-silk.vercel.app/"), None, false)];
        let achado = app_targeting_host(&apps, "m-finance-silk.vercel.app").expect("achou o App");
        assert!(!achado.can_write);
    }
}
