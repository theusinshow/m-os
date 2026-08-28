//! Os comandos da integracao academica externa.
//!
//! A divisao segue a do resto do desktop: o motor de reconciliacao esta em
//! `mos_core::academic_sync`, a aplicacao no banco em
//! `mos_storage_sqlite::academic_provider_repository`, e o dialeto do Univirtus
//! em `crate::univirtus`. Aqui fica so o que nenhum dos tres pode ter — a
//! janela, o relogio de quem esta na frente da tela e o aviso ao renderer.
//!
//! # O login mora numa janela, e nao num formulario
//!
//! O M/OS **nao pede RU e senha**. Ele abre a pagina oficial do Univirtus numa
//! janela do proprio app, a pessoa entra la, e o M/OS recolhe da janela apenas
//! as duas pecas que a API exige: o cookie de sessao e o `X-time`. A senha nunca
//! passa por aqui, nunca e digitada num campo nosso e nunca chega ao banco.
//!
//! Nao e escrupulo: e o que a investigacao mediu. Nao existe endpoint que troque
//! credencial por token no Univirtus (`docs/UNIVIRTUS-INTEGRATION.md` §2). Um
//! formulario proprio teria de reimplementar o POST do portal e guardar a senha
//! para repeti-lo — assumindo um risco para entregar exatamente a mesma sessao
//! que a janela entrega sem ele.

use std::sync::Mutex;

use mos_core::academic_sync::{
    ProviderConnection, ProviderStatus, SyncOutcome, SyncReport, PROVIDER_UNIVIRTUS,
};
use mos_core::{CoreError, ErrorCode};
use mos_storage_sqlite::{AcademicProviderRepository, ProviderSubjectFact};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

use crate::univirtus::{self, UnivirtusClient, UnivirtusSession};
use crate::AppState;

/// O rotulo da janela de login. Fixo para que abrir duas vezes reaproveite a
/// mesma em vez de empilhar janelas.
const JANELA: &str = "univirtus-login";

fn avisar<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit_to("main", "data-changed", "academic-changed");
}

// ===========================================================================
// Estado
// ===========================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnivirtusStatus {
    #[serde(flatten)]
    pub status: ProviderStatus,
    /// Ha sessao guardada no cofre do sistema? **Nao** diz qual — so se existe.
    pub has_session: bool,
}

#[tauri::command]
pub fn univirtus_status<R: Runtime>(app: AppHandle<R>) -> Result<UnivirtusStatus, CoreError> {
    let storage = app.state::<AppState>().storage.clone();
    let mut status = storage.provider_status(PROVIDER_UNIVIRTUS)?;
    let has_session = univirtus::ha_sessao();
    // O banco pode dizer "conectado" enquanto o cofre ja foi limpo por fora.
    // Quem manda sobre "da para chamar a API" e o cofre.
    if !has_session && status.connection == ProviderConnection::Connected {
        status.connection = ProviderConnection::Expired;
    }
    Ok(UnivirtusStatus {
        status,
        has_session,
    })
}

#[tauri::command]
pub fn univirtus_subject_facts<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<ProviderSubjectFact>, CoreError> {
    app.state::<AppState>()
        .storage
        .provider_subject_facts(PROVIDER_UNIVIRTUS)
}

/// O ultimo endereco visto de um material.
///
/// Pode estar vencido — a URL do Univirtus e assinada e dura horas. Quem chama
/// trata `None` e URL morta do mesmo jeito: pedindo uma sincronizacao.
#[tauri::command]
pub fn univirtus_material_url<R: Runtime>(
    app: AppHandle<R>,
    external_id: String,
) -> Result<Option<String>, CoreError> {
    app.state::<AppState>()
        .storage
        .material_url(PROVIDER_UNIVIRTUS, &external_id)
}

#[tauri::command]
pub fn univirtus_disconnect<R: Runtime>(app: AppHandle<R>) -> Result<(), CoreError> {
    univirtus::esquecer_sessao()?;
    app.state::<AppState>()
        .storage
        .forget_provider(PROVIDER_UNIVIRTUS)?;
    avisar(&app);
    Ok(())
}

// ===========================================================================
// Conectar
// ===========================================================================

/// O que a janela de login devolveu, enquanto ela ainda esta aberta.
struct Colheita {
    x_time: Mutex<Option<String>>,
}

