//! Vocabulario do gateway do Hermes.
//!
//! JSON-RPC 2.0 newline-delimited sobre WebSocket. Cada nome aqui foi lido do
//! codigo do Hermes e esta registrado em `docs/HERMES-GATEWAY-CONTRACT.md`.
//! Nada nesta lista foi inventado, e nada pode ser acrescentado sem verificar
//! antes contra `tui_gateway/server.py`.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::HermesError;

/// Eventos que o gateway emite.
///
/// `Unknown` existe de proposito: o contrato foi lido de um checkout local, e a
/// VPS pode divergir numa atualizacao futura. Um tipo desconhecido precisa
/// chegar a superficie nomeando a si mesmo, nunca virar falha generica.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HermesEvent {
    /// Aceite da conexao. E este frame — nao um HTTP 200 — que significa Online.
    Ready,
    MessageStart,
    MessageDelta {
        text: String,
    },
    MessageComplete,
    /// Raciocinio. Acumulado e escondido por padrao.
    ReasoningDelta {
        text: String,
    },
    ToolStart {
        name: String,
    },
    ToolComplete {
        name: String,
    },
    /// Evento de ENTRADA: o agente parou e espera. Ignorar isto trava a
    /// conversa em silencio.
    ApprovalRequest {
        prompt: String,
    },
    Status {
        text: String,
    },
    Failed {
        message: String,
    },
    /// Resposta de erro a um request nosso. `code` importa: 4009 e "session
    /// busy" e tem tratamento de UI proprio.
    Rejected {
        code: i64,
        message: String,
        id: Option<i64>,
    },
    Unknown {
        kind: String,
    },
}

#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    params: Option<EventParams>,
    #[serde(default)]
    error: Option<ErrorBody>,
    #[serde(default)]
    id: Option<i64>,
}

#[derive(Deserialize)]
struct EventParams {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    payload: Value,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: i64,
    message: String,
}

