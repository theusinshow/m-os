//! A superficie de rede do hub: duas rotas de contrato, e duas de metadado.
//!
//! O `Transport` do motor tem dois metodos, `push` e `pull`. `/sync/push` e
//! `/sync/pull` sao a traducao literal dos dois para HTTP, e nenhuma regra de
//! dominio pode entrar nelas — o § "o servidor coordena e persiste, nao
//! transforma cliente em terminal burro" do `SYNC.md` continua valendo palavra
//! por palavra.
//!
//! `/sync/aparelho` e `/sync/aparelhos` sao a excecao consciente. Elas nao
//! carregam regra nenhuma: o hub grava o que o aparelho diz de si — nome,
//! plataforma, versao, contrato — e devolve a lista. Nenhuma operacao e
//! recusada por causa disso, nenhum cliente e bloqueado, e o motor sequer sabe
//! que elas existem (a batida vive fora do `Transport`, no `mos-sync-http`).
//!
//! Elas existem porque a pergunta "quem esta na malha, e em que versao" nao
//! tinha onde ser respondida — nem no servidor nem na tela. Responde-la em
//! 02/09/2026 custou uma manha de investigacao com `curl` dentro de um tunel
//! SSH, para descobrir que um dos PCs tinha trocado de identidade.

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use mos_sync::{contrato_compativel, Lote, Op, CONTRACT_VERSION};
use serde::{Deserialize, Serialize};

use crate::hub::Hub;

/// O teto de um lote, decidido pelo servidor.
///
/// O cliente pede um limite e o servidor corta: sem isto, um `limite` grande
/// vindo de fora vira uma resposta de tamanho arbitrario montada em memoria.
/// Quem define o custo maximo de uma chamada e quem paga por ela.
const LIMITE_MAXIMO: usize = 500;
const LIMITE_PADRAO: usize = 100;

#[derive(Clone)]
pub struct Estado {
    hub: Arc<Mutex<Hub>>,
    /// O segredo compartilhado. Ver `autorizado`.
    token: Arc<String>,
}

impl Estado {
    pub fn novo(hub: Hub, token: impl Into<String>) -> Self {
        Self {
            hub: Arc::new(Mutex::new(hub)),
            token: Arc::new(token.into()),
        }
    }
}

/// As rotas.
pub fn rotas(estado: Estado) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sync/push", post(push))
        .route("/sync/pull", get(pull))
        .route("/sync/aparelho", post(registrar_aparelho))
        .route("/sync/aparelhos", get(aparelhos))
        .with_state(estado)
}

// ------------------------------------------------------------------ erros

/// O erro na rede, com a mesma divisao que o `SyncError` do motor faz.
///
/// `retriavel` viaja porque e ela que o cliente usa para decidir entre backoff
/// e desistir. Um 500 por disco cheio pede nova tentativa; um contrato
/// incompativel nunca vai dar certo por insistencia, e insistir nele e uma
/// bateria queimada a toa no celular.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErroJson {
    mensagem: String,
    retriavel: bool,
}

struct Erro(StatusCode, String, bool);

impl IntoResponse for Erro {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(ErroJson {
                mensagem: self.1,
                retriavel: self.2,
            }),
        )
            .into_response()
    }
}

// ------------------------------------------------------------------- auth

/// Segredo compartilhado no cabecalho `Authorization: Bearer`.
///
/// Um segredo, e nao um por dispositivo: o M/OS tem um dono, e um dono e uma
/// conta. Emitir credencial por aparelho so passa a valer alguma coisa no dia
/// em que houver aparelho para revogar sem derrubar os outros — e nesse dia
/// isto vira uma tabela, sem o contrato mudar.
///
/// A comparacao roda em tempo constante. Comparar segredo com `==` vaza o
/// tamanho do prefixo correto pelo tempo de resposta, e o servidor fica exposto
/// numa porta publica.
fn autorizado(cabecalhos: &HeaderMap, esperado: &str) -> bool {
    let Some(valor) = cabecalhos
        .get("authorization")
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(recebido) = valor.strip_prefix("Bearer ") else {
        return false;
    };
    tempo_constante(recebido.as_bytes(), esperado.as_bytes())
}

