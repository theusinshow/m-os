//! Fronteira Tauri da ponte com o Hermes.
//!
//! A ponte vive numa task dedicada que possui o socket. Comandos chegam por um
//! canal; eventos saem por `emit`, reusando o mesmo caminho que o app ja usa
//! para `capture-changed`. Nenhuma chamada de rede em componente React, e
//! nenhuma credencial atravessa para o WebView.
//!
//! O que este arquivo NAO faz: modelar conversa. A traducao entre `Outcome` e
//! parte de mensagem mora em `jarvis.rs`, e a persistencia em `mos-core`. Aqui
//! ficam conexao, sessao e o encaminhamento.

use std::sync::{Arc, Mutex};

use mos_hermes::{Bridge, ConnectionState, Credentials, Gateway, HermesError, Outcome};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::mpsc;

use crate::jarvis::{self, ContextInput, TurnEvent, TurnRecorder};
use crate::AppState;
use mos_core::MessageStatus;

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:9119";

/// Ordens que a task da ponte aceita.
enum Order {
    Submit(String),
    Interrupt,
    Approve(bool),
    /// Responde a clarificacao. O `request_id` correlaciona; nao ha sessao no
    /// frame porque o servidor nao a consulta neste caminho.
    Clarify(String, String),
    /// Aponta a sessao para outra conversa. `None` cria; `Some` retoma.
    Switch(Option<String>),
    RequestHistory,
    AskTitle,
    Close,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HermesStatus {
    pub state: ConnectionState,
    pub has_credentials: bool,
    pub base_url: String,
    /// Mensagem legivel do ultimo erro. Vazia quando nao ha.
    pub detail: String,
    /// `Online` significa socket aceito; nao significa sessao aberta.
    ///
    /// O `gateway.ready` chega no aceite, mas o id da sessao so vem na resposta
    /// do `session.create` — sobre um tunel SSH ate uma VPS isso e uma janela
    /// de 100 a 300 ms em que a UI dizia ONLINE e aceitava pergunta que falhava
    /// com "Nenhuma sessao aberta". O campo separa as duas coisas.
    pub session_ready: bool,
    /// A conversa a que a sessao atual pertence. Vazia antes da primeira.
    pub conversation_id: String,
}

pub struct HermesState {
    orders: Mutex<Option<mpsc::Sender<Order>>>,
    connection: Arc<Mutex<ConnectionState>>,
    detail: Arc<Mutex<String>>,
    /// Id da sessao na VPS. Continua em memoria porque e cache do turno; a
    /// verdade durável vive na conversa, em `hermes_session_id` (ADR-025).
    session_id: Arc<Mutex<Option<String>>>,
    /// Conversa a que a sessao aberta pertence.
    conversation_id: Arc<Mutex<String>>,
    /// Turno em curso. `None` quando nao ha resposta chegando.
    recorder: Arc<Mutex<Option<TurnRecorder>>>,
    base_url: Mutex<String>,
}

impl Default for HermesState {
    fn default() -> Self {
        Self {
            orders: Mutex::new(None),
            connection: Arc::new(Mutex::new(ConnectionState::Offline)),
            detail: Arc::new(Mutex::new(String::new())),
            session_id: Arc::new(Mutex::new(None)),
            conversation_id: Arc::new(Mutex::new(String::new())),
            recorder: Arc::new(Mutex::new(None)),
            base_url: Mutex::new(DEFAULT_BASE_URL.to_owned()),
        }
    }
}

fn set<T>(slot: &Mutex<T>, value: T) {
    if let Ok(mut guard) = slot.lock() {
        *guard = value;
    }
}

fn get<T: Clone>(slot: &Mutex<T>) -> T {
    slot.lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|error| error.into_inner().clone())
}

fn status_of(state: &HermesState) -> HermesStatus {
    HermesStatus {
        state: get(&state.connection),
        has_credentials: Credentials::exist(),
        base_url: get(&state.base_url),
        detail: get(&state.detail),
        session_ready: get(&state.session_id).is_some(),
        conversation_id: get(&state.conversation_id),
    }
}

