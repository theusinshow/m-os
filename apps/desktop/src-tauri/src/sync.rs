//! A sincronizacao, vista do app.
//!
//! Tres coisas moram aqui, e nenhuma delas e regra: onde fica o hub, qual e o
//! segredo, e quando rodar. O laco em si e do `mos-sync`, e a traducao de
//! operacao em entidade e do `mos-storage-sqlite` — este arquivo so liga os
//! dois a um botao.
//!
//! # O segredo nao passa por aqui de volta
//!
//! O token vai para o Credential Manager do Windows, pelo mesmo caminho que a
//! credencial do Hermes ja usa. O renderer aprende que EXISTE um token; nunca o
//! recebe de volta. Um segredo que a interface pode ler e um segredo que
//! aparece num screenshot.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager, State};

const SERVICO: &str = "com.codedbym.mos.sync";
const CONTA: &str = "hub";

/// Quantas operacoes por rodada.
///
/// Cem, e nao "todas": o `pull` devolve um cursor, entao uma sincronizacao
/// inicial grande acontece em varias rodadas curtas em vez de uma longa que
/// falha inteira no meio.
const LIMITE: usize = 100;

fn entrada() -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICO, CONTA)
        .map_err(|erro| format!("Credential Manager indisponivel: {erro}"))
}

fn token_guardado() -> Option<String> {
    entrada().ok()?.get_password().ok()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    /// Onde o hub esta. Vazio significa "nao configurado".
    pub endpoint: String,
    /// Se ha token guardado. **Nunca** o token.
    pub has_token: bool,
    /// Quantas mudancas locais esperam para subir. E o que a interface mostra
    /// para dizer "ha alteracoes pendentes" sem varrer a fila inteira.
    pub pending: usize,
    /// Se a emissao esta ligada neste dispositivo.
    pub enabled: bool,
    /// Uma rodada corre agora.
    pub running: bool,
    /// RFC3339 de quando a ultima terminou. `None`: nunca rodou nesta sessao.
    pub last_sync_at: Option<String>,
    /// Por que a ultima parou. `None`: terminou inteira.
    pub last_error: Option<String>,
    /// O resumo da primeira rodada do dia, enquanto nao for lido.
    pub day_summary: Option<Resumo>,
    /// O id DESTE aparelho, para a tela saber qual linha da malha e ela mesma.
    /// Vazio quando a identidade nao pode ser lida — a malha ainda aparece, so
    /// sem o "este aparelho".
    pub device_id: String,
    /// A versao DESTE app, para a tela marcar quem esta em versao diferente.
    pub app_version: String,
}

/// O resultado de uma rodada, para a tela.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRound {
    pub sent: usize,
    pub received: usize,
    pub conflicts: usize,
    pub pending: usize,
    /// Quantas ENTIDADES de cada tipo chegaram. Nao e `received`
    /// reparticionado: aquele conta operacoes.
    pub received_by_kind: BTreeMap<String, usize>,
    /// Preenchido quando a rodada parou por erro. **O que ja foi feito ate ali
    /// permanece feito** — sincronizacao parcial e melhor que nenhuma, e mentir
    /// que nada aconteceu faria o proximo clique parecer o primeiro.
    pub error: Option<String>,
}

