//! O contrato do servidor, conferido contra o `HubLocal`.
//!
//! O `SYNC.md` §11 elegeu o `HubLocal` de `sync_two_devices.rs` como
//! especificacao executavel: **guardar em ordem, devolver a partir de um
//! cursor, aceitar reenvio sem duplicar**. Cada teste aqui prende uma dessas
//! tres frases, mais o que so existe quando o hub sai da memoria e vai para uma
//! porta: sobreviver ao reinicio, recusar quem nao tem credencial e recusar
//! quem fala outro contrato.
//!
//! O servidor sobe de verdade, numa porta efemera, e os testes falam com ele
//! por HTTP. Chamar os handlers direto pularia o cabecalho de autorizacao, o
//! parse da query e o codigo de status — que e onde mora metade do que pode
//! estar errado numa superficie de rede.

use std::net::SocketAddr;

use mos_sync::{EntityRef, Hlc, Lote, Op, OpBody, CONTRACT_VERSION};
use mos_sync_server::{Estado, Hub};
use serde_json::json;
use uuid::Uuid;

const TOKEN: &str = "segredo-de-teste";

/// Sobe o servidor com o hub dado e devolve o endereco.
async fn servir(hub: Hub) -> SocketAddr {
    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_sync_server::rotas(Estado::novo(hub, TOKEN));
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    endereco
}

fn op_de(device: Uuid, contador: u32, campo: &str, valor: &str) -> Op {
    Op::new(
        Uuid::now_v7(),
        EntityRef::new("task", Uuid::now_v7()),
        OpBody::Update {
            fields: json!({ campo: valor }).as_object().unwrap().clone(),
        },
        Hlc {
            wall_ms: 1_700_000_000_000,
            counter: contador,
            device: mos_sync::DeviceId(device),
        },
    )
}

async fn push(endereco: SocketAddr, token: &str, contrato: u32, ops: &[Op]) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("http://{endereco}/sync/push"))
        .bearer_auth(token)
        .json(&json!({ "contrato": contrato, "ops": ops }))
        .send()
        .await
        .unwrap()
}

