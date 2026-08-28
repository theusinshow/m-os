//! A superficie: capturar, inbox, tasks.
//!
//! Tres verbos e meia duzia de rotas. O desktop expoe 280 comandos, e a
//! distancia entre os dois numeros NAO e uma lacuna a ser fechada — e a
//! fronteira: isto e uma porta, e uma porta que vira casa deixa de ser porta.
//!
//! # Toda escrita responde ANTES de sincronizar
//!
//! A captura ja esta gravada no banco local quando a tela responde; a subida
//! acontece depois, em segundo plano. O contrario ligaria "tirar da cabeca" a
//! ter sinal — e a ideia que nao se escreve porque o 4G caiu e uma ideia
//! perdida.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use mos_core::{CaptureSource, CoreError, CreateCaptureInput, CreateTaskInput, TaskState};
use serde::{Deserialize, Serialize};

use crate::estado::Estado;

/// Quantos itens uma lista devolve.
///
/// A tela do celular mostra o que cabe no polegar; pedir mais seria gastar
/// bateria e rede para desenhar o que ninguem rola. Quem quer o resto abre o
/// desktop, que e onde o recorte existe.
const LIMITE: usize = 50;

pub fn rotas() -> Router<Estado> {
    Router::new()
        .route("/api/capturar", post(capturar))
        .route("/api/inbox", get(inbox))
        .route("/api/tasks", get(tasks).post(criar_task))
        .route("/api/tasks/{id}/estado", post(mudar_estado))
        .route("/api/estado", get(estado_do_aparelho))
        .route("/api/push/assinar", post(assinar_push))
        .route("/api/push/testar", post(testar_push))
}

/// As rotas mais a pagina.
pub fn rotas_com_pagina() -> Router<Estado> {
    rotas()
        .route("/api/porta/estado", get(porta_estado))
        .route("/api/porta/sair", post(sair))
        .fallback(get(pagina))
}

/// O servidor pronto: rotas, estado e o GUARDIAO.
///
/// Existe como funcao separada porque o guardiao precisa do estado ja
/// resolvido, e porque montar a porta a mao em cada lugar que sobe o servidor e
/// exatamente como uma delas fica sem porta. Quem sobe o `mos-web` de verdade
/// chama isto — o `main.rs` e o teste da porta.
pub fn servidor(estado: Estado) -> Router {
    let sessoes = estado.sessoes.clone();

    // A cerimonia entra ANTES do guardiao, e por isso ela e um `merge` e nao um
    // sub-router qualquer: as rotas dela vivem sob `/api/porta/`, que e o unico
    // prefixo livre. Fora dele, quem ainda nao entrou nao consegue nem pedir
    // para entrar.
    #[cfg(feature = "passkey")]
    let cerimonia = match (&estado.webauthn, &estado.sessoes) {
        (Some(webauthn), Some(sessoes)) => crate::auth::rotas(
            std::sync::Arc::clone(webauthn),
            std::sync::Arc::clone(sessoes),
        ),
        _ => Router::new(),
    };

    let servidor = rotas_com_pagina().with_state(estado);

    #[cfg(feature = "passkey")]
    let servidor = servidor.merge(cerimonia);

    servidor.layer(axum::middleware::from_fn_with_state(
        sessoes,
        crate::porta::guarda,
    ))
}

// ------------------------------------------------------------------- porta

/// O que a tela precisa saber ANTES de qualquer login.
///
/// Rota livre, e a unica informacao que ela entrega e se ha porta e se ja existe
/// aparelho registrado — o suficiente para a tela escolher entre "registrar" e
/// "entrar", e nada alem disso.
async fn porta_estado(State(estado): State<Estado>) -> Json<serde_json::Value> {
    let (tem_porta, registrado) = match &estado.sessoes {
        Some(sessoes) => (true, sessoes.ha_credencial().unwrap_or(false)),
        None => (false, false),
    };
    Json(serde_json::json!({
        "porta": tem_porta,
        "registrado": registrado,
        // A cerimonia WebAuthn so existe com a feature compilada. Sem ela, a
        // tela nao deve oferecer um botao que nao tem servidor do outro lado.
        "passkey": cfg!(feature = "passkey"),
    }))
}