#[tauri::command]
pub fn sync_status(
    state: State<'_, crate::AppState>,
    runtime: State<'_, SyncRuntime>,
) -> SyncStatus {
    use mos_sync::OutboxRepository;
    // Lock envenenado nao pode derrubar a tela: sem o que a ultima rodada
    // disse, a faixa some — e uma Home sem faixa e melhor que uma Home que nao
    // abre.
    let ultima = runtime.ultima.lock().ok();
    SyncStatus {
        endpoint: crate::load_settings(&state.settings_path).sync_endpoint,
        has_token: token_guardado().is_some(),
        pending: state.storage.quantidade_pendente().unwrap_or(0),
        enabled: state.storage.sync_ligado(),
        running: runtime.rodando.load(Ordering::Relaxed),
        last_sync_at: ultima.as_ref().and_then(|u| u.em.clone()),
        last_error: ultima.as_ref().and_then(|u| u.erro.clone()),
        day_summary: ultima.as_ref().and_then(|u| u.resumo.clone()),
        device_id: {
            use mos_sync::DeviceRepository;
            let nome = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Este PC".to_owned());
            state
                .storage
                .este_dispositivo(&nome, "windows", env!("CARGO_PKG_VERSION"))
                .map(|eu| eu.id.to_string())
                .unwrap_or_default()
        },
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

#[tauri::command]
pub fn sync_set_endpoint(url: String, state: State<'_, crate::AppState>) -> Result<(), String> {
    let limpo = url.trim().trim_end_matches('/').to_owned();
    // Vazio e uma resposta valida: e como se desliga a sincronizacao sem apagar
    // o token nem o que ja esta na fila.
    if !limpo.is_empty() && !limpo.starts_with("http://") && !limpo.starts_with("https://") {
        return Err("O endereco do hub deve comecar com http:// ou https://.".into());
    }
    let mut settings = crate::load_settings(&state.settings_path);
    settings.sync_endpoint = limpo;
    crate::save_settings(&state.settings_path, &settings).map_err(|erro| erro.message)
}

#[tauri::command]
pub fn sync_set_token(token: String) -> Result<(), String> {
    let token = token.trim();
    // O servidor recusa a subir com menos que isto; recusar aqui tambem evita
    // a viagem ate descobrir que a credencial nunca ia servir.
    if token.len() < 32 {
        return Err("O segredo precisa ter ao menos 32 caracteres.".into());
    }
    entrada()?
        .set_password(token)
        .map_err(|erro| format!("Nao foi possivel guardar o segredo: {erro}"))
}

#[tauri::command]
pub fn sync_clear_token() -> Result<(), String> {
    match entrada()?.delete_credential() {
        Ok(()) => Ok(()),
        // Nao havia nada guardado. Pedir para apagar o que ja nao existe e um
        // pedido atendido, e nao um erro.
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(erro) => Err(format!("Nao foi possivel apagar o segredo: {erro}")),
    }
}

/// De quanto em quanto tempo a rodada acontece sem ninguem pedir.
///
/// Quinze minutos e a REDE DE SEGURANCA, e nao o mecanismo: o que sincroniza de
/// verdade sao a abertura, o primeiro plano e a mutacao. Este intervalo existe
/// para o caso que nenhum dos tres cobre — a rede que voltou sozinha enquanto a
/// janela ficou aberta e parada. O `SYNC.md` §51 proibe polling agressivo, e
/// uma tentativa por quarto de hora nao e polling.
const REDE_DE_SEGURANCA: Duration = Duration::from_secs(15 * 60);

/// Quanto esperar depois de uma mutacao antes de sincronizar.
///
/// Sem isto, arrastar cinco tasks no Kanban dispararia cinco rodadas. O motor
/// segura o mutex do relogio durante a rodada, entao rodada a mais nao e so
/// trafego — e a interface esperando.
pub const DEBOUNCE_DA_MUTACAO: Duration = Duration::from_secs(10);

/// Ate quando esperar a tela dizer que abriu, antes de rodar assim mesmo.
///
/// O sinal vem do renderer, e um sync que DEPENDE da tela e um sync que morre
/// em silencio quando a tela nao abre — e o M/OS pode abrir minimizado na
/// bandeja por configuracao. O teto existe para o automatico nunca ficar refem
/// de uma janela.
const TETO_DA_ABERTURA: Duration = Duration::from_secs(30);

/// O que a interface precisa saber, e que nao mora no banco.
pub struct SyncRuntime {
    /// Uma rodada por vez. Duas brigariam pelo mesmo `HlcClock`, e duas
    /// operacoes com o mesmo instante e o mesmo dispositivo quebram a ordem
    /// total — a unica coisa que a reconciliacao tem para desempatar.
    ///
    /// Assincrono de proposito: o clique manual ESPERA a rodada automatica e ve
    /// o resultado dela, em vez de falhar ou enfileirar uma segunda.
    rodada: tokio::sync::Mutex<()>,
    /// Acorda o laco: primeiro plano, mutacao, clique.
    acordar: tokio::sync::Notify,
    /// A tela terminou de abrir.
    pronto: tokio::sync::Notify,
    rodando: AtomicBool,
    ultima: Mutex<UltimaRodada>,
}

#[derive(Default)]
struct UltimaRodada {
    /// RFC3339 de quando a ultima rodada terminou.
    em: Option<String>,
    erro: Option<String>,
    /// O resumo por ler. `Some` so quando foi a primeira rodada do dia E ela
    /// trouxe alguma coisa.
    resumo: Option<Resumo>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resumo {
    pub by_kind: BTreeMap<String, usize>,
    pub at: String,
}

impl Default for SyncRuntime {
    fn default() -> Self {
        Self {
            rodada: tokio::sync::Mutex::new(()),
            acordar: tokio::sync::Notify::new(),
            pronto: tokio::sync::Notify::new(),
            rodando: AtomicBool::new(false),
            ultima: Mutex::new(UltimaRodada::default()),
        }
    }
}

/// Uma rodada — a mesma para o daemon e para o botao.
///
/// Uma so implementacao de proposito: duplicar aqui duplicaria a decisao de
/// quando parar, e a copia ficaria para tras no primeiro ajuste.
///
/// `Ok(None)` quando nao ha nada configurado. Isso NAO e erro: o M/OS funciona
/// inteiro sem sincronizar, e o daemon chamaria isto a cada quinze minutos numa
/// maquina onde o sync nunca foi ligado.
///
/// # Por que `spawn_blocking`
///
/// O `HttpTransport` e bloqueante, porque o motor e sincrono. Chamar direto de
/// dentro de um worker do tokio derruba o processo com "cannot block the
/// current thread from within a runtime" — na hora, nao intermitentemente.
async fn rodar(app: &tauri::AppHandle) -> Result<Option<SyncRound>, String> {
    let (storage, settings_path) = {
        let state = app.state::<crate::AppState>();
        (Arc::clone(&state.storage), state.settings_path.clone())
    };
    let endpoint = crate::load_settings(&settings_path).sync_endpoint;
    if endpoint.is_empty() {
        return Ok(None);
    }
    let Some(token) = token_guardado() else {
        return Ok(None);
    };

    let runtime = app.state::<SyncRuntime>();
    // Uma por vez. Quem chegou depois ESPERA e ve o resultado desta.
    let _turno = runtime.rodada.lock().await;
    runtime.rodando.store(true, Ordering::Relaxed);
    let _ = app.emit("sync-changed", ());

    let resultado = tauri::async_runtime::spawn_blocking(move || {
        let transporte =
            mos_sync_http::HttpTransport::novo(endpoint, token).map_err(|erro| erro.mensagem)?;

        // A batida vem ANTES da rodada, e o erro dela nao interrompe nada: quem
        // nao conseguiu se anunciar ainda tem trabalho a sincronizar, e trocar
        // dado por metadado seria pessimo negocio.
        anunciar(storage.as_ref(), &transporte);

        let agora = time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        storage
            .sincronizar_agora(&transporte, agora as i64, LIMITE)
            .map_err(|erro| erro.message)
    })
    .await;

    runtime.rodando.store(false, Ordering::Relaxed);

    let rodada =
        resultado.map_err(|erro| format!("A rodada de sincronizacao nao terminou: {erro}"))??;

    let agora = time::OffsetDateTime::now_utc();
    let em = agora
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let hoje = agora.date().to_string();

    {
        let mut settings = crate::load_settings(&settings_path);
        let primeira_do_dia = settings.sync_ultimo_resumo_em != hoje;
        if let Ok(mut ultima) = runtime.ultima.lock() {
            ultima.em = Some(em.clone());
            ultima.erro = rodada.erro.clone();
            // O resumo so nasce na PRIMEIRA rodada do dia, e so se trouxe algo.
            // Nao ter noticia nao e noticia.
            if primeira_do_dia && !rodada.recebidas_por_tipo.is_empty() {
                ultima.resumo = Some(Resumo {
                    by_kind: rodada.recebidas_por_tipo.clone(),
                    at: em.clone(),
                });
                // Marca ANTES de a tela ler: se o app fechar entre a rodada e a
                // leitura, o resumo se perde — e perder uma noticia e melhor
                // que mostrar a de anteontem como se fosse de hoje.
                settings.sync_ultimo_resumo_em = hoje;
                let _ = crate::save_settings(&settings_path, &settings);
            }
        }
    }

    let _ = app.emit("sync-changed", ());
    Ok(Some(SyncRound {
        sent: rodada.enviadas,
        received: rodada.recebidas,
        conflicts: rodada.conflitos,
        pending: rodada.pendentes,
        received_by_kind: rodada.recebidas_por_tipo,
        error: rodada.erro,
    }))
}

/// Uma rodada, agora, pedida pelo botao.
#[tauri::command]
pub async fn sync_now(app: tauri::AppHandle) -> Result<SyncRound, String> {
    match rodar(&app).await? {
        Some(rodada) => Ok(rodada),
        None => Err("Configure o endereco do hub antes de sincronizar.".into()),
    }
}

/// O laco que faz o M/OS sincronizar sozinho.
///
/// # Por que a primeira rodada ESPERA
///
/// `sincronizar_agora` segura o mutex do relogio a rodada inteira, de proposito
/// — soltar no meio faria uma mutacao local emitir um instante que o motor ja
/// passou. Com fila grande, rodar junto com a abertura seguraria o banco
/// enquanto a webview faz a rajada de IPC do boot: o `abertura.ts` gastaria as
/// 12 tentativas dele contra um banco ocupado, e o sintoma seria a tela de erro
/// que 4800e75 acabou de consertar — com causa nova e a mesma mensagem
/// mentirosa.
pub fn iniciar_daemon(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        {
            let runtime = app.state::<SyncRuntime>();
            let _ = tokio::time::timeout(TETO_DA_ABERTURA, runtime.pronto.notified()).await;
        }

        loop {
            if let Err(erro) = rodar(&app).await {
                // Ao `stderr`, e nao numa caixa: a rodada de fundo que falha
                // nao pode interromper quem esta trabalhando. A faixa conta.
                eprintln!("[sync] a rodada de fundo parou: {erro}");
                let runtime = app.state::<SyncRuntime>();
                runtime.rodando.store(false, Ordering::Relaxed);
                if let Ok(mut ultima) = runtime.ultima.lock() {
                    ultima.erro = Some(erro);
                }
                let _ = app.emit("sync-changed", ());
            }

            let runtime = app.state::<SyncRuntime>();
            let _ = tokio::time::timeout(REDE_DE_SEGURANCA, runtime.acordar.notified()).await;
        }
    });
}

/// Pede uma rodada. Ignorado se o daemon ainda nao existe.
///
/// `notify_one` e nao `notify_waiters`: o laco e um so, e um pedido que chega
/// enquanto ele ja roda fica GUARDADO — a proxima espera retorna na hora, em vez
/// de a mutacao que chegou no meio da rodada esperar quinze minutos.
pub fn acordar<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(runtime) = app.try_state::<SyncRuntime>() {
        runtime.acordar.notify_one();
    }
}

