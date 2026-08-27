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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EstadoDoAparelho {
    /// Mudancas locais esperando para subir. E o unico sinal honesto de que a
    /// sincronizacao esta ou nao acontecendo: se este numero nao baixa, algo
    /// esta errado, e a tela precisa poder dizer isso.
    pendentes: usize,
    /// Se ha hub configurado. Sem ele o `mos-web` funciona — sozinho.
    sincroniza: bool,
}

async fn estado_do_aparelho(State(estado): State<Estado>) -> Json<EstadoDoAparelho> {
    use mos_sync::OutboxRepository;
    Json(EstadoDoAparelho {
        pendentes: estado.storage.quantidade_pendente().unwrap_or(0),
        sincroniza: estado.hub.is_some(),
    })
}
