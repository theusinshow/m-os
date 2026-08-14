//! A ponte: sessao, correlacao de requests e maquina de estados.
//!
//! Nao conhece rede. Recebe `Channels` e opera sobre eles, o que faz todos os
//! cenarios da spec rodarem sem tocar a rede.

#[cfg(test)]
use tokio::sync::mpsc;

use crate::{Channels, ConnectionState, HermesError, HermesEvent, Request};

/// O que a ponte entrega para cima. E o que vira evento Tauri.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// Texto da resposta, token a token. Nao reagrupar: o servidor desativa o
    /// algoritmo de Nagle de proposito para preservar a cadencia, e
    /// rebufferizar aqui desfaria esse trabalho.
    Delta {
        text: String,
    },
    Complete,
    /// Acumulado e escondido por padrao.
    Reasoning {
        text: String,
    },
    Tool {
        name: String,
        running: bool,
    },
    /// O agente parou e espera. Ignorar isto trava a conversa em silencio.
    Approval {
        prompt: String,
    },
    Busy,
    Failed {
        message: String,
    },
    /// Contrato divergiu. Nomeia o que veio, em vez de virar erro generico.
    UnknownFrame {
        kind: String,
    },
}

pub struct Bridge {
    channels: Channels,
    state: ConnectionState,
    session_id: Option<String>,
    next_id: i64,
}

impl Bridge {
    /// A ponte nasce `Connecting`: so o frame `gateway.ready` promove para
    /// `Online`. Um HTTP 200 nao basta — e o falso positivo que o cliente de
    /// referencia documenta.
    pub fn new(channels: Channels) -> Self {
        Self {
            channels,
            state: ConnectionState::Connecting,
            session_id: None,
            next_id: 1,
        }
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    fn take_id(&mut self) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    async fn send(&mut self, frame: String) -> Result<(), HermesError> {
        self.channels
            .outgoing
            .send(frame)
            .await
            .map_err(|_| HermesError::Unreachable("a conexao com o Hermes caiu".into()))
    }

    /// Abre a sessao do M/OS. `resume` recebe o id guardado localmente; quando
    /// ele nao existe mais na VPS, `session.resume` aceita o titulo como alvo,
    /// e "M/OS" e a rota de recuperacao antes de criar sessao nova.
    pub async fn open_session(&mut self, resume: Option<&str>) -> Result<(), HermesError> {
        let id = self.take_id();
        let frame = match resume {
            Some(session_id) => Request::session_resume(id, session_id),
            None => Request::session_create(id),
        };
        self.send(frame).await
    }

    pub async fn submit(&mut self, text: &str) -> Result<(), HermesError> {
        let session_id = self
            .session_id
            .clone()
            .ok_or_else(|| HermesError::Gateway("Nenhuma sessao aberta no Hermes.".into()))?;
        let id = self.take_id();
        self.send(Request::prompt_submit(id, &session_id, text))
            .await
    }

    /// Tambem e a saida segura de um approval travado: o servidor resolve como
    /// `deny` todas as aprovacoes pendentes da sessao ao interromper.
    pub async fn interrupt(&mut self) -> Result<(), HermesError> {
        let Some(session_id) = self.session_id.clone() else {
            return Ok(());
        };
        let id = self.take_id();
        self.send(Request::session_interrupt(id, &session_id)).await
    }

    pub async fn respond_approval(&mut self, approved: bool) -> Result<(), HermesError> {
        let session_id = self
            .session_id
            .clone()
            .ok_or_else(|| HermesError::Gateway("Nenhuma sessao aberta no Hermes.".into()))?;
        let id = self.take_id();
        self.send(Request::approval_respond(id, &session_id, approved))
            .await
    }

    pub async fn close(&mut self) -> Result<(), HermesError> {
        let Some(session_id) = self.session_id.clone() else {
            return Ok(());
        };
        let id = self.take_id();
        self.send(Request::session_close(id, &session_id)).await
    }

    /// Consome o proximo frame. `None` significa socket encerrado — queda de
    /// conexao, nao fim de turno.
    pub async fn next(&mut self) -> Option<Result<Outcome, HermesError>> {
        let frame = match self.channels.incoming.recv().await {
            Some(Ok(frame)) => frame,
            Some(Err(error)) => {
                self.state = ConnectionState::Offline;
                return Some(Err(error));
            }
            None => {
                self.state = ConnectionState::Offline;
                return None;
            }
        };

        match HermesEvent::parse(&frame) {
            Err(error) => Some(Err(error)),
            Ok(event) => self.absorb(event, &frame),
        }
    }

    fn absorb(&mut self, event: HermesEvent, frame: &str) -> Option<Result<Outcome, HermesError>> {
        match event {
            HermesEvent::Ready => {
                self.state = ConnectionState::Online;
                None
            }
            HermesEvent::MessageStart => None,
            HermesEvent::MessageDelta { text } => Some(Ok(Outcome::Delta { text })),
            HermesEvent::MessageComplete => Some(Ok(Outcome::Complete)),
            HermesEvent::ReasoningDelta { text } => Some(Ok(Outcome::Reasoning { text })),
            HermesEvent::ToolStart { name } => Some(Ok(Outcome::Tool {
                name,
                running: true,
            })),
            HermesEvent::ToolComplete { name } => Some(Ok(Outcome::Tool {
                name,
                running: false,
            })),
            HermesEvent::ApprovalRequest { prompt } => Some(Ok(Outcome::Approval { prompt })),
            HermesEvent::Failed { message } => Some(Ok(Outcome::Failed { message })),
            HermesEvent::Status { .. } => None,
            // 4009 e envio concorrente: a UI oferece cancelar, nao repete.
            HermesEvent::Rejected { code: 4009, .. } => Some(Ok(Outcome::Busy)),
            HermesEvent::Rejected { code, message, .. } => Some(Ok(Outcome::Failed {
                message: format!("{message} ({code})"),
            })),
            HermesEvent::Unknown { kind } => {
                // session.create e session.resume respondem com o id da sessao
                // num result, nao num evento tipado. Colhemos aqui.
                if let Some(session_id) = session_id_from(frame) {
                    self.session_id = Some(session_id);
                    return None;
                }
                Some(Ok(Outcome::UnknownFrame { kind }))
            }
        }
    }
}

/// Procura `result.session_id` na resposta de `session.create`/`session.resume`.
fn session_id_from(frame: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(frame).ok()?;
    value
        .get("result")?
        .get("session_id")?
        .as_str()
        .map(str::to_owned)
}

/// Canais para exercitar a ponte sem rede.
#[cfg(test)]
pub(crate) fn fake_channels() -> (
    Channels,
    mpsc::Receiver<String>,
    mpsc::Sender<Result<String, HermesError>>,
) {
    let (outgoing_tx, outgoing_rx) = mpsc::channel::<String>(32);
    let (incoming_tx, incoming_rx) = mpsc::channel::<Result<String, HermesError>>(256);
    (
        Channels {
            outgoing: outgoing_tx,
            incoming: incoming_rx,
        },
        outgoing_rx,
        incoming_tx,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const READY: &str =
        r#"{"jsonrpc":"2.0","method":"event","params":{"type":"gateway.ready","payload":{}}}"#;
    const SESSION: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"session_id":"a1b2c3d4"}}"#;