fn text_of(payload: &Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

impl HermesEvent {
    pub fn parse(frame: &str) -> Result<Self, HermesError> {
        let envelope: Envelope = serde_json::from_str(frame)
            .map_err(|error| HermesError::Protocol(format!("frame ilegivel: {error}")))?;

        if let Some(error) = envelope.error {
            return Ok(Self::Rejected {
                code: error.code,
                message: error.message,
                id: envelope.id,
            });
        }

        let Some(params) = envelope.params else {
            return Ok(Self::Unknown {
                kind: "<sem params>".to_owned(),
            });
        };

        let payload = &params.payload;
        Ok(match params.kind.as_str() {
            "gateway.ready" => Self::Ready,
            "message.start" => Self::MessageStart,
            "message.delta" => Self::MessageDelta {
                text: text_of(payload, "text"),
            },
            "message.complete" => Self::MessageComplete,
            "reasoning.delta" | "thinking.delta" => Self::ReasoningDelta {
                text: text_of(payload, "text"),
            },
            "tool.start" => Self::ToolStart {
                name: text_of(payload, "name"),
            },
            "tool.complete" => Self::ToolComplete {
                name: text_of(payload, "name"),
            },
            "approval.request" => Self::ApprovalRequest {
                prompt: text_of(payload, "prompt"),
            },
            "status.update" => Self::Status {
                text: text_of(payload, "text"),
            },
            "error" => Self::Failed {
                message: text_of(payload, "message"),
            },
            other => Self::Unknown {
                kind: other.to_owned(),
            },
        })
    }
}

/// Requests que o M/OS emite. Os nomes seguem o contrato exatamente.
pub struct Request;

impl Request {
    fn call(id: i64, method: &str, params: Value) -> String {
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
    }

    /// Titulo fixo em "M/OS" para a sessao ser reconhecivel no dashboard, e
    /// para servir de rota de recuperacao: session.resume aceita titulo quando
    /// o id nao existe mais.
    ///
    /// Sem model, provider ou reasoning_effort: o Hermes e dono disso.
    pub fn session_create(id: i64) -> String {
        Self::call(id, "session.create", json!({ "title": "M/OS" }))
    }

    pub fn session_resume(id: i64, target: &str) -> String {
        Self::call(id, "session.resume", json!({ "session_id": target }))
    }

    pub fn prompt_submit(id: i64, session_id: &str, text: &str) -> String {
        Self::call(
            id,
            "prompt.submit",
            json!({ "session_id": session_id, "text": text }),
        )
    }

    /// Tambem resolve como `deny` todas as aprovacoes pendentes da sessao, o
    /// que faz do cancelamento a saida segura de um approval travado.
    pub fn session_interrupt(id: i64, session_id: &str) -> String {
        Self::call(id, "session.interrupt", json!({ "session_id": session_id }))
    }

    pub fn session_close(id: i64, session_id: &str) -> String {
        Self::call(id, "session.close", json!({ "session_id": session_id }))
    }

    /// `choice` tem default "deny" no servidor, e o M/OS segue o mesmo default:
    /// fechar sem escolher nega, nunca aprova por omissao.
    pub fn approval_respond(id: i64, session_id: &str, approved: bool) -> String {
        Self::call(
            id,
            "approval.respond",
            json!({
                "session_id": session_id,
                "choice": if approved { "approve" } else { "deny" },
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_ready_frame() {
        let frame = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"gateway.ready","payload":{"skin":"default"}}}"#;
        assert_eq!(HermesEvent::parse(frame).unwrap(), HermesEvent::Ready);
    }

    #[test]
    fn parses_a_message_delta() {
        let frame = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"message.delta","payload":{"text":"ola"}}}"#;
        match HermesEvent::parse(frame).unwrap() {
            HermesEvent::MessageDelta { text } => assert_eq!(text, "ola"),
            other => panic!("esperava MessageDelta, veio {other:?}"),
        }
    }

    #[test]
    fn thinking_and_reasoning_collapse_into_one_event() {
        let thinking = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"thinking.delta","payload":{"text":"hm"}}}"#;
        let reasoning = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"reasoning.delta","payload":{"text":"hm"}}}"#;
        assert_eq!(
            HermesEvent::parse(thinking).unwrap(),
            HermesEvent::parse(reasoning).unwrap()
        );
    }

    #[test]
    fn parses_an_approval_request() {
        let frame = r#"{"jsonrpc":"2.0","method":"event","params":{"type":"approval.request","payload":{"prompt":"rodar git push?"}}}"#;
        match HermesEvent::parse(frame).unwrap() {
            HermesEvent::ApprovalRequest { prompt } => assert_eq!(prompt, "rodar git push?"),
            other => panic!("esperava ApprovalRequest, veio {other:?}"),
        }
    }

    #[test]
    fn unknown_event_names_itself() {
        let frame =
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"quantum.flux","payload":{}}}"#;
        match HermesEvent::parse(frame).unwrap() {
            HermesEvent::Unknown { kind } => assert_eq!(kind, "quantum.flux"),
            other => panic!("esperava Unknown, veio {other:?}"),
        }
    }

    #[test]
    fn parses_a_busy_error_response() {
        let frame = r#"{"jsonrpc":"2.0","error":{"code":4009,"message":"session busy"},"id":7}"#;
        match HermesEvent::parse(frame).unwrap() {
            HermesEvent::Rejected { code, message, id } => {
                assert_eq!(code, 4009);
                assert_eq!(message, "session busy");
                assert_eq!(id, Some(7));
            }
            other => panic!("esperava Rejected, veio {other:?}"),
        }
    }

    #[test]
    fn an_unreadable_frame_is_a_protocol_error() {
        assert!(HermesEvent::parse("isto nao e json").is_err());
    }

    #[test]
    fn requests_use_the_verified_method_names() {
        assert!(Request::session_create(1).contains(r#""method":"session.create""#));
        assert!(Request::prompt_submit(2, "abc", "oi").contains(r#""method":"prompt.submit""#));
        assert!(Request::session_interrupt(3, "abc").contains(r#""method":"session.interrupt""#));
        assert!(Request::session_close(4, "abc").contains(r#""method":"session.close""#));
        assert!(Request::session_resume(5, "M/OS").contains(r#""method":"session.resume""#));
        assert!(
            Request::approval_respond(6, "abc", false).contains(r#""method":"approval.respond""#)
        );
    }

    #[test]
    fn approval_defaults_to_deny_wording() {
        assert!(Request::approval_respond(1, "abc", false).contains(r#""choice":"deny""#));
        assert!(Request::approval_respond(1, "abc", true).contains(r#""choice":"approve""#));
    }

    #[test]
    fn session_create_does_not_pin_a_model() {
        // O Hermes e dono de modelo, provider e reasoning. Mandar qualquer um
        // destes daqui seria o M/OS opinando sobre o que nao lhe pertence.
        let frame = Request::session_create(1);
        assert!(!frame.contains("model"));
        assert!(!frame.contains("provider"));
        assert!(!frame.contains("reasoning_effort"));
        assert!(frame.contains(r#""title":"M/OS""#));
    }
}