fn announce<R: Runtime>(app: &AppHandle<R>, state: &HermesState) {
    let _ = app.emit("hermes-state", status_of(state));
}

#[tauri::command]
pub fn hermes_status(state: State<'_, HermesState>) -> HermesStatus {
    status_of(&state)
}

/// Nunca devolve a senha. O renderer so aprende que existe credencial.
#[tauri::command]
pub fn hermes_set_credentials(username: String, password: String) -> Result<(), String> {
    Credentials::store(username.trim(), &password).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hermes_clear_credentials() -> Result<(), String> {
    Credentials::clear().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hermes_set_base_url(url: String, state: State<'_, HermesState>) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return Err("O endereco do Hermes deve comecar com http:// ou https://.".into());
    }
    let next = trimmed.trim_end_matches('/').to_owned();
    // Trocar de gateway invalida a sessao guardada: o id pertence ao gateway
    // anterior, e tentar retomar la seria pedir uma sessao que nunca existiu.
    if next != get(&state.base_url) {
        set(&state.session_id, None);
    }
    set(&state.base_url, next);
    Ok(())
}

/// Falha estruturada da conexao. O renderer precisa saber SE pode tentar de
/// novo, e antes disto so recebia a mensagem em texto — decidir por substring
/// seria decidir por acaso, num caminho onde errar significa martelar o login e
/// tomar 429.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HermesFailure {
    kind: String,
    message: String,
    retriable: bool,
}

impl From<mos_hermes::HermesError> for HermesFailure {
    fn from(error: mos_hermes::HermesError) -> Self {
        Self {
            kind: error.kind().to_owned(),
            message: error.to_string(),
            retriable: error.retriable(),
        }
    }
}

/// Fecha o turno em curso gravando o que chegou ate aqui.
///
/// Chamado quando o turno assenta, quando o socket cai e quando o app fecha. Um
/// texto recebido e nao gravado seria exatamente a promessa que a UX-PRINCIPLES
/// §52 proibe: "minha resposta sumiu?".
fn settle_turn<R: Runtime>(app: &AppHandle<R>, state: &HermesState, status: MessageStatus) {
    let Some(recorder) = state
        .recorder
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
    else {
        return;
    };
    if !recorder.has_content() && status == MessageStatus::Interrupted {
        // Nada chegou e o turno morreu: a mensagem vazia ainda precisa deixar de
        // dizer "pensando", mas nao ha parte para gravar.
        let service = app.state::<AppState>().conversations.clone();
        if let Ok(message) = service.finish_answer(&recorder.message_id, status, Vec::new()) {
            jarvis::announce_message(app, &message);
        }
        return;
    }

    let message_id = recorder.message_id.clone();
    let parts = recorder.into_parts(status);
    let service = app.state::<AppState>().conversations.clone();
    if let Ok(message) = service.finish_answer(&message_id, status, parts) {
        jarvis::announce_message(app, &message);
    }
}