    fn delta(text: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","method":"event","params":{{"type":"message.delta","payload":{{"text":"{text}"}}}}}}"#
        )
    }

    #[tokio::test]
    async fn only_the_ready_frame_promotes_to_online() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);
        assert_eq!(bridge.state(), ConnectionState::Connecting);

        push.send(Ok(READY.into())).await.unwrap();
        assert!(bridge.next().await.is_none());
        assert_eq!(bridge.state(), ConnectionState::Online);
    }

    #[tokio::test]
    async fn collects_the_session_id_from_the_create_result() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(SESSION.into())).await.unwrap();
        assert!(bridge.next().await.is_none());
        assert_eq!(bridge.session_id(), Some("a1b2c3d4"));
    }

    #[tokio::test]
    async fn streams_deltas_without_regrouping() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        for piece in ["Bom", " dia"] {
            push.send(Ok(delta(piece))).await.unwrap();
        }

        // Dois frames entram, dois saem. Reagrupar aqui desfaria o trabalho que
        // o servidor faz desligando o Nagle para preservar a cadencia.
        assert_eq!(
            bridge.next().await.unwrap().unwrap(),
            Outcome::Delta { text: "Bom".into() }
        );
        assert_eq!(
            bridge.next().await.unwrap().unwrap(),
            Outcome::Delta {
                text: " dia".into()
            }
        );
    }

    #[tokio::test]
    async fn a_dropped_socket_is_a_connection_loss_not_an_end_of_turn() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);
        push.send(Ok(READY.into())).await.unwrap();
        bridge.next().await;
        assert_eq!(bridge.state(), ConnectionState::Online);

        drop(push);
        assert!(bridge.next().await.is_none());
        assert_eq!(bridge.state(), ConnectionState::Offline);
    }

    #[tokio::test]
    async fn busy_is_its_own_outcome() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(
            r#"{"jsonrpc":"2.0","error":{"code":4009,"message":"session busy"},"id":3}"#.into(),
        ))
        .await
        .unwrap();
        assert_eq!(bridge.next().await.unwrap().unwrap(), Outcome::Busy);
    }

    #[tokio::test]
    async fn an_unknown_frame_names_itself() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"quantum.flux","payload":{}}}"#
                .into(),
        ))
        .await
        .unwrap();
        assert_eq!(
            bridge.next().await.unwrap().unwrap(),
            Outcome::UnknownFrame {
                kind: "quantum.flux".into()
            }
        );
    }

    #[tokio::test]
    async fn approval_reaches_the_surface() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"approval.request","payload":{"prompt":"apagar?"}}}"#.into(),
        ))
        .await
        .unwrap();
        assert_eq!(
            bridge.next().await.unwrap().unwrap(),
            Outcome::Approval {
                prompt: "apagar?".into()
            }
        );
    }

    #[tokio::test]
    async fn resume_targets_the_stored_id_and_falls_back_to_the_title() {
        let (channels, mut sent, _push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        bridge.open_session(Some("a1b2c3d4")).await.unwrap();
        let frame = sent.recv().await.unwrap();
        assert!(frame.contains(r#""method":"session.resume""#));
        assert!(frame.contains("a1b2c3d4"));

        bridge.open_session(Some("M/OS")).await.unwrap();
        assert!(sent.recv().await.unwrap().contains("M/OS"));
    }

    #[tokio::test]
    async fn interrupt_without_a_session_is_harmless() {
        let (channels, _sent, _push) = fake_channels();
        let mut bridge = Bridge::new(channels);
        // Cancelar antes de abrir sessao nao pode explodir: o usuario pode
        // apertar Esc a qualquer momento.
        assert!(bridge.interrupt().await.is_ok());
    }

    #[tokio::test]
    async fn submitting_without_a_session_is_refused_clearly() {
        let (channels, _sent, _push) = fake_channels();
        let mut bridge = Bridge::new(channels);
        let error = bridge.submit("oi").await.unwrap_err();
        assert!(error.to_string().contains("sessao"));
    }
}
