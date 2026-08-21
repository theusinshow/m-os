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
    /// Quantas buscas extras ainda cabem na pergunta em curso.
    ///
    /// Zera a cada `hermes_send` e nao entre turnos: o orcamento e da PERGUNTA,
    /// e nao da sessao. Sem isso, uma conversa longa acumularia saltos e uma
    /// pergunta trivial poderia disparar uma cadeia de buscas herdada das
    /// anteriores.
    query_hops: Arc<Mutex<u8>>,
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
            query_hops: Arc::new(Mutex::new(0)),
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

/// Fecha o turno em curso gravando o que chegou ate aqui, e devolve a busca que
/// o modelo pediu, se pediu.
///
/// Chamado quando o turno assenta, quando o socket cai e quando o app fecha. Um
/// texto recebido e nao gravado seria exatamente a promessa que a UX-PRINCIPLES
/// §52 proibe: "minha resposta sumiu?".
///
/// O retorno existe por causa do salto de busca: quem chama precisa saber que o
/// modelo pediu uma, e so pode saber depois de o turno assentar — mandar outro
/// `prompt.submit` antes disso levaria um `4009 session busy`.
fn settle_turn<R: Runtime>(
    app: &AppHandle<R>,
    state: &HermesState,
    status: MessageStatus,
) -> Option<String> {
    let recorder = state
        .recorder
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())?;
    if !recorder.has_content() && status == MessageStatus::Interrupted {
        // Nada chegou e o turno morreu: a mensagem vazia ainda precisa deixar de
        // dizer "pensando", mas nao ha parte para gravar.
        let service = app.state::<AppState>().conversations.clone();
        if let Ok(message) = service.finish_answer(&recorder.message_id, status, Vec::new()) {
            jarvis::announce_message(app, &message);
        }
        return None;
    }

    let message_id = recorder.message_id.clone();
    // A busca so vale se o turno chegou inteiro. Uma cerca aberta por
    // interrupcao produziria um pedido pela metade, e responde-lo seria
    // continuar uma frase que o usuario mandou parar.
    let query = (status == MessageStatus::Complete)
        .then(|| recorder.requested_query())
        .flatten();
    let parts = recorder.into_parts(status, crate::surface::now_local(app));
    let service = app.state::<AppState>().conversations.clone();
    if let Ok(message) = service.finish_answer(&message_id, status, parts) {
        jarvis::announce_message(app, &message);
    }
    query
}