/// Conexao preguicosa na origem: acontecia so quando o usuario entrava no modo
/// Hermes pela primeira vez. Continua barata — tunel morto nao atrasa o boot —,
/// mas agora quem chama tambem pode ser o supervisor de reconexao do renderer.
#[tauri::command]
pub async fn hermes_connect<R: Runtime>(app: AppHandle<R>) -> Result<(), HermesFailure> {
    let state = app.state::<HermesState>();
    // Curto-circuito tambem em Connecting, nao so em Online.
    //
    // Sem isso, duas chamadas concorrentes rodavam dois handshakes e criavam
    // duas tasks de pump; a segunda sobrescrevia o sender da primeira, que
    // ficava orfa mas continuava emitindo — o usuario veria cada token duas
    // vezes, e duas sessoes seriam abertas no servidor.
    if matches!(
        get(&state.connection),
        ConnectionState::Online | ConnectionState::Connecting
    ) {
        return Ok(());
    }

    let base_url = get(&state.base_url);
    let connection = state.connection.clone();
    let detail = state.detail.clone();
    let session_slot = state.session_id.clone();
    let conversation_slot = state.conversation_id.clone();

    // A conversa corrente decide qual sessao retomar. E o vinculo que a ADR-025
    // colocou em disco: antes dele o resume nunca acontecia entre aberturas.
    let (conversation_id, resume) = {
        let service = app.state::<AppState>().conversations.clone();
        match service.current_or_new() {
            Ok(conversation) => (
                conversation.id.to_string(),
                conversation.hermes_session_id.clone(),
            ),
            Err(_) => (String::new(), None),
        }
    };
    set(&conversation_slot, conversation_id);

    set(&connection, ConnectionState::Connecting);
    set(&detail, String::new());
    announce(&app, &state);

    let channels = match handshake(&base_url).await {
        Ok(channels) => channels,
        Err(error) => {
            set(&connection, ConnectionState::Offline);
            set(&detail, error.to_string());
            announce(&app, &state);
            return Err(error.into());
        }
    };

    let (order_tx, mut order_rx) = mpsc::channel::<Order>(16);
    set(&state.orders, Some(order_tx));

    let pump = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut bridge = Bridge::new(channels);
        let mut last_state = ConnectionState::Connecting;
        let mut last_session: Option<String> = None;
        if let Err(error) = bridge.open_session(resume.as_deref()).await {
            set(&detail, error.to_string());
        }

        loop {
            tokio::select! {
                order = order_rx.recv() => {
                    let outcome = match order {
                        Some(Order::Submit(text)) => bridge.submit(&text).await,
                        Some(Order::Interrupt) => bridge.interrupt().await,
                        Some(Order::Approve(approved)) => bridge.respond_approval(approved).await,
                        Some(Order::Clarify(request_id, answer)) => {
                            bridge.respond_clarify(&request_id, &answer).await
                        }
                        Some(Order::Switch(resume)) => {
                            // A sessao anterior deixa de valer neste instante,
                            // e nao no momento em que a nova responde: aceitar
                            // envio no meio mandaria a pergunta para a conversa
                            // errada.
                            last_session = None;
                            set(&session_slot, None);
                            bridge.open_session(resume.as_deref()).await
                        }
                        Some(Order::RequestHistory) => bridge.request_history().await,
                        Some(Order::AskTitle) => bridge.set_title(None).await,
                        Some(Order::Close) | None => {
                            let _ = bridge.close().await;
                            break;
                        }
                    };
                    if let Err(error) = outcome {
                        set(&detail, error.to_string());
                        let _ = pump.emit("hermes-event", Outcome::Failed { message: error.to_string() });
                    }
                }
                event = bridge.next() => {
                    match event {
                        Some(Ok(outcome)) => {
                            let state = pump.state::<HermesState>();
                            match &outcome {
                                Outcome::History { messages } => {
                                    jarvis::absorb_history(&pump, &get(&state.conversation_id), messages);
                                }
                                Outcome::Title { title } => {
                                    jarvis::absorb_title(&pump, &get(&state.conversation_id), title);
                                }
                                _ => {
                                    let settled = state
                                        .recorder
                                        .lock()
                                        .ok()
                                        .and_then(|mut guard| {
                                            guard.as_mut().map(|recorder| {
                                                (
                                                    recorder.conversation_id.clone(),
                                                    recorder.message_id.clone(),
                                                    recorder.absorb(&outcome),
                                                )
                                            })
                                        });
                                    if let Some((conversation_id, message_id, settled)) = settled {
                                        let _ = pump.emit(
                                            "hermes-event",
                                            TurnEvent {
                                                conversation_id,
                                                message_id,
                                                outcome: outcome.clone(),
                                            },
                                        );
                                        if settled {
                                            let status = if matches!(outcome, Outcome::Failed { .. }) {
                                                MessageStatus::Failed
                                            } else {
                                                MessageStatus::Complete
                                            };
                                            settle_turn(&pump, &state, status);
                                            // O titulo so existe depois do
                                            // primeiro turno. Perguntar antes
                                            // devolveria vazio.
                                            if let Ok(guard) = state.orders.lock() {
                                                if let Some(sender) = guard.clone() {
                                                    let _ = sender.try_send(Order::AskTitle);
                                                }
                                            }
                                        }
                                    } else {
                                        // Sem turno em curso: quadro solto, que
                                        // ainda merece chegar a superficie.
                                        let _ = pump.emit("hermes-event", outcome.clone());
                                    }
                                }
                            }
                        }
                        Some(Err(error)) => {
                            set(&detail, error.to_string());
                            let _ = pump.emit("hermes-event", Outcome::Failed { message: error.to_string() });
                        }
                        // Socket seco: queda de conexao, nao fim de turno.
                        None => break,
                    }
                }
            }

            // Anunciar so quando algo mudou de verdade.
            //
            // Antes isto rodava a cada frame recebido, ou seja, uma vez por
            // token da resposta. Cada volta fazia uma leitura bloqueante do
            // Credential Manager (Credentials::exist) de dentro da task async,
            // mais um emit que re-renderizava a superficie inteira. Uma resposta
            // longa produzia milhares de acessos ao cofre.
            let next_state = bridge.state();
            let next_session = bridge.session_id().map(str::to_owned);
            if next_state != last_state || next_session != last_session {
                last_state = next_state;
                last_session.clone_from(&next_session);
                set(&connection, next_state);
                if let Some(session_id) = next_session.clone() {
                    set(&session_slot, Some(session_id.clone()));
                    // O vinculo vai para o disco assim que existe. Guardar isto
                    // so na memoria era o defeito que deixava session.resume
                    // morto entre aberturas do app.
                    let conversation_id = get(&conversation_slot);
                    if !conversation_id.is_empty() {
                        let service = pump.state::<AppState>().conversations.clone();
                        let _ = service.bind_session(&conversation_id, Some(&session_id));
                    }
                }
                let state = pump.state::<HermesState>();
                announce(&pump, &state);
            }
        }

        // O socket caiu ou o app fechou. O texto que ja chegou nao pode sumir.
        let state = pump.state::<HermesState>();
        settle_turn(&pump, &state, MessageStatus::Interrupted);
        set(&connection, ConnectionState::Offline);
        announce(&pump, &state);
    });

    Ok(())
}