/// Abre a pagina oficial e espera a pessoa entrar.
///
/// O que este comando faz de verdade:
///
/// 1. abre `https://univirtus.uninter.com/ava/web/` numa janela propria;
/// 2. espera a URL virar `#/ava` — o que so acontece depois do login;
/// 3. le `sessionStorage.user.time` (o `X-time`) por `eval_with_callback`;
/// 4. le o cookie de sessao do proprio WebView, que o JS nao alcanca por ser
///    `HttpOnly`;
/// 5. guarda os dois no Credential Manager e fecha a janela.
///
/// Nada do que a pessoa digita atravessa o M/OS. O passo 4 e a razao de isto
/// precisar ser uma janela do app em vez do navegador do sistema: o cookie e
/// `HttpOnly`, e so o processo dono do WebView consegue le-lo.
#[tauri::command]
pub async fn univirtus_connect<R: Runtime>(
    app: AppHandle<R>,
) -> Result<UnivirtusStatus, CoreError> {
    let url: tauri::Url = univirtus::LOGIN_URL
        .parse()
        .map_err(|_| falha("Endereco de login invalido."))?;

    if let Some(existente) = app.get_webview_window(JANELA) {
        let _ = existente.set_focus();
    } else {
        WebviewWindowBuilder::new(&app, JANELA, WebviewUrl::External(url.clone()))
            .title("Entrar no Univirtus")
            .inner_size(1100.0, 800.0)
            .center()
            .build()
            .map_err(|error| falha(&format!("Nao foi possivel abrir a janela: {error}")))?;
    }

    let colheita = std::sync::Arc::new(Colheita {
        x_time: Mutex::new(None),
    });

    // A espera tem teto. Sem ele, uma pessoa que desiste do login deixaria uma
    // tarefa viva ate o app fechar.
    let limite = std::time::Duration::from_secs(300);
    let comeco = std::time::Instant::now();

    let sessao = loop {
        if comeco.elapsed() > limite {
            fechar(&app);
            return Err(falha(
                "O login demorou demais. Abra de novo quando estiver pronto.",
            ));
        }
        let Some(janela) = app.get_webview_window(JANELA) else {
            // A pessoa fechou a janela. Nao e erro: e desistir.
            return Err(falha("A janela do Univirtus foi fechada antes do login."));
        };

        tokio::time::sleep(std::time::Duration::from_millis(700)).await;

        // A URL so chega a `#/ava` depois de autenticar e escolher o ambiente.
        let dentro = janela
            .url()
            .map(|u| u.as_str().contains("#/ava"))
            .unwrap_or(false);
        if !dentro {
            continue;
        }

        // `JSON.stringify` explicito: o retorno chega serializado, e um valor
        // numerico de 18 digitos passaria por `f64` e perderia precisao no
        // caminho. Como texto, ele chega inteiro.
        let alvo = colheita.clone();
        let _ = janela.eval_with_callback(
            "(() => { try { const u = JSON.parse(sessionStorage.getItem('user') || '{}'); \
              return String(u.time || ''); } catch (e) { return ''; } })()",
            move |resultado| {
                let limpo = resultado.trim().trim_matches('"').to_owned();
                if !limpo.is_empty() && limpo != "null" {
                    if let Ok(mut slot) = alvo.x_time.lock() {
                        *slot = Some(limpo);
                    }
                }
            },
        );

        // O callback e assincrono: uma volta curta antes de olhar.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let x_time = colheita
            .x_time
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_default();
        if x_time.is_empty() {
            continue;
        }

        let cookies = janela
            .cookies_for_url(url.clone())
            .map_err(|error| falha(&format!("Nao foi possivel ler a sessao: {error}")))?;
        // O header inteiro, como o navegador o mandaria. O cookie de consentimento
        // vai junto de proposito: filtrar por nome exigiria conhecer o nome, e
        // ele nao e contrato — o portal pode renomea-lo amanha.
        let cookie = cookies
            .iter()
            .map(|c| format!("{}={}", c.name(), c.value()))
            .collect::<Vec<_>>()
            .join("; ");

        match UnivirtusSession::new(cookie, x_time) {
            Ok(sessao) => break sessao,
            // Ainda incompleto: o portal pode nao ter terminado de escrever.
            Err(_) => continue,
        }
    };

    // Prova que a sessao serve antes de guardar. Guardar primeiro deixaria o
    // M/OS dizendo "conectado" com uma sessao que nunca funcionou.
    let cliente = UnivirtusClient::new(sessao.clone())?;
    cliente.check_session().await?;

    univirtus::guardar_sessao(&sessao)?;
    app.state::<AppState>()
        .storage
        .set_provider_connection(PROVIDER_UNIVIRTUS, ProviderConnection::Connected)?;
    fechar(&app);
    avisar(&app);
    univirtus_status(app)
}

