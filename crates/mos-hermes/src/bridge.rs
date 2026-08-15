//! A ponte: sessao, correlacao de requests e maquina de estados.
//!
//! Nao conhece rede. Recebe `Channels` e opera sobre eles, o que faz todos os
//! cenarios da spec rodarem sem tocar a rede.

use std::collections::HashMap;

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
    /// O agente parou e espera INFORMACAO, nao permissao.
    Clarify {
        request_id: String,
        question: String,
        choices: Vec<String>,
    },
    /// O agente pediu senha de sudo na VPS e a ponte ja recusou por politica.
    /// Chega a superficie so para a recusa poder ser explicada — nao ha decisao
    /// pendente aqui.
    SudoRefused,
    /// `status.update`. Antes era absorvido e perdido; e o texto que explica o
    /// que o agente esta fazendo entre um token e outro.
    Status {
        text: String,
    },
    /// Resposta de `session.history`. E a conversa que ja existia na VPS.
    History {
        messages: Vec<HistoryMessage>,
    },
    /// Resposta de `session.title`, com ou sem titulo pedido.
    Title {
        title: String,
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

/// Uma mensagem do historico da VPS.
///
/// Deliberadamente rasa: o M/OS projeta isto em `Message`/`MessagePart` do lado
/// do dominio, e a ponte nao conhece esse modelo.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessage {
    /// `user`, `assistant`, `tool` ou `system`.
    pub role: String,
    pub content: String,
    /// Preenchido so quando `role` e `tool`.
    pub tool_name: String,
}

/// O que esperamos de volta de um request nosso.
///
/// Substituiu um `Option<i64>` que so sabia rastrear o resume. Com history e
/// title no caminho, correlacionar por id deixou de ser caso especial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Resume,
    Session,
    History,
    Title,
}

