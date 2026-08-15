//! Fronteira Tauri da ponte com o Hermes.
//!
//! A ponte vive numa task dedicada que possui o socket. Comandos chegam por um
//! canal; eventos saem por `emit`, reusando o mesmo caminho que o app ja usa
//! para `capture-changed`. Nenhuma chamada de rede em componente React, e
//! nenhuma credencial atravessa para o WebView.

use std::sync::{Arc, Mutex};

use mos_hermes::{Bridge, ConnectionState, Credentials, Gateway, HermesError, Outcome};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::mpsc;

pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:9119";

/// Ordens que a task da ponte aceita.
enum Order {
    Submit(String),
    Interrupt,
    Approve(bool),
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
}

pub struct HermesState {
    orders: Mutex<Option<mpsc::Sender<Order>>>,
    connection: Arc<Mutex<ConnectionState>>,
    detail: Arc<Mutex<String>>,
    /// Guardado localmente para `session.resume` na proxima abertura. O
    /// historico nao vem junto: ele vive no state.db da VPS.
    session_id: Arc<Mutex<Option<String>>>,
    base_url: Mutex<String>,
}

impl Default for HermesState {
    fn default() -> Self {
        Self {
            orders: Mutex::new(None),
            connection: Arc::new(Mutex::new(ConnectionState::Offline)),
            detail: Arc::new(Mutex::new(String::new())),
            session_id: Arc::new(Mutex::new(None)),
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

fn announce<R: Runtime>(app: &AppHandle<R>, state: &HermesState) {
    let status = HermesStatus {
        state: get(&state.connection),
        has_credentials: Credentials::exist(),
        base_url: get(&state.base_url),
        detail: get(&state.detail),
        session_ready: get(&state.session_id).is_some(),
    };
    let _ = app.emit("hermes-state", status);
}

#[tauri::command]
pub fn hermes_status(state: State<'_, HermesState>) -> HermesStatus {
    HermesStatus {
        state: get(&state.connection),
        has_credentials: Credentials::exist(),
        base_url: get(&state.base_url),
        detail: get(&state.detail),
        session_ready: get(&state.session_id).is_some(),
    }
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
    let resume = get(&state.session_id);
    let connection = state.connection.clone();
    let detail = state.detail.clone();
    let session_slot = state.session_id.clone();

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
                        Some(Ok(outcome)) => { let _ = pump.emit("hermes-event", outcome); }
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
                if next_session.is_some() {
                    set(&session_slot, next_session);
                }
                let state = pump.state::<HermesState>();
                announce(&pump, &state);
            }
        }

        set(&connection, ConnectionState::Offline);
        let state = pump.state::<HermesState>();
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

#[tauri::command]
pub async fn hermes_send<R: Runtime>(app: AppHandle<R>, text: String) -> Result<(), String> {
    order(&app, Order::Submit(text)).await
}

#[tauri::command]
pub async fn hermes_interrupt<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    order(&app, Order::Interrupt).await
}

/// Fechar sem escolher nega. O servidor tambem tem `deny` como default, e
/// aprovar por omissao seria o pior erro possivel neste caminho.
#[tauri::command]
pub async fn hermes_approve<R: Runtime>(app: AppHandle<R>, approved: bool) -> Result<(), String> {
    order(&app, Order::Approve(approved)).await
}

#[tauri::command]
pub async fn hermes_disconnect<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    order(&app, Order::Close).await
}