fn fechar<R: Runtime>(app: &AppHandle<R>) {
    if let Some(janela) = app.get_webview_window(JANELA) {
        let _ = janela.close();
    }
}

// ===========================================================================
// Sincronizar
// ===========================================================================

/// Traz o que o portal tem agora.
///
/// # O que acontece quando a sessao caiu
///
/// O retrato **nao e aplicado**. Um retrato vazio, aplicado, seria lido pela
/// reconciliacao como "sumiu tudo", e toda disciplina viraria `unavailable` de
/// uma vez. O que acontece e o oposto: a conexao vira `expired`, os dados de
/// antes ficam intactos, e a tela passa a oferecer "Reconectar".
#[tauri::command]
pub async fn univirtus_sync<R: Runtime>(app: AppHandle<R>) -> Result<SyncReport, CoreError> {
    let comeco = time::OffsetDateTime::now_utc();
    let storage = app.state::<AppState>().storage.clone();

    let Some(sessao) = univirtus::ler_sessao() else {
        storage.set_provider_connection(PROVIDER_UNIVIRTUS, ProviderConnection::Disconnected)?;
        return Err(falha("O Univirtus nao esta conectado."));
    };

    // O fuso de quem esta na frente da tela. O Univirtus manda data SEM fuso, e
    // le-la como UTC adiantaria todo prazo em tres horas: "vence 23h59" seria
    // gravado como 23h59 UTC e apareceria na tela como 20h59.
    //
    // Por isso a recusa em vez do palpite. Sincronizar antes de a tela montar
    // gravaria a hora errada em todos os prazos de uma vez, e a proxima
    // sincronizacao nao consertaria — o hash bateria, e nada seria atualizado.
    if !crate::surface::offset_publicado(&app) {
        return Err(falha(
            "A tela ainda nao publicou o fuso. Tente sincronizar em um instante.",
        ));
    }
    let offset = crate::surface::now_local(&app).offset();

    let cliente = UnivirtusClient::new(sessao)?;
    let retrato = match cliente.snapshot(offset).await {
        Ok(retrato) => retrato,
        Err(erro) if erro.code == ErrorCode::ProviderUnauthorized => {
            storage.set_provider_connection(PROVIDER_UNIVIRTUS, ProviderConnection::Expired)?;
            avisar(&app);
            return Err(erro);
        }
        Err(erro) => {
            // Falha de rede nao muda o estado da conexao: a sessao pode estar
            // ótima e o wi-fi, nao.
            return Err(erro);
        }
    };

    let relatorio = storage.apply_provider_snapshot(&retrato, comeco)?;
    avisar(&app);
    Ok(relatorio)
}

/// A sincronizacao da abertura do app.
///
/// Silenciosa: nao abre janela, nao mostra erro e nao muda nada se o Univirtus
/// nao estiver conectado. Dado academico muda algumas vezes por semana, e um
/// polling agressivo contra um portal de faculdade seria uma gentileza que
/// ninguem pediu.
pub fn sincronizar_na_abertura<R: Runtime>(app: AppHandle<R>) {
    if !univirtus::ha_sessao() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        // Espera a tela publicar o fuso. Sem isto o sync da abertura roda com
        // offset zero e grava todo prazo tres horas adiantado — foi o que
        // aconteceu na primeira sincronizacao real, e a tela mostrou 20h59
        // onde o portal diz 23h59.
        for _ in 0..60 {
            if crate::surface::offset_publicado(&app) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        if !crate::surface::offset_publicado(&app) {
            return;
        }
        match univirtus_sync(app.clone()).await {
            Ok(relatorio) if relatorio.outcome != SyncOutcome::Completed => {
                let _ = app.emit_to("main", "univirtus-synced", &relatorio);
            }
            Ok(relatorio) => {
                // So avisa quando ha o que dizer. "Tudo em dia" nao merece
                // nem toast.
                if !relatorio.resumo().is_empty() {
                    let _ = app.emit_to("main", "univirtus-synced", &relatorio);
                }
            }
            Err(_) => {
                // Sessao expirada ja marcou o estado; a tela conta a historia
                // quando a pessoa abrir Settings. Um popup na abertura seria o
                // app cobrando algo que ninguem pediu agora.
            }
        }
    });
}

fn falha(mensagem: &str) -> CoreError {
    CoreError::new(ErrorCode::Io, mensagem.to_owned(), false)
}