async fn sair(
    State(estado): State<Estado>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> (
    axum_extra::extract::cookie::CookieJar,
    Json<serde_json::Value>,
) {
    if let Some(sessoes) = &estado.sessoes {
        let _ = sessoes.encerrar(&jar);
    }
    (
        jar.add(crate::porta::cookie_vazio()),
        Json(serde_json::json!({ "ok": true })),
    )
}

// ----------------------------------------------------------------- pagina

/// Os arquivos que o `vite build` produziu, dentro do binario.
///
/// Embutidos, e nao lidos do disco: um servico que depende de uma pasta ao lado
/// do executavel quebra quando alguem move o executavel, e o `systemd` roda com
/// um `WorkingDirectory` que nem sempre e o que se imagina.
///
/// A pasta precisa existir no momento da COMPILACAO — `npm run build` na `ui/`
/// vem antes de `cargo build`. O Cargo nao sabe disso sozinho, e por isso esta
/// escrito no README e no workflow.
#[derive(rust_embed::Embed)]
#[folder = "static/"]
struct Estaticos;

/// Qualquer caminho desconhecido devolve o `index.html`, e nao 404: a PWA e uma
/// pagina so, e um app instalado na tela de inicio recarrega numa rota interna o
/// tempo todo.
async fn pagina(uri: axum::http::Uri) -> Response {
    let caminho = uri.path().trim_start_matches('/');
    let (nome, arquivo) = match Estaticos::get(caminho) {
        Some(encontrado) => (caminho, Some(encontrado)),
        None => ("index.html", Estaticos::get("index.html")),
    };
    match arquivo {
        Some(conteudo) => {
            let tipo = mime_guess::from_path(nome).first_or_octet_stream();
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, tipo.as_ref().to_owned())],
                conteudo.data,
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            "A interface nao foi embutida neste binario.",
        )
            .into_response(),
    }
}

// ------------------------------------------------------------------ erros

struct Erro(StatusCode, String);

impl IntoResponse for Erro {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "erro": self.1 }))).into_response()
    }
}