async fn pull(endereco: SocketAddr, cursor: &str, limite: usize) -> Lote {
    reqwest::Client::new()
        .get(format!(
            "http://{endereco}/sync/pull?contrato={CONTRACT_VERSION}&cursor={cursor}&limite={limite}"
        ))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[tokio::test]
async fn guarda_em_ordem_e_devolve_a_partir_do_cursor() {
    let endereco = servir(Hub::em_memoria().unwrap()).await;
    let device = Uuid::now_v7();
    let ops: Vec<Op> = (0..5)
        .map(|i| op_de(device, i, "titulo", &format!("op {i}")))
        .collect();

    assert!(push(endereco, TOKEN, CONTRACT_VERSION, &ops)
        .await
        .status()
        .is_success());

    let primeiro = pull(endereco, "", 2).await;
    assert_eq!(primeiro.ops.len(), 2);
    assert_eq!(primeiro.ops[0].id, ops[0].id, "a ordem e a de chegada");
    assert!(primeiro.tem_mais);

    let segundo = pull(endereco, &primeiro.proximo_cursor, 2).await;
    assert_eq!(segundo.ops[0].id, ops[2].id, "continua de onde parou");

    let terceiro = pull(endereco, &segundo.proximo_cursor, 2).await;
    assert_eq!(terceiro.ops.len(), 1);
    assert!(!terceiro.tem_mais, "o ultimo lote nao promete mais nada");
}

/// O atalho `ops.len() == limite` erraria aqui: o lote acaba exato, e o cliente
/// gastaria uma rodada inteira para receber vazio. No iPhone isso e radio
/// ligado a toa.
#[tokio::test]
async fn lote_exato_nao_promete_mais() {
    let endereco = servir(Hub::em_memoria().unwrap()).await;
    let device = Uuid::now_v7();
    let ops: Vec<Op> = (0..4).map(|i| op_de(device, i, "titulo", "x")).collect();
    push(endereco, TOKEN, CONTRACT_VERSION, &ops).await;

    let lote = pull(endereco, "", 4).await;
    assert_eq!(lote.ops.len(), 4);
    assert!(!lote.tem_mais);
}

#[tokio::test]
async fn reenviar_confirma_de_novo_sem_duplicar() {
    let endereco = servir(Hub::em_memoria().unwrap()).await;
    let device = Uuid::now_v7();
    let ops = vec![op_de(device, 0, "titulo", "uma so")];

    let primeira = push(endereco, TOKEN, CONTRACT_VERSION, &ops).await;
    let aceitas_1: serde_json::Value = primeira.json().await.unwrap();

    // O cliente perdeu a resposta e reenviou. Aceitar e diferente de guardar: a
    // operacao ja conhecida e confirmada do mesmo jeito, senao ela nunca sairia
    // da fila de saida.
    let segunda = push(endereco, TOKEN, CONTRACT_VERSION, &ops).await;
    let aceitas_2: serde_json::Value = segunda.json().await.unwrap();

    assert_eq!(aceitas_1, aceitas_2);
    assert_eq!(
        pull(endereco, "", 100).await.ops.len(),
        1,
        "guardou uma vez"
    );
}

#[tokio::test]
async fn o_cursor_sobrevive_ao_reinicio_do_servidor() {
    let pasta = tempfile::tempdir().unwrap();
    let caminho = pasta.path().join("hub.db");
    let device = Uuid::now_v7();

    let endereco = servir(Hub::abrir(&caminho).unwrap()).await;
    let ops: Vec<Op> = (0..3).map(|i| op_de(device, i, "titulo", "a")).collect();
    push(endereco, TOKEN, CONTRACT_VERSION, &ops).await;
    let antes = pull(endereco, "", 2).await;

    // Outro processo, mesmo arquivo. O cursor e uma promessa externa: um
    // dispositivo que guardou "2" e voltou uma semana depois precisa continuar
    // de 2, e nao receber tudo de novo nem perder o meio.
    let depois_endereco = servir(Hub::abrir(&caminho).unwrap()).await;
    let depois = pull(depois_endereco, &antes.proximo_cursor, 100).await;

    assert_eq!(depois.ops.len(), 1);
    assert_eq!(depois.ops[0].id, ops[2].id);
}

#[tokio::test]
async fn sem_credencial_nao_entra() {
    let endereco = servir(Hub::em_memoria().unwrap()).await;
    let ops = vec![op_de(Uuid::now_v7(), 0, "titulo", "x")];

    let errada = push(endereco, "token-errado", CONTRACT_VERSION, &ops).await;
    assert_eq!(errada.status(), reqwest::StatusCode::UNAUTHORIZED);

    let sem = reqwest::Client::new()
        .get(format!(
            "http://{endereco}/sync/pull?contrato={CONTRACT_VERSION}&cursor=&limite=10"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(sem.status(), reqwest::StatusCode::UNAUTHORIZED);

    // E nada entrou pela porta recusada.
    assert_eq!(pull(endereco, "", 100).await.ops.len(), 0);
}

#[tokio::test]
async fn contrato_incompativel_e_recusado_sem_pedir_nova_tentativa() {
    let endereco = servir(Hub::em_memoria().unwrap()).await;
    let ops = vec![op_de(Uuid::now_v7(), 0, "titulo", "x")];

    let resposta = push(endereco, TOKEN, CONTRACT_VERSION + 99, &ops).await;
    assert_eq!(resposta.status(), reqwest::StatusCode::CONFLICT);

    let corpo: serde_json::Value = resposta.json().await.unwrap();
    assert_eq!(
        corpo["retriavel"], false,
        "insistir num contrato incompativel nunca vai dar certo, e no celular custa bateria"
    );
}

#[tokio::test]
async fn health_responde_sem_credencial() {
    let endereco = servir(Hub::em_memoria().unwrap()).await;
    let corpo: serde_json::Value = reqwest::get(format!("http://{endereco}/health"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(corpo["ok"], true);
    assert_eq!(corpo["contrato"], CONTRACT_VERSION);
}
