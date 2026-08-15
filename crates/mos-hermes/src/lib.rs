//! Ponte entre o M/OS e o Hermes que ja existe.
//!
//! O M/OS nao cria um agente. Ele e mais uma superficie do mesmo Hermes usado
//! por WhatsApp e pelo dashboard: o gateway delega ao mesmo dispatcher, entao
//! as skills e as ferramentas sao identicas sem nenhum trabalho de sincronia.
//!
//! Divisao de responsabilidade:
//!
//! | Hermes                        | M/OS                  |
//! |-------------------------------|-----------------------|
//! | modelo, provider, reasoning   | interface e UX        |
//! | skills e tools                | contexto local        |
//! | execucao agentic              | sessao propria        |
//! | historico da conversa         | o session_id, so isso |
//!
//! Contrato verificado em `docs/HERMES-GATEWAY-CONTRACT.md`.

mod auth;
mod bridge;
mod protocol;
mod transport;

pub use auth::{Credentials, Gateway, GatewayStatus};
pub use bridge::{Bridge, Outcome};
pub use protocol::{HermesEvent, Request};
pub use transport::{connect, Channels};

use thiserror::Error;

/// Estado da conexao. `Error` e por turno, nao pela conexao.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    #[default]
    Offline,
    Connecting,
    Online,
}

#[derive(Debug, Error)]
pub enum HermesError {
    /// Nao alcancou o endpoint local. A causa quase sempre e o tunel fechado,
    /// e a mensagem diz isso em vez de "erro de rede".
    #[error("Hermes indisponivel: o tunel SSH nao parece estar aberto ({0})")]
    Unreachable(String),

    /// 401 do gateway, ou fechamento 4401 no socket.
    #[error("Credencial recusada pelo Hermes: {0}")]
    Unauthorized(String),

    /// 429 no login. Nao deve disparar retry: repetir piora o que causou o
    /// bloqueio.
    #[error("Tentativas demais de login. Espere antes de tentar de novo.")]
    RateLimited,

    /// Frame que nao casa com o contrato conhecido.
    #[error("Protocolo do Hermes divergiu do contrato conhecido: {0}")]
    Protocol(String),

    #[error("Nenhuma credencial do Hermes configurada em Settings.")]
    MissingCredentials,

    #[error("{0}")]
    Gateway(String),
}

impl HermesError {
    /// Nome estavel da falha, para quem esta do outro lado do IPC decidir o que
    /// fazer. Sem isto o renderer so recebe a mensagem em texto, e decidir por
    /// substring seria decidir por acaso.
    pub fn kind(&self) -> &'static str {
        match self {
            HermesError::Unreachable(_) => "unreachable",
            HermesError::Unauthorized(_) => "unauthorized",
            HermesError::RateLimited => "rate_limited",
            HermesError::Protocol(_) => "protocol",
            HermesError::MissingCredentials => "missing_credentials",
            HermesError::Gateway(_) => "gateway",
        }
    }

    /// Se repetir a MESMA tentativa pode dar certo sozinho.
    ///
    /// So a ausencia de transporte e: o tunel abre depois e a proxima tentativa
    /// encontra o gateway. Credencial recusada nao muda por insistencia, e
    /// RateLimited piora — repetir e exatamente o que causou o bloqueio. E o
    /// motivo de este metodo existir: um supervisor que reconecta sozinho
    /// precisa saber a diferenca antes de tentar de novo.
    pub fn retriable(&self) -> bool {
        matches!(self, HermesError::Unreachable(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_is_the_default_state() {
        assert_eq!(ConnectionState::default(), ConnectionState::Offline);
    }

    /// O texto do erro precisa dizer a causa provavel. "Erro de rede" mandaria
    /// o usuario procurar no lugar errado: o tunel e iniciado por fora e cair
    /// e o modo de falha esperado.
    #[test]
    fn unreachable_names_the_tunnel() {
        let message = HermesError::Unreachable("connection refused".into()).to_string();
        assert!(message.contains("tunel"));
    }
}