/// O erro do dominio vira status HTTP pela sua PROPRIA classificacao.
///
/// Adivinhar pelo texto seria decidir por acaso — e o `mos-core` ja responde a
/// pergunta com `ErrorCode`, que existe justamente para nao ser interpretado.
fn de_core(causa: CoreError) -> Erro {
    use mos_core::ErrorCode;
    let status = match causa.code {
        ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::InvalidTransition => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    Erro(status, causa.message)
}

type Resultado<T> = Result<T, Erro>;

/// Dispara o sync sem fazer a resposta esperar por ele.
fn empurrar(estado: &Estado) {
    if let Some(hub) = &estado.hub {
        crate::sync::agora(
            std::sync::Arc::clone(&estado.storage),
            std::sync::Arc::clone(hub),
        );
    }
}

// ------------------------------------------------------------------ rotas

#[derive(Deserialize)]
pub struct Captura {
    pub texto: String,
}

async fn capturar(
    State(estado): State<Estado>,
    Json(pedido): Json<Captura>,
) -> Resultado<Json<serde_json::Value>> {
    let capture = estado
        .captures
        .create(CreateCaptureInput {
            content: pedido.texto,
            // A origem diz de ONDE veio, e isso e informacao de verdade: uma
            // captura feita no celular no meio da rua tem outra natureza da que
            // foi digitada no PC com o projeto aberto.
            source: CaptureSource::QuickCapture,
        })
        .map_err(de_core)?;

    empurrar(&estado);
    Ok(Json(serde_json::json!({ "id": capture.id.to_string() })))
}

async fn inbox(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let itens = estado.captures.inbox(LIMITE).map_err(de_core)?;
    Ok(Json(serde_json::to_value(itens).unwrap_or_default()))
}

async fn tasks(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let itens = estado.work.tasks(false).map_err(de_core)?;
    Ok(Json(serde_json::to_value(itens).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct NovaTask {
    pub titulo: String,
    #[serde(default)]
    pub descricao: String,
    #[serde(default)]
    pub project_id: Option<String>,
}

async fn criar_task(
    State(estado): State<Estado>,
    Json(pedido): Json<NovaTask>,
) -> Resultado<Json<serde_json::Value>> {
    let task = estado
        .work
        .create_task(CreateTaskInput {
            title: pedido.titulo,
            description: pedido.descricao,
            project_id: pedido.project_id,
            source_capture_id: None,
        })
        .map_err(de_core)?;

    empurrar(&estado);
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct MudarEstado {
    pub estado: TaskState,
}

async fn mudar_estado(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    Json(pedido): Json<MudarEstado>,
) -> Resultado<Json<serde_json::Value>> {
    let task = estado
        .work
        .set_task_state(&id, pedido.estado)
        .map_err(de_core)?;

    empurrar(&estado);
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

// -------------------------------------------------------------------- push

/// O push, ou o erro que explica a ausencia dele.
///
/// 501 e nao 500: nao ha defeito aqui, ha uma configuracao ausente. A tela
/// mostra a diferenca, e quem le o log tambem.
fn sem_push(estado: &Estado) -> Resultado<&crate::estado::PushLigado> {
    estado.push.as_ref().ok_or_else(|| {
        Erro(
            StatusCode::NOT_IMPLEMENTED,
            String::from(
                "este servidor nao tem chave VAPID configurada, entao nao manda                  notificacao nenhuma",
            ),
        )
    })
}

/// A tela assina.
///
/// O corpo e o `PushSubscription.toJSON()` do navegador, repassado inteiro. O
/// servidor nao interpreta nada dele — ver `push.rs`.
async fn assinar_push(
    State(estado): State<Estado>,
    Json(assinatura): Json<crate::push::Assinatura>,
) -> Resultado<Json<serde_json::Value>> {
    let push = sem_push(&estado)?;

    let agora_ms = (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
    push.assinaturas
        .salvar(&assinatura, agora_ms)
        .map_err(|causa| Erro(StatusCode::INTERNAL_SERVER_ERROR, causa.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Manda uma notificacao agora, para conferir.
///
/// Existe porque a alternativa para saber se o push funciona seria criar um
/// lembrete e ESPERAR ele vencer. Depois de instalar na tela de inicio, esta e a
/// unica pergunta que importa — chega ou nao chega? —, e ela merece resposta em
/// dois segundos.
async fn testar_push(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let push = sem_push(&estado)?;

    let avisador = std::sync::Arc::clone(&push.avisador);
    let aviso = crate::avisos::Aviso {
        titulo: String::from("M/OS"),
        corpo: String::from("Se voce esta lendo isto, a notificacao funciona."),
        tag: String::from("teste"),
        url: String::from("/"),
    };
    // `spawn_blocking` porque o envio e bloqueante — ver `push::enviar`.
    let aceitos = tokio::task::spawn_blocking(move || avisador.disparar(&aviso))
        .await
        .unwrap_or(0);

    Ok(Json(serde_json::json!({ "enviadas": aceitos })))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EstadoDoAparelho {
    /// Mudancas locais esperando para subir. E o unico sinal honesto de que a
    /// sincronizacao esta ou nao acontecendo: se este numero nao baixa, algo
    /// esta errado, e a tela precisa poder dizer isso.
    pendentes: usize,
    /// Se ha hub configurado. Sem ele o `mos-web` funciona — sozinho.
    sincroniza: bool,
    /// A chave publica VAPID, ou vazio quando este servidor nao manda
    /// notificacao. A tela decide o que mostrar a partir disto: com chave, o
    /// botao de ativar; sem chave, a frase que explica por que ele nao existe.
    chave_push: Option<String>,
    /// Quantos aparelhos ja assinaram. Serve para voce saber que ativou — sem
    /// isso, "ativar" e um botao que muda de cor e nao prova nada.
    aparelhos_avisados: usize,
}

async fn estado_do_aparelho(State(estado): State<Estado>) -> Json<EstadoDoAparelho> {
    use mos_sync::OutboxRepository;
    Json(EstadoDoAparelho {
        pendentes: estado.storage.quantidade_pendente().unwrap_or(0),
        sincroniza: estado.hub.is_some(),
        chave_push: estado.push.as_ref().map(|push| push.chave_publica.clone()),
        aparelhos_avisados: estado
            .push
            .as_ref()
            .and_then(|push| push.assinaturas.quantas().ok())
            .unwrap_or(0),
    })
}