/// A ordem do contrato, sem atalho: status, login, ticket, socket.
async fn handshake(base_url: &str) -> Result<mos_hermes::Channels, HermesError> {
    let gateway = Gateway::new(base_url)?;
    let status = gateway.status().await?;

    if status.auth_required {
        let credentials = Credentials::load()?;
        let provider = status
            .auth_providers
            .first()
            .cloned()
            .unwrap_or_else(|| "basic".to_owned());
        gateway.login(&credentials, &provider).await?;
    }

    // Cunhado agora porque vale 30 segundos.
    let ticket = gateway.mint_ticket().await?;
    mos_hermes::connect(&gateway.websocket_url(&ticket)).await
}

async fn order<R: Runtime>(app: &AppHandle<R>, order: Order) -> Result<(), String> {
    let sender = {
        let state = app.state::<HermesState>();
        let guard = state.orders.lock().map_err(|_| "estado interno travado")?;
        guard.clone()
    };
    match sender {
        Some(sender) => sender
            .send(order)
            .await
            .map_err(|_| "A conexao com o Hermes caiu.".to_owned()),
        None => Err("O Hermes ainda nao esta conectado.".to_owned()),
    }
}

/// Envia uma pergunta, com o contexto que o usuario anexou.
///
/// A ordem importa e nao e acidental: grava a pergunta, monta o contexto, grava
/// o registro do que sera enviado, abre a resposta e so entao submete. Se
/// qualquer passo falhar, nada saiu para a VPS — e o registro do que saiu nunca
/// fica atras do que efetivamente saiu (ADR-027).
#[tauri::command]
pub async fn hermes_send<R: Runtime>(
    app: AppHandle<R>,
    conversation_id: String,
    text: String,
    contexts: Vec<ContextInput>,
) -> Result<(), String> {
    let service = app.state::<AppState>().conversations.clone();

    let question = service
        .append_user_message(&conversation_id, &text)
        .map_err(|error| error.message.clone())?;

    let assembled =
        jarvis::assemble_context(&app, &contexts).map_err(|error| error.message.clone())?;

    if !assembled.parts.is_empty() {
        let mut parts = vec![mos_core::PartBody::Text { text: text.clone() }];
        parts.extend(assembled.parts);
        let question = service
            .attach_parts(&question.id.to_string(), MessageStatus::Complete, parts)
            .map_err(|error| error.message.clone())?;
        jarvis::announce_message(&app, &question);
    } else {
        jarvis::announce_message(&app, &question);
    }

    let answer = service
        .start_answer(&conversation_id)
        .map_err(|error| error.message.clone())?;
    jarvis::announce_message(&app, &answer);

    {
        let state = app.state::<HermesState>();
        set(
            &state.recorder,
            Some(TurnRecorder::start(
                conversation_id.clone(),
                answer.id.to_string(),
            )),
        );
    }

    // O contrato de acoes desce em toda mensagem, antes do contexto. Ele diz o
    // que existe e como propor — e diz explicitamente que o modelo nao executa
    // nada. Sem essa ultima frase, um modelo prestativo tende a preencher campo
    // que o usuario nao disse.
    //
    // Ele sai da maquina junto com o resto do prompt: nomes de acao e forma de
    // argumento vao para a VPS. Nao sao dados pessoais, mas sao um mapa do que o
    // sistema sabe fazer, e isso esta registrado na spec.
    let prompt = format!("{}{}{}", mos_core::action_contract(), assembled.block, text);
    let sent = order(&app, Order::Submit(prompt)).await;
    if sent.is_err() {
        // Falhou antes de sair: a resposta aberta precisa parar de dizer
        // "pensando" em vez de ficar pendurada para sempre.
        let state = app.state::<HermesState>();
        settle_turn(&app, &state, MessageStatus::Failed);
    }
    sent
}

