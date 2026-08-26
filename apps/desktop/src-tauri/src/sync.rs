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

use std::sync::Arc;

use mos_storage_sqlite::SqliteStorage;
use serde::Serialize;
use tauri::State;

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
}

/// O resultado de uma rodada, para a tela.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRound {
    pub sent: usize,
    pub received: usize,
    pub conflicts: usize,
    pub pending: usize,
    /// Preenchido quando a rodada parou por erro. **O que ja foi feito ate ali
    /// permanece feito** — sincronizacao parcial e melhor que nenhuma, e mentir
    /// que nada aconteceu faria o proximo clique parecer o primeiro.
    pub error: Option<String>,
}

#[tauri::command]
pub fn sync_status(state: State<'_, crate::AppState>) -> SyncStatus {
    use mos_sync::OutboxRepository;
    SyncStatus {
        endpoint: crate::load_settings(&state.settings_path).sync_endpoint,
        has_token: token_guardado().is_some(),
        pending: state.storage.quantidade_pendente().unwrap_or(0),
        enabled: state.storage.sync_ligado(),
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

/// Uma rodada, agora.
///
/// # Por que `spawn_blocking`
///
/// O `HttpTransport` e bloqueante, porque o motor e sincrono. Chamar isto
/// direto num comando `async` do Tauri derruba o processo com "cannot block the
/// current thread from within a runtime" — e derruba na hora, nao
/// intermitentemente. Ver o topo do `mos-sync-http`.
#[tauri::command]
pub async fn sync_now(state: State<'_, crate::AppState>) -> Result<SyncRound, String> {
    let endpoint = crate::load_settings(&state.settings_path).sync_endpoint;
    if endpoint.is_empty() {
        return Err("Configure o endereco do hub antes de sincronizar.".into());
    }
    let Some(token) = token_guardado() else {
        return Err("Falta o segredo do hub.".into());
    };
    let storage: Arc<SqliteStorage> = Arc::clone(&state.storage);

    tauri::async_runtime::spawn_blocking(move || {
        let transporte = mos_sync_http::HttpTransport::novo(endpoint, token)
            .map_err(|erro| erro.mensagem)?;
        let agora = time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        let rodada = storage
            .sincronizar_agora(&transporte, agora as i64, LIMITE)
            .map_err(|erro| erro.message)?;
        Ok(SyncRound {
            sent: rodada.enviadas,
            received: rodada.recebidas,
            conflicts: rodada.conflitos,
            pending: rodada.pendentes,
            error: rodada.erro,
        })
    })
    .await
    .map_err(|erro| format!("A rodada de sincronizacao nao terminou: {erro}"))?
}