fn tempo_constante(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn negado() -> Erro {
    Erro(
        StatusCode::UNAUTHORIZED,
        "Credencial ausente ou invalida.".into(),
        false,
    )
}

/// O contrato e conferido em TODA chamada, e nao so no handshake.
///
/// Nao ha handshake: o cliente pode atualizar entre um `push` e o `pull`
/// seguinte, e o servidor nao guarda sessao para perceber.
fn conferir_contrato(recebido: u32) -> Result<(), Erro> {
    if contrato_compativel(recebido) {
        return Ok(());
    }
    Err(Erro(
        StatusCode::CONFLICT,
        mos_sync::erro_de_contrato(recebido).mensagem,
        false,
    ))
}

fn falha_no_banco(causa: crate::hub::HubError) -> Erro {
    // Retriavel: disco cheio, lock ocupado e arquivo temporariamente
    // indisponivel sao os casos comuns aqui, e os tres passam numa nova
    // tentativa. A mensagem crua nao vai para o cliente — ela nomeia caminho de
    // arquivo, e isso e do servidor.
    eprintln!("[sync] falha no hub: {causa}");
    Erro(
        StatusCode::INTERNAL_SERVER_ERROR,
        "O hub nao conseguiu gravar agora.".into(),
        true,
    )
}

// ------------------------------------------------------------------ rotas

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Saude {
    ok: bool,
    contrato: u32,
    minimo: u32,
}

/// Sem autenticacao de proposito: e a rota que o monitoramento e o proxy
/// consultam, e ela nao conta nada que ja nao esteja no binario do cliente.
async fn health() -> Json<Saude> {
    Json(Saude {
        ok: true,
        contrato: CONTRACT_VERSION,
        minimo: mos_sync::MIN_CONTRACT_VERSION,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushPedido {
    contrato: u32,
    ops: Vec<Op>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PushResposta {
    aceitas: Vec<uuid::Uuid>,
}

async fn push(
    State(estado): State<Estado>,
    cabecalhos: HeaderMap,
    Json(pedido): Json<PushPedido>,
) -> Result<Json<PushResposta>, Erro> {
    if !autorizado(&cabecalhos, &estado.token) {
        return Err(negado());
    }
    conferir_contrato(pedido.contrato)?;

    let agora = agora_iso();
    let mut hub = estado.hub.lock().expect("hub envenenado");
    let aceitas = hub.push(&pedido.ops, &agora).map_err(falha_no_banco)?;
    Ok(Json(PushResposta { aceitas }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullPedido {
    contrato: u32,
    #[serde(default)]
    cursor: String,
    #[serde(default)]
    limite: Option<usize>,
}

async fn pull(
    State(estado): State<Estado>,
    cabecalhos: HeaderMap,
    Query(pedido): Query<PullPedido>,
) -> Result<Json<Lote>, Erro> {
    if !autorizado(&cabecalhos, &estado.token) {
        return Err(negado());
    }
    conferir_contrato(pedido.contrato)?;

    let limite = pedido
        .limite
        .unwrap_or(LIMITE_PADRAO)
        .clamp(1, LIMITE_MAXIMO);
    let hub = estado.hub.lock().expect("hub envenenado");
    let lote = hub.pull(&pedido.cursor, limite).map_err(falha_no_banco)?;
    Ok(Json(lote))
}

fn agora_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

// ------------------------------------------------------------- a malha

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AparelhoPedido {
    id: String,
    nome: String,
    plataforma: String,
    versao: String,
    contrato: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AparelhoJson {
    id: String,
    nome: String,
    plataforma: String,
    versao: String,
    contrato: u32,
    visto_em: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MalhaResposta {
    aparelhos: Vec<AparelhoJson>,
}

/// A batida de um aparelho.
///
/// O contrato NAO e conferido aqui, e a ausencia e a decisao: um aparelho velho
/// demais para sincronizar ainda precisa conseguir se anunciar — e ver "versao
/// 0.2.9, visto ha tres dias" na tela e exatamente como se descobre isso. Nas
/// rotas de contrato ele continua sendo conferido em toda chamada.
async fn registrar_aparelho(
    State(estado): State<Estado>,
    cabecalhos: HeaderMap,
    Json(pedido): Json<AparelhoPedido>,
) -> Result<StatusCode, Erro> {
    if !autorizado(&cabecalhos, &estado.token) {
        return Err(negado());
    }
    let aparelho = crate::hub::AparelhoRegistrado {
        id: pedido.id,
        nome: pedido.nome,
        plataforma: pedido.plataforma,
        versao: pedido.versao,
        contrato: pedido.contrato,
        visto_em: String::new(),
    };
    let agora = agora_iso();
    let mut hub = estado.hub.lock().expect("hub envenenado");
    hub.registrar_aparelho(&aparelho, &agora)
        .map_err(falha_no_banco)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Quem esta na malha.
///
/// Lista vazia nao e erro: um hub que ainda nao recebeu batida de ninguem
/// responde vazio, e a tela sabe dizer isso melhor que uma falha.
async fn aparelhos(
    State(estado): State<Estado>,
    cabecalhos: HeaderMap,
) -> Result<Json<MalhaResposta>, Erro> {
    if !autorizado(&cabecalhos, &estado.token) {
        return Err(negado());
    }
    let hub = estado.hub.lock().expect("hub envenenado");
    let lista = hub.aparelhos().map_err(falha_no_banco)?;
    Ok(Json(MalhaResposta {
        aparelhos: lista
            .into_iter()
            .map(|aparelho| AparelhoJson {
                id: aparelho.id,
                nome: aparelho.nome,
                plataforma: aparelho.plataforma,
                versao: aparelho.versao,
                contrato: aparelho.contrato,
                visto_em: aparelho.visto_em,
            })
            .collect(),
    }))
}