#[tauri::command]
pub async fn hermes_interrupt<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let result = order(&app, Order::Interrupt).await;
    // Cancelar grava o que chegou. O contrario seria perder a resposta parcial
    // exatamente no momento em que o usuario decidiu ficar com ela.
    let state = app.state::<HermesState>();
    settle_turn(&app, &state, MessageStatus::Interrupted);
    result
}

/// Fechar sem escolher nega. O servidor tambem tem `deny` como default, e
/// aprovar por omissao seria o pior erro possivel neste caminho.
#[tauri::command]
pub async fn hermes_approve<R: Runtime>(app: AppHandle<R>, approved: bool) -> Result<(), String> {
    order(&app, Order::Approve(approved)).await
}

/// Responde a clarificacao. Sem isto o agente fica travado por cinco minutos.
#[tauri::command]
pub async fn hermes_clarify<R: Runtime>(
    app: AppHandle<R>,
    request_id: String,
    answer: String,
) -> Result<(), String> {
    order(&app, Order::Clarify(request_id, answer)).await
}

/// Aponta a sessao do Hermes para outra conversa.
#[tauri::command]
pub async fn hermes_select_conversation<R: Runtime>(
    app: AppHandle<R>,
    conversation_id: String,
) -> Result<(), String> {
    let service = app.state::<AppState>().conversations.clone();
    let conversation = service
        .get(&conversation_id)
        .map_err(|error| error.message.clone())?;

    {
        let state = app.state::<HermesState>();
        // Um turno da conversa anterior nao pode continuar gravando na nova.
        settle_turn(&app, &state, MessageStatus::Interrupted);
        set(&state.conversation_id, conversation_id.clone());
        set(&state.session_id, None);
        announce(&app, &state);
    }

    order(&app, Order::Switch(conversation.hermes_session_id)).await
}

/// Pede o historico que a VPS ja tem desta sessao.
#[tauri::command]
pub async fn hermes_load_history<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    order(&app, Order::RequestHistory).await
}

#[tauri::command]
pub async fn hermes_disconnect<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    order(&app, Order::Close).await
}