/// Executa a busca que o modelo pediu e devolve o resultado a ele.
///
/// # Por que isto nao viola a ADR-028
///
/// A ADR escolheu injecao de contexto e adiou o MCP local, porque expor um
/// servidor da maquina a VPS muda a superficie de ataque. Nada disso acontece
/// aqui: **o M/OS continua sendo quem fala primeiro.** O agente pediu por
/// escrito, num bloco de texto que ja atravessou o canal existente; o M/OS leu,
/// pesquisou na propria base e mandou outro prompt pelo mesmo socket. Nenhuma
/// porta nova, nenhuma inversao de tunel, nenhum dado saindo sem o M/OS
/// escolher o que sai.
///
/// O que muda e o numero de idas, e so isso e o custo: o turno inteiro roda
/// duas vezes.
async fn answer_query<R: Runtime>(app: &AppHandle<R>, raw: &str) {
    // O pedido e lido ANTES de gastar o salto. Um bloco ilegivel ja virou uma
    // execucao com erro na thread, dentro de `into_parts`; consumir o orcamento
    // por causa dele tiraria a busca boa que viesse depois.
    let Ok(request) = mos_core::parse_query(raw) else {
        return;
    };

    let state = app.state::<HermesState>();
    {
        let Ok(mut guard) = state.query_hops.lock() else {
            return;
        };
        if *guard == 0 {
            return;
        }
        *guard -= 1;
    }

    let candidatos = jarvis::run_query(app, &request);
    let conversation_id = get(&state.conversation_id);
    let service = app.state::<AppState>().conversations.clone();

    // A resposta abre uma mensagem NOVA do assistente, e nao continua a
    // anterior: aquela ja foi gravada com o pedido de busca dentro. Duas
    // mensagens tambem sao a leitura honesta do que aconteceu — houve dois
    // turnos, e a thread mostra os dois.
    let Ok(answer) = service.start_answer(&conversation_id) else {
        return;
    };
    jarvis::announce_message(app, &answer);

    // O que a busca devolveu atravessa a ponte AGORA, antes de o modelo falar.
    // A ADR-027 pede o registro do que foi enviado, e a mensagem anterior ja
    // foi gravada — entao ele nasce com este turno, como o passo que ele e.
    let mut recorder = TurnRecorder::start(conversation_id.clone(), answer.id.to_string());
    recorder.seed(mos_core::PartBody::ToolRun {
        name: "Busca no M/OS".to_owned(),
        state: mos_core::ToolRunState::Success,
        detail: if candidatos.is_empty() {
            format!("\"{}\" — nada encontrado", request.search)
        } else {
            format!(
                "\"{}\" — enviados: {}",
                request.search,
                candidatos
                    .iter()
                    .map(|candidate| candidate.label.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    });
    set(&state.recorder, Some(recorder));

    // O preambulo NAO desce de novo. A sessao do gateway guarda o historico do
    // turno anterior, e ele ja carrega a identidade, o relogio e o catalogo —
    // repetir tudo dobraria o custo do salto para reafirmar o que o modelo
    // acabou de ler.
    let prompt = mos_core::query_answer(&request, &candidatos);
    if order(app, Order::Submit(prompt)).await.is_err() {
        // A resposta aberta precisa parar de dizer "pensando": o pedido de
        // busca ja foi gravado, e a mensagem que ia responde-lo ficaria
        // pendurada para sempre.
        let _ = settle_turn(app, &state, MessageStatus::Failed);
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
                                            let query = settle_turn(&pump, &state, status);
                                            // O titulo so existe depois do
                                            // primeiro turno. Perguntar antes
                                            // devolveria vazio.
                                            if let Ok(guard) = state.orders.lock() {
                                                if let Some(sender) = guard.clone() {
                                                    let _ = sender.try_send(Order::AskTitle);
                                                }
                                            }
                                            // O segundo salto sai numa task
                                            // propria: esperar por ele aqui
                                            // travaria o laco que le o socket,
                                            // e a resposta que ele mesmo espera
                                            // chega por este laco.
                                            if let Some(raw) = query {
                                                let outra = pump.clone();
                                                tauri::async_runtime::spawn(async move {
                                                    answer_query(&outra, &raw).await;
                                                });
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
        let _ = settle_turn(&pump, &state, MessageStatus::Interrupted);
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

    // O que o M/OS descobre sozinho, antes de enviar.
    //
    // Isto e o coracao da mudanca. Ate aqui, o unico contexto que atravessava a
    // ponte era o que o usuario anexava a mao com `@` — e quem escreve "a task
    // do Victor" nao anexa nada. O agente recebia a frase e nao tinha como
    // saber se aquela Task existe, entao ou perguntava ou criava uma segunda.
    //
    // Agora o M/OS le a propria frase, procura no FTS local e manda os
    // candidatos junto. E a mesma ADR-028 (injecao de contexto), com o M/OS
    // deixando de esperar que o usuario faca a busca por ele.
    let here = crate::surface::here(&app);
    let candidates = jarvis::candidates_for(&app, &text);
    let now_local = crate::surface::now_local(&app);
    let hops = mos_core::MAX_QUERY_HOPS;
    set(&app.state::<HermesState>().query_hops, hops);

    // O registro do que saiu (ADR-027). Um chip por candidato seria honesto e
    // ilegivel — doze chips numa mensagem sem anexo nenhum esconderiam os
    // anexos de verdade —, entao a busca inteira vira UMA parte, com os nomes
    // do que foi dentro de `fields`.
    let mut automatic = Vec::new();
    let here_block = mos_core::here_block(&here);
    if !here_block.is_empty() {
        automatic.push(mos_core::PartBody::ContextRef {
            origin: mos_core::ContextOrigin::Automatic,
            entity: mos_core::ContextEntity::Screen,
            id: String::new(),
            label: if here.screen.is_empty() {
                "tela atual".to_owned()
            } else {
                here.screen.clone()
            },
            fields: [
                here.project.as_ref().map(|named| named.label.clone()),
                here.task.as_ref().map(|named| named.label.clone()),
                here.workspace.as_ref().map(|named| named.label.clone()),
            ]
            .into_iter()
            .flatten()
            .collect(),
            bytes: here_block.len(),
        });
    }
    let candidates_block = mos_core::candidates_block(&candidates);
    if !candidates_block.is_empty() {
        automatic.push(mos_core::PartBody::ContextRef {
            origin: mos_core::ContextOrigin::Automatic,
            entity: mos_core::ContextEntity::Search,
            id: String::new(),
            label: format!("{} encontradas no M/OS", candidates.len()),
            fields: candidates
                .iter()
                .map(|candidate| candidate.label.clone())
                .collect(),
            bytes: candidates_block.len(),
        });
    }

    if !assembled.parts.is_empty() || !automatic.is_empty() {
        let mut parts = vec![mos_core::PartBody::Text { text: text.clone() }];
        parts.extend(automatic);
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

    // O preambulo desce em toda mensagem, antes do contexto: quem ele e, que
    // horas sao, onde o usuario esta, o que ele pode propor e o que o M/OS ja
    // encontrou. O contrato de acoes continua dizendo explicitamente que o
    // modelo nao executa nada — sem essa frase, um modelo prestativo tende a
    // preencher campo que o usuario nao disse.
    //
    // Ele sai da maquina junto com o resto do prompt: nomes de acao e forma de
    // argumento vao para a VPS. Nao sao dados pessoais, mas sao um mapa do que o
    // sistema sabe fazer, e isso esta registrado na spec.
    //
    // So desce no catalogo quando o App que aponta para o M-Finance tem
    // can_write marcado no Registry — a mesma capacidade que ja existia, so
    // passando a ter efeito real pela primeira vez (SPEC-ACOES-ENTRE-APPS.md).
    //
    // A busca e pelo ALVO e nao pelo id: `AppId` e um UUID sorteado no
    // cadastro, entao `app("m-finance")` nunca casava com nada e o gate
    // ficava fechado para sempre, marcasse o usuario o que marcasse.
    let finance_enabled = app
        .state::<AppState>()
        .apps
        .apps(false)
        .ok()
        .and_then(|apps| {
            mos_core::app_targeting_host(&apps, crate::finance::ACTION_HOST)
                .map(|entry| entry.can_write)
        })
        .unwrap_or(false);
    // A ordem do preambulo esta em `mos_core::preamble`, e o contexto ANEXADO
    // vem depois dele: o que o usuario escolheu mandar fica colado na pergunta,
    // que e onde ele espera que esteja.
    let prompt = format!(
        "{}{}{}",
        mos_core::preamble(mos_core::PreambleInput {
            now_local,
            here: &here,
            candidates: &candidates,
            finance_enabled,
            hops_left: hops,
            today: crate::daily::bloco_de_hoje(&app),
        }),
        assembled.block,
        text
    );
    let sent = order(&app, Order::Submit(prompt)).await;
    if sent.is_err() {
        // Falhou antes de sair: a resposta aberta precisa parar de dizer
        // "pensando" em vez de ficar pendurada para sempre.
        let state = app.state::<HermesState>();
        let _ = settle_turn(&app, &state, MessageStatus::Failed);
    }
    sent
}

#[tauri::command]
pub async fn hermes_interrupt<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let result = order(&app, Order::Interrupt).await;
    // Cancelar grava o que chegou. O contrario seria perder a resposta parcial
    // exatamente no momento em que o usuario decidiu ficar com ela.
    let state = app.state::<HermesState>();
    let _ = settle_turn(&app, &state, MessageStatus::Interrupted);
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

/// `Esc` sobre uma pergunta do Hermes: desiste de responder.
///
/// Responder e obrigatorio ANTES de interromper. O `_block()` do gateway so
/// solta a thread do agente quando `clarify.respond` chega, e
/// `session.interrupt` resolve as aprovacoes pendentes — nao as perguntas
/// (`protocol.rs:220`). Fechar apenas a caixa na tela, que era o que a UI fazia,
/// deixava o agente travado ate o teto de 300 s com a unica interface de
/// resposta ja fora do alcance: pensamento infinito sem saida.
///
/// Vive no Rust, e nao como duas chamadas encadeadas no renderer, porque as duas
/// ordens precisam entrar na fila nesta ordem — e porque o turno tem de assentar
/// mesmo quando a primeira falha.
#[tauri::command]
pub async fn hermes_clarify_cancel<R: Runtime>(
    app: AppHandle<R>,
    request_id: String,
) -> Result<(), String> {
    let answered = order(&app, Order::Clarify(request_id, String::new())).await;
    let interrupted = order(&app, Order::Interrupt).await;
    // Assenta mesmo se o gateway caiu no meio: e exatamente quando a resposta
    // nao chega que a tela nao pode continuar dizendo "pensando".
    let state = app.state::<HermesState>();
    let _ = settle_turn(&app, &state, MessageStatus::Interrupted);
    answered.and(interrupted)
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
        let _ = settle_turn(&app, &state, MessageStatus::Interrupted);
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

/// A `base_url` configurada agora.
///
/// Existe para quem precisa falar com o gateway FORA da conexao da conversa —
/// hoje, a analise de reuniao. Ler daqui em vez de duplicar o padrao garante que
/// trocar o endereco em Settings valha para os dois caminhos.
pub fn current_base_url<R: Runtime>(app: &AppHandle<R>) -> String {
    get(&app.state::<HermesState>().base_url)
}

/// Uma pergunta ao Hermes fora da conversa.
///
/// **A analise nao e uma conversa** (`MEETING-AGENT.md` §11.2). Despejar uma
/// transcricao de uma hora na thread do usuario seria ruido, e a resposta — um
/// bloco estruturado — nao e feita para ser lida como prosa.
///
/// Ela abre socket e sessao proprios, faz UMA pergunta, junta a resposta e
/// fecha. Nao toca o `HermesState`, entao uma analise em curso nao interfere na
/// conversa aberta e vice-versa.
pub async fn ask_once(base_url: &str, prompt: &str) -> Result<String, HermesError> {
    use tokio::time::{interval, timeout, Duration};

    // Uma reuniao de uma hora e um prompt grande, e o modelo pensa antes de
    // responder. Cinco minutos e generoso; sem teto nenhum, um turno travado na
    // VPS deixaria a reuniao em `analyzing` para sempre.
    const TETO: Duration = Duration::from_secs(300);

    let channels = handshake(base_url).await?;
    let mut bridge = Bridge::new(channels);

    // A sessao e aberta IMEDIATAMENTE, como o laco da conversa faz — e nao
    // depois de esperar `gateway.ready`.
    //
    // A primeira versao esperava, e travava. `gateway.ready` e o `result` do
    // `session.create` sao os dois absorvidos por `Bridge::absorb` sem produzir
    // saida visivel, e `next()` continua lendo em vez de devolver. Quem espera
    // por eles fica bloqueado num socket que nao vai mandar mais nada, porque a
    // proxima coisa a acontecer depende de NOS enviarmos a pergunta.
    bridge.open_session(None).await?;

    let mut resposta = String::new();
    let mut enviado = false;
    // O relogio que devolve o controle para perguntar se a sessao ja abriu. E o
    // mesmo mecanismo que o canal de ordens cumpre no laco da conversa: sem uma
    // segunda fonte de acordar, `next()` monopoliza o laco.
    let mut relogio = interval(Duration::from_millis(50));

    let colheita = timeout(TETO, async {
        loop {
            tokio::select! {
                event = bridge.next() => match event {
                    Some(Ok(Outcome::Delta { text })) => resposta.push_str(&text),
                    Some(Ok(Outcome::Complete)) => return Ok(()),
                    Some(Ok(Outcome::Failed { message })) => {
                        return Err(HermesError::Gateway(message))
                    }
                    // O agente pediu aprovacao no meio. Numa sessao efemera nao
                    // ha ninguem para responder, e esperar travaria a analise
                    // ate o teto. Negar e a resposta honesta, e e a mesma
                    // omissao que o `HERMES-GATEWAY-CONTRACT.md` §6 fixou.
                    Some(Ok(Outcome::Approval { .. })) => {
                        bridge.respond_approval(false).await?;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => return Err(error),
                    None => {
                        return Err(HermesError::Unreachable(
                            "o gateway fechou no meio da analise".into(),
                        ))
                    }
                },
                _ = relogio.tick(), if !enviado => {
                    if bridge.session_id().is_some() {
                        bridge.submit(prompt).await?;
                        enviado = true;
                    }
                }
            }
        }
    })
    .await;

    // Fechar em qualquer desfecho: uma sessao efemera que nao fecha vira lixo na
    // VPS, e elas se acumulam a cada reuniao.
    let _ = bridge.close().await;

    match colheita {
        Ok(Ok(())) => Ok(resposta),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(HermesError::Unreachable(
            "o Hermes nao respondeu a tempo".into(),
        )),
    }
}

#[cfg(test)]
mod gate_d {
    //! Prova o Gate D contra o Hermes REAL.
    //!
    //! `#[ignore]` porque depende de um tunel aberto e da credencial no
    //! Credential Manager. Ele nao pede senha e nao a le: usa
    //! `Credentials::load()`, exatamente o caminho que o aplicativo usa.
    //!
    //! ```powershell
    //! cargo test -p mos-desktop --lib gate_d -- --ignored --nocapture
    //! ```

    use mos_core::{interleave, MeetingId, RawSegment};

    /// Uma reuniao curta e realista, com compromissos dos dois lados.
    fn transcricao() -> Vec<mos_core::TranscriptSegment> {
        let fala = |start: i64, text: &str| RawSegment {
            start_ms: start,
            end_ms: start + 4_000,
            text: text.into(),
            confidence: None,
        };
        interleave(
            MeetingId::new(),
            vec![
                fala(0, "Bom dia pessoal, vamos comecar o alinhamento do NexoDoc."),
                fala(5_000, "Eu termino os slides da apresentacao amanha de manha e mando para voces."),
                fala(11_000, "Sobre o orcamento, acho que precisamos revisar os valores antes de fechar."),
                fala(17_000, "Ficou decidido entao que usamos o Hermes para a camada de inteligencia."),
                fala(23_000, "Eu fico responsavel por falar com o cliente ainda esta semana."),
            ],
            vec![
                fala(1_000, "Perfeito, bom dia. Eu revisei o documento ontem a noite."),
                fala(6_500, "Combinado, eu reviso os slides na sexta-feira pela manha."),
                fala(12_500, "Concordo com a revisao do orcamento. Eu levanto os numeros do trimestre."),
                fala(19_000, "Uma duvida que ficou em aberto e se o prazo de entrega continua o mesmo."),
                fala(25_000, "Talvez a gente possa antecipar a proxima reuniao, mas nao tenho certeza."),
            ],
        )
    }

    #[tokio::test]
    #[ignore = "precisa do tunel do Hermes aberto e da credencial salva"]
    async fn o_hermes_devolve_um_bloco_que_o_dominio_aceita() {
        let segments = transcricao();
        let meeting_id = segments[0].meeting_id;
        let windows = mos_core::build_windows(&segments, mos_core::WINDOW_BUDGET_CHARS);
        assert_eq!(windows.len(), 1, "esta transcricao cabe numa janela");

        let prompt = format!(
            "{}\n\n---\n\n{}",
            mos_core::instructions("NexoDoc — Comercial", ""),
            windows[0].text
        );
        println!("prompt: {} caracteres", prompt.len());

        let resposta = super::ask_once(super::DEFAULT_BASE_URL, &prompt)
            .await
            .expect("o Hermes precisa responder");
        println!("\nresposta crua ({} caracteres):\n{resposta}", resposta.len());

        let outcome = mos_core::parse_analysis(meeting_id, &resposta, &segments)
            .expect("o bloco precisa passar pela validacao do dominio");

        println!("\nRESUMO: {}", outcome.summary);
        println!("TOPICOS: {:?}", outcome.topics);
        println!("\n{} itens:", outcome.insights.len());
        for item in &outcome.insights {
            println!(
                "  [{:>13}] {:<6} {}  (evidencias: {})",
                item.kind.as_str(),
                item.confidence.as_str(),
                item.text,
                item.evidence.len()
            );
            for evidencia in &item.evidence {
                let trecho = segments
                    .iter()
                    .find(|s| s.id == evidencia.segment_id)
                    .map(|s| s.text.as_str())
                    .unwrap_or("<<<INEXISTENTE>>>");
                println!("        → {trecho}");
            }
        }
        println!("\nRECUSAS: {:?}", outcome.rejections);

        assert!(!outcome.summary.is_empty(), "precisa vir resumo");
        assert!(!outcome.insights.is_empty(), "precisa vir item");

        // A prova que sustenta o `WHY?`: toda evidencia que sobreviveu aponta
        // para um segmento que EXISTE. A validacao ja garante isso; o assert
        // existe para que uma regressao nela apareca aqui.
        for item in &outcome.insights {
            for evidencia in &item.evidence {
                assert!(
                    segments.iter().any(|s| s.id == evidencia.segment_id),
                    "evidencia apontando para segmento inexistente sobreviveu"
                );
            }
        }

        // E o que a feature promete: pelo menos um compromisso MEU, com
        // proveniencia. Sem isso, "o que EU prometi" nao existe.
        let meus: Vec<_> = outcome
            .insights
            .iter()
            .filter(|item| item.kind == mos_core::InsightKind::MyAction)
            .collect();
        println!("\ncompromissos meus: {}", meus.len());
        assert!(
            !meus.is_empty(),
            "a transcricao tem dois compromissos meus explicitos"
        );
        assert!(
            meus.iter().any(|item| !item.evidence.is_empty()),
            "pelo menos um compromisso precisa de evidencia"
        );
    }
}