/// A tela terminou de abrir. Libera a primeira rodada.
#[tauri::command]
pub fn sync_app_pronto(runtime: State<'_, SyncRuntime>) {
    runtime.pronto.notify_waiters();
}

/// O resumo do dia foi lido.
#[tauri::command]
pub fn sync_dispensar_resumo(
    app: tauri::AppHandle,
    runtime: State<'_, SyncRuntime>,
) -> Result<(), String> {
    runtime
        .ultima
        .lock()
        .map_err(|_| "Estado do sync indisponivel.".to_string())?
        .resumo = None;
    let _ = app.emit("sync-changed", ());
    Ok(())
}

/// Diz ao hub quem e este PC.
///
/// Falha em silencio no log, e nao para a tela: a batida e metadado, e uma
/// mensagem de erro sobre ela roubaria o lugar do que a rodada tem a dizer
/// sobre o trabalho de verdade.
fn anunciar(
    storage: &mos_storage_sqlite::SqliteStorage,
    transporte: &mos_sync_http::HttpTransport,
) {
    use mos_sync::DeviceRepository;

    let nome = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Este PC".to_owned());
    let eu = match storage.este_dispositivo(&nome, "windows", env!("CARGO_PKG_VERSION")) {
        Ok(eu) => eu,
        Err(causa) => {
            eprintln!("[sync] sem identidade para anunciar: {}", causa.mensagem);
            return;
        }
    };
    // Manifesto que nao pode ser calculado nao impede a batida: o aparelho ainda
    // precisa aparecer na malha, mesmo que sem retrato.
    let manifesto: Vec<mos_sync_http::FamiliaNoAnuncio> = storage
        .manifesto()
        .map(|linhas| {
            linhas
                .into_iter()
                .map(|linha| mos_sync_http::FamiliaNoAnuncio {
                    familia: linha.familia,
                    contagem: linha.contagem,
                    hash: linha.hash,
                })
                .collect()
        })
        .unwrap_or_default();

    if let Err(causa) = transporte.anunciar(&mos_sync_http::Anuncio {
        id: &eu.id.to_string(),
        nome: &eu.name,
        plataforma: "windows",
        versao: env!("CARGO_PKG_VERSION"),
        contrato: mos_sync::CONTRACT_VERSION,
        manifesto: &manifesto,
    }) {
        eprintln!("[sync] a batida nao chegou: {}", causa.mensagem);
    }
}