pub struct Bridge {
    channels: Channels,
    state: ConnectionState,
    session_id: Option<String>,
    next_id: i64,
    /// Requests em voo, por id. Reconhecer a rejeicao de um `session.resume`
    /// para cair em `session.create` continua sendo o caso critico — sem isso,
    /// uma sessao que sumiu da VPS deixava o Hermes inutilizavel ate o app
    /// reiniciar.
    pending: HashMap<i64, Pending>,
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
            pending: HashMap::new(),
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
            Some(session_id) => {
                self.pending.insert(id, Pending::Resume);
                Request::session_resume(id, session_id)
            }
            None => {
                self.pending.insert(id, Pending::Session);
                Request::session_create(id)
            }
        };
        self.send(frame).await
    }

    /// Abre sessao nova depois de um resume rejeitado.
    async fn create_after_failed_resume(&mut self) -> Result<(), HermesError> {
        let id = self.take_id();
        self.pending.insert(id, Pending::Session);
        self.send(Request::session_create(id)).await
    }

    /// Pede o historico que ja existe na VPS.
    ///
    /// E o que torna a conversa continua entre aberturas do app: o M/OS guarda o
    /// vinculo, e o conteudo vem de la.
    pub async fn request_history(&mut self) -> Result<(), HermesError> {
        let session_id = self.require_session()?;
        let id = self.take_id();
        self.pending.insert(id, Pending::History);
        self.send(Request::session_history(id, &session_id)).await
    }

    /// Define o titulo, ou pede o atual quando `title` e `None`.
    pub async fn set_title(&mut self, title: Option<&str>) -> Result<(), HermesError> {
        let session_id = self.require_session()?;
        let id = self.take_id();
        self.pending.insert(id, Pending::Title);
        self.send(Request::session_title(id, &session_id, title))
            .await
    }

    /// Responde a clarificacao. Sem sessao no frame: a correlacao e por
    /// `request_id`, e o servidor nao consulta a sessao aqui.
    pub async fn respond_clarify(
        &mut self,
        request_id: &str,
        answer: &str,
    ) -> Result<(), HermesError> {
        let id = self.take_id();
        self.send(Request::clarify_respond(id, request_id, answer))
            .await
    }

    fn require_session(&self) -> Result<String, HermesError> {
        self.session_id
            .clone()
            .ok_or_else(|| HermesError::Gateway("Nenhuma sessao aberta no Hermes.".into()))
    }

    pub async fn submit(&mut self, text: &str) -> Result<(), HermesError> {
        let session_id = self.require_session()?;
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
        let session_id = self.require_session()?;
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

    /// Consome frames ate ter algo para entregar.
    ///
    /// `None` significa UMA coisa so: o socket fechou.
    ///
    /// Antes significava duas — "socket fechou" e "este quadro nao produz saida
    /// visivel" — e as duas colidiam no primeiro quadro da conexao. O servidor
    /// emite `gateway.ready` no aceite; `absorb` o traduzia para `None`; e o
    /// laco da ponte no app, que trata `None` como queda, encerrava a conexao
    /// no exato instante em que ela era confirmada. Reconectava, recebia ready
    /// de novo, encerrava de novo: o Hermes nunca chegou a funcionar, e o
    /// sintoma era um laco Conectando -> Desconectado sem fim.
    ///
    /// Agora quadro sem saida visivel apenas continua a leitura.
    pub async fn next(&mut self) -> Option<Result<Outcome, HermesError>> {
        loop {
            let frame = match self.channels.incoming.recv().await {
                Some(Ok(frame)) => frame,
                // Erro de frame nao derruba a conexao: um frame ilegivel ou
                // binario nao fecha o socket, e marcar Offline aqui travava o
                // estado para sempre — so `gateway.ready` volta a promover para
                // Online, e ele so chega uma vez, no aceite.
                Some(Err(error)) => return Some(Err(error)),
                // Canal seco: o socket fechou de verdade.
                None => {
                    self.state = ConnectionState::Offline;
                    return None;
                }
            };

            let event = match HermesEvent::parse(&frame) {
                Err(error) => return Some(Err(error)),
                Ok(event) => event,
            };

            // Resume rejeitado: a sessao sumiu da VPS, ou o gateway nao conhece
            // o metodo. Abrir uma nova em vez de deixar o Hermes inutilizavel.
            if let HermesEvent::Rejected { id: Some(id), .. } = &event {
                if self.pending.remove(id) == Some(Pending::Resume) {
                    if let Err(error) = self.create_after_failed_resume().await {
                        return Some(Err(error));
                    }
                    continue;
                }
            }

            // Sudo e decisao de politica, nao do usuario: a ponte recusa aqui
            // mesmo e destrava o agente na hora. Levar isto ate a UI para
            // esperar um clique manteria a thread do agente parada por 120 s
            // para chegar a mesma resposta.
            if let HermesEvent::SudoRequest { request_id } = &event {
                let id = self.take_id();
                let frame = Request::sudo_refuse(id, request_id);
                if let Err(error) = self.send(frame).await {
                    return Some(Err(error));
                }
                return Some(Ok(Outcome::SudoRefused));
            }

            if let Some(outcome) = self.absorb(event, &frame) {
                return Some(outcome);
            }
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
            HermesEvent::ClarifyRequest {
                request_id,
                question,
                choices,
            } => Some(Ok(Outcome::Clarify {
                request_id,
                question,
                choices,
            })),
            // Ja tratado em `next`, antes de chegar aqui.
            HermesEvent::SudoRequest { .. } => None,
            HermesEvent::Failed { message } => Some(Ok(Outcome::Failed { message })),
            HermesEvent::Status { text } => Some(Ok(Outcome::Status { text })),
            // 4009 e envio concorrente: a UI oferece cancelar, nao repete.
            HermesEvent::Rejected { code: 4009, .. } => Some(Ok(Outcome::Busy)),
            HermesEvent::Rejected { code, message, .. } => Some(Ok(Outcome::Failed {
                message: format!("{message} ({code})"),
            })),
            HermesEvent::Unknown { kind } => {
                // Resposta a um request nosso chega como `result`, sem `params`,
                // entao nao tem tipo de evento — a correlacao e pelo id.
                if let Some((id, result)) = result_of(frame) {
                    match self.pending.remove(&id) {
                        Some(Pending::History) => {
                            return Some(Ok(Outcome::History {
                                messages: history_from(&result),
                            }))
                        }
                        Some(Pending::Title) => {
                            return Some(Ok(Outcome::Title {
                                title: title_from(&result),
                            }))
                        }
                        Some(Pending::Resume | Pending::Session) | None => {}
                    }
                }
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

/// Extrai `(id, result)` de uma resposta nossa. `None` quando o frame nao e
/// resposta — evento nao tem `result`.
fn result_of(frame: &str) -> Option<(i64, serde_json::Value)> {
    let value: serde_json::Value = serde_json::from_str(frame).ok()?;
    let id = value.get("id")?.as_i64()?;
    Some((id, value.get("result")?.clone()))
}

/// Projeta `result.messages` de `session.history`.
///
/// Tolerante por escolha: uma mensagem malformada e pulada em vez de derrubar a
/// reidratacao inteira. Perder uma linha do historico e ruim; perder a conversa
/// toda porque uma linha divergiu e pior.
fn history_from(result: &serde_json::Value) -> Vec<HistoryMessage> {
    let Some(messages) = result.get("messages").and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    messages
        .iter()
        .filter_map(|message| {
            let role = message.get("role")?.as_str()?.to_owned();
            let content = message
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned();
            let tool_name = message
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_owned();
            // Mensagem de ferramenta traz `context` no lugar de `content`.
            let content = if content.is_empty() {
                message
                    .get("context")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_owned()
            } else {
                content
            };
            Some(HistoryMessage {
                role,
                content,
                tool_name,
            })
        })
        .collect()
}

fn title_from(result: &serde_json::Value) -> String {
    result
        .get("title")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned()
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

        // `next()` nao devolve nada para o ready — ele apenas promove o estado e
        // segue lendo. Fechar o canal em seguida e o que faz a chamada retornar,
        // e o estado ja tem de estar Online quando isso acontece.
        push.send(Ok(READY.into())).await.unwrap();
        drop(push);
        assert!(bridge.next().await.is_none());
        assert_eq!(bridge.state(), ConnectionState::Offline);
    }

    /// A regressao que derrubava o Hermes inteiro.
    ///
    /// `gateway.ready` e o PRIMEIRO quadro da conexao e nao produz saida
    /// visivel. Enquanto `next()` devolvia `None` para ele, o laco da ponte no
    /// app — que trata `None` como queda de socket — encerrava a conexao no
    /// instante em que ela era confirmada, reconectava, e repetia para sempre.
    /// O chat nunca chegou a funcionar.
    ///
    /// Quadro sem saida visivel deve apenas continuar a leitura.
    #[tokio::test]
    async fn ready_does_not_end_the_stream() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(READY.into())).await.unwrap();
        push.send(Ok(SESSION.into())).await.unwrap();
        push.send(Ok(delta("oi"))).await.unwrap();

        // A primeira saida visivel e o delta: ready e o resultado da sessao sao
        // absorvidos no caminho, sem interromper a leitura.
        let outcome = bridge.next().await.expect("o socket nao pode ter fechado");
        assert!(matches!(outcome, Ok(Outcome::Delta { .. })));
        assert_eq!(bridge.state(), ConnectionState::Online);
    }

    #[tokio::test]
    async fn collects_the_session_id_from_the_create_result() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(SESSION.into())).await.unwrap();
        drop(push);
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
        // O ready nao retorna sozinho — ele promove e a leitura segue. Um delta
        // atras dele da a `next()` algo para devolver.
        push.send(Ok(READY.into())).await.unwrap();
        push.send(Ok(delta("x"))).await.unwrap();
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

    /// Uma sessao que sumiu da VPS nao pode deixar o Hermes inutilizavel: o
    /// resume rejeitado tem que virar um create, sozinho.
    #[tokio::test]
    async fn a_rejected_resume_falls_back_to_create() {
        let (channels, mut sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        bridge.open_session(Some("sessao-morta")).await.unwrap();
        let resume = sent.recv().await.unwrap();
        assert!(resume.contains(r#""method":"session.resume""#));

        // O gateway rejeita o resume com o mesmo id do request.
        push.send(Ok(
            r#"{"jsonrpc":"2.0","error":{"code":4004,"message":"unknown session"},"id":1}"#.into(),
        ))
        .await
        .unwrap();

        // A rejeicao e absorvida — nao vira Failed na cara do usuario...
        drop(push);
        assert!(bridge.next().await.is_none());
        // ...e uma sessao nova e aberta no lugar.
        let created = sent.recv().await.unwrap();
        assert!(created.contains(r#""method":"session.create""#));
    }

    /// Erro de frame nao fecha o socket, entao nao pode derrubar o estado: so
    /// `gateway.ready` promove para Online, e ele chega uma vez so.
    #[tokio::test]
    async fn a_frame_error_does_not_latch_the_connection_offline() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);
        // Ready e o erro numa chamada so: o ready e absorvido no caminho e a
        // leitura continua ate o primeiro quadro com saida visivel.
        push.send(Ok(READY.into())).await.unwrap();
        push.send(Err(HermesError::Protocol("frame binario".into())))
            .await
            .unwrap();
        assert!(bridge.next().await.unwrap().is_err());
        assert_eq!(
            bridge.state(),
            ConnectionState::Online,
            "o socket continua vivo, entao o estado tambem"
        );
    }

    /// O defeito que a auditoria de 2026-08-15 encontrou.
    ///
    /// `clarify.request` passa pelo mesmo `_block()` do approval: emite e trava
    /// a thread do agente por ate 300 s esperando `clarify.respond`. Enquanto
    /// caia em `Unknown`, a UI o descartava e a resposta congelava sem causa na
    /// tela. Ele tem que chegar a superficie com o `request_id`, porque sem ele
    /// a resposta nao correlaciona.
    #[tokio::test]
    async fn clarify_reaches_the_surface_with_its_request_id() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"clarify.request","payload":{"question":"qual projeto?","choices":["Minarum"],"request_id":"a1b2"}}}"#.into(),
        ))
        .await
        .unwrap();

        assert_eq!(
            bridge.next().await.unwrap().unwrap(),
            Outcome::Clarify {
                request_id: "a1b2".into(),
                question: "qual projeto?".into(),
                choices: vec!["Minarum".into()],
            }
        );
    }

    #[tokio::test]
    async fn responding_to_a_clarify_correlates_by_request_id() {
        let (channels, mut sent, _push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        bridge.respond_clarify("a1b2", "Minarum").await.unwrap();
        let frame = sent.recv().await.unwrap();
        assert!(frame.contains(r#""method":"clarify.respond""#));
        assert!(frame.contains(r#""request_id":"a1b2""#));
    }

    /// Sudo e decisao de politica: a ponte recusa sozinha, na hora.
    ///
    /// O M/OS nao tem senha de root da VPS para oferecer, e um campo de senha
    /// que aparece sozinho no meio de uma conversa tem a forma de um phishing.
    /// Esperar um clique do usuario para chegar na mesma recusa manteria o
    /// agente parado pelos 120 s do `_block`.
    #[tokio::test]
    async fn sudo_is_refused_by_the_bridge_without_asking_anyone() {
        let (channels, mut sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"sudo.request","payload":{"request_id":"s9"}}}"#.into(),
        ))
        .await
        .unwrap();

        assert_eq!(
            bridge.next().await.unwrap().unwrap(),
            Outcome::SudoRefused,
            "a recusa precisa chegar a superficie para poder ser explicada"
        );
        let frame = sent.recv().await.unwrap();
        assert!(frame.contains(r#""method":"sudo.respond""#));
        assert!(frame.contains(r#""password":"""#));
    }

    /// `status.update` era absorvido e perdido. E o texto que explica o que o
    /// agente faz entre um token e outro.
    #[tokio::test]
    async fn status_reaches_the_surface() {
        let (channels, _sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(
            r#"{"jsonrpc":"2.0","method":"event","params":{"type":"status.update","payload":{"text":"lendo 4 fontes"}}}"#.into(),
        ))
        .await
        .unwrap();
        assert_eq!(
            bridge.next().await.unwrap().unwrap(),
            Outcome::Status {
                text: "lendo 4 fontes".into()
            }
        );
    }

    /// Resposta nossa nao tem tipo de evento — a correlacao e pelo id do
    /// request. Sem isso o historico chegaria como quadro desconhecido.
    #[tokio::test]
    async fn history_is_correlated_by_the_request_id() {
        let (channels, mut sent, push) = fake_channels();
        let mut bridge = Bridge::new(channels);

        push.send(Ok(SESSION.into())).await.unwrap();
        push.send(Ok(delta("x"))).await.unwrap();
        bridge.next().await;

        bridge.request_history().await.unwrap();
        let asked = sent.recv().await.unwrap();
        assert!(asked.contains(r#""method":"session.history""#));
        let id = asked
            .split(r#""id":"#)
            .nth(1)
            .and_then(|rest| rest.split(',').next())
            .unwrap()
            .to_owned();

        push.send(Ok(format!(
            r#"{{"jsonrpc":"2.0","id":{id},"result":{{"count":2,"messages":[
                {{"role":"user","content":"oi"}},
                {{"role":"assistant","content":"ola"}}
            ]}}}}"#
        )))
        .await
        .unwrap();

        match bridge.next().await.unwrap().unwrap() {
            Outcome::History { messages } => {
                assert_eq!(messages.len(), 2);
                assert_eq!(messages[0].role, "user");
                assert_eq!(messages[1].content, "ola");
            }
            other => panic!("esperava History, veio {other:?}"),
        }
    }

    /// Uma linha divergente do historico nao pode custar a conversa inteira.
    #[tokio::test]
    async fn a_malformed_history_row_is_skipped_not_fatal() {
        let parsed = history_from(&serde_json::json!({
            "messages": [
                {"content": "sem papel"},
                {"role": "assistant", "content": "sobrevivi"}
            ]
        }));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].content, "sobrevivi");
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
