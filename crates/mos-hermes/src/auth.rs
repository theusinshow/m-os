//! Autenticacao no gateway do Hermes.
//!
//! Fluxo verificado ao vivo (ver `HERMES-GATEWAY-CONTRACT.md` §3.1):
//!
//! ```text
//! GET  /api/status            -> auth_required, auth_providers
//! POST /auth/password-login   -> cookies de sessao
//! POST /api/auth/ws-ticket    -> {ticket, ttl_seconds: 30}
//! ws://.../api/ws?ticket=...  -> imediatamente
//! ```
//!
//! Nada aqui toca o renderer. Usuario e senha vivem no Windows Credential
//! Manager e sao lidos so deste lado (`ARCHITECTURE.md:556`).

use serde::Deserialize;

use crate::HermesError;

const SERVICE: &str = "m-os";
const ACCOUNT: &str = "hermes-gateway";

/// Credencial do dashboard, guardada no Credential Manager do sistema.
///
/// O renderer nunca recebe nenhum destes campos — so um booleano dizendo se
/// existe credencial configurada.
#[derive(Clone, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    fn entry() -> Result<keyring::Entry, HermesError> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|error| {
            HermesError::Gateway(format!("Credential Manager indisponivel: {error}"))
        })
    }

    pub fn load() -> Result<Self, HermesError> {
        let stored = Self::entry()?
            .get_password()
            .map_err(|_| HermesError::MissingCredentials)?;
        // O usuario nao pode conter ':'; a senha pode, e por isso o split e
        // limitado a duas partes.
        let (username, password) = stored.split_once(':').ok_or_else(|| {
            HermesError::Gateway("Credencial guardada em formato invalido.".into())
        })?;
        Ok(Self {
            username: username.to_owned(),
            password: password.to_owned(),
        })
    }

    pub fn store(username: &str, password: &str) -> Result<(), HermesError> {
        if username.contains(':') {
            return Err(HermesError::Gateway(
                "O usuario do Hermes nao pode conter ':'.".into(),
            ));
        }
        Self::entry()?
            .set_password(&format!("{username}:{password}"))
            .map_err(|error| HermesError::Gateway(format!("Nao foi possivel guardar: {error}")))
    }

    pub fn clear() -> Result<(), HermesError> {
        match Self::entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(HermesError::Gateway(format!(
                "Nao foi possivel remover: {error}"
            ))),
        }
    }

    pub fn exist() -> bool {
        Self::load().is_ok()
    }
}

#[derive(Debug, Deserialize)]
pub struct GatewayStatus {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub auth_required: bool,
    #[serde(default)]
    pub auth_providers: Vec<String>,
}

#[derive(Deserialize)]
struct Ticket {
    ticket: String,
}

/// Cliente HTTP do gateway.
///
/// O cookie jar e persistente de proposito: o middleware rotaciona o access
/// token (~15 min) a partir do refresh (24h) na proxima requisicao autenticada.
/// Requisicoes soltas perderiam a sessao a cada 15 minutos sem motivo.
pub struct Gateway {
    base_url: String,
    client: reqwest::Client,
}

impl Gateway {
    pub fn new(base_url: &str) -> Result<Self, HermesError> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .map_err(|error| HermesError::Gateway(error.to_string()))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            client,
        })
    }

    fn unreachable(error: reqwest::Error) -> HermesError {
        HermesError::Unreachable(error.to_string())
    }

    pub async fn status(&self) -> Result<GatewayStatus, HermesError> {
        self.client
            .get(format!("{}/api/status", self.base_url))
            .send()
            .await
            .map_err(Self::unreachable)?
            .json()
            .await
            .map_err(|error| HermesError::Protocol(format!("/api/status ilegivel: {error}")))
    }

    /// Falhas sao propositalmente genericas do lado do servidor, para o
    /// endpoint nao virar oraculo de enumeracao. Traduzimos por codigo, e 429
    /// nunca vira retry: repetir piora exatamente o que causou o bloqueio.
    pub async fn login(
        &self,
        credentials: &Credentials,
        provider: &str,
    ) -> Result<(), HermesError> {
        let response = self
            .client
            .post(format!("{}/auth/password-login", self.base_url))
            .json(&serde_json::json!({
                "provider": provider,
                "username": credentials.username,
                "password": credentials.password,
            }))
            .send()
            .await
            .map_err(Self::unreachable)?;

        match response.status().as_u16() {
            200 => Ok(()),
            401 => Err(HermesError::Unauthorized(
                "usuario ou senha do dashboard nao conferem".into(),
            )),
            404 => Err(HermesError::Gateway(format!(
                "O gateway nao expoe o provider '{provider}'."
            ))),
            429 => Err(HermesError::RateLimited),
            503 => Err(HermesError::Gateway(
                "O Hermes esta no ar mas nao alcanca o proprio armazenamento de sessao.".into(),
            )),
            other => Err(HermesError::Gateway(format!(
                "Login recusado com status {other}."
            ))),
        }
    }

    /// Vale 30 segundos e e de uso unico. Cunhar um por socket, imediatamente
    /// antes de conectar — guardar ticket e bug.
    pub async fn mint_ticket(&self) -> Result<String, HermesError> {
        let response = self
            .client
            .post(format!("{}/api/auth/ws-ticket", self.base_url))
            .send()
            .await
            .map_err(Self::unreachable)?;

        if response.status().as_u16() == 401 {
            return Err(HermesError::Unauthorized("sessao expirada".into()));
        }

        let ticket: Ticket = response
            .json()
            .await
            .map_err(|error| HermesError::Protocol(format!("ws-ticket ilegivel: {error}")))?;
        Ok(ticket.ticket)
    }

    /// `http` vira `ws`, `https` vira `wss`, e o prefixo de caminho e
    /// preservado — o gateway pode viver atras de um proxy com prefixo.
    pub fn websocket_url(&self, ticket: &str) -> String {
        let base = self
            .base_url
            .replacen("https://", "wss://", 1)
            .replacen("http://", "ws://", 1);
        format!("{base}/api/ws?ticket={}", urlencode(ticket))
    }
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_url_switches_scheme_and_keeps_prefix() {
        let gateway = Gateway::new("http://127.0.0.1:9119").unwrap();
        assert_eq!(
            gateway.websocket_url("abc"),
            "ws://127.0.0.1:9119/api/ws?ticket=abc"
        );

        let secure = Gateway::new("https://box:9119/hermes/").unwrap();
        assert_eq!(
            secure.websocket_url("abc"),
            "wss://box:9119/hermes/api/ws?ticket=abc"
        );
    }

    #[test]
    fn ticket_is_escaped() {
        let gateway = Gateway::new("http://127.0.0.1:9119").unwrap();
        assert!(gateway.websocket_url("a b/c+d").contains("a%20b%2Fc%2Bd"));
    }

    #[test]
    fn status_parses_the_real_body() {
        // Corpo capturado ao vivo do gateway em 2026-08-13.
        let body = r#"{"version":"0.16.0","release_date":"2026.6.5","gateway_running":true,
            "active_sessions":0,"auth_required":true,"auth_providers":["basic"]}"#;
        let status: GatewayStatus = serde_json::from_str(body).unwrap();
        assert_eq!(status.version, "0.16.0");
        assert!(status.auth_required);
        assert_eq!(status.auth_providers, vec!["basic".to_owned()]);
    }

    #[test]
    fn a_username_with_colon_is_refused_before_storing() {
        // O separador guardado e ':'; aceitar um usuario com ':' corromperia a
        // leitura de um jeito que so apareceria no proximo login.
        assert!(Credentials::store("a:b", "senha").is_err());
    }
}