/// Quem esta na malha, como o hub conhece.
///
/// Lista vazia nao e erro: sem hub configurado, ou com um hub que ainda nao
/// recebeu batida de ninguem, a resposta certa e "ninguem ainda" — e a tela diz
/// isso melhor que uma mensagem de falha.
#[tauri::command]
pub async fn sync_malha(
    app: tauri::AppHandle,
) -> Result<Vec<mos_sync_http::AparelhoNaMalha>, String> {
    let settings_path = {
        let state = app.state::<crate::AppState>();
        state.settings_path.clone()
    };
    let endpoint = crate::load_settings(&settings_path).sync_endpoint;
    if endpoint.is_empty() {
        return Ok(Vec::new());
    }
    let Some(token) = token_guardado() else {
        return Ok(Vec::new());
    };
    tauri::async_runtime::spawn_blocking(move || {
        let transporte =
            mos_sync_http::HttpTransport::novo(endpoint, token).map_err(|erro| erro.mensagem)?;
        transporte.malha().map_err(|erro| erro.mensagem)
    })
    .await
    .map_err(|erro| format!("A consulta a malha nao terminou: {erro}"))?
}

/// Roda a varredura de reparo agora, a pedido da tela.
///
/// Existe como botao alem da abertura porque quem esta olhando a malha e vendo
/// "divergente" quer agir naquele minuto, e nao no proximo reinicio.
#[tauri::command]
pub async fn sync_reparar(app: tauri::AppHandle) -> Result<mos_storage_sqlite::Reparo, String> {
    let storage = {
        let state = app.state::<crate::AppState>();
        Arc::clone(&state.storage)
    };
    let reparo = tauri::async_runtime::spawn_blocking(move || {
        storage
            .reparar_materializacao()
            .map_err(|erro| erro.message)
    })
    .await
    .map_err(|erro| format!("O reparo nao terminou: {erro}"))??;
    // A tela inteira pode ter ganhado linhas; quem estiver com uma lista aberta
    // precisa reler.
    if reparo.reparadas > 0 {
        let _ = app.emit("data-changed", "reparo");
    }
    Ok(reparo)
}
