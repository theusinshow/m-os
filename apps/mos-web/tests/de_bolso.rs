//! A captura feita no bolso aparece no PC.
//!
//! E o teste que justifica o `mos-web` existir. Ele NAO usa mock de nada: sobe o
//! hub de verdade, o `mos-web` de verdade com banco proprio, e um segundo M/OS
//! tambem de verdade. A captura entra pela rota HTTP que o navegador vai chamar,
//! e a conferencia e feita pela leitura normal do outro aparelho.
//!
//! Se um dia isto passar com o `mos-web` sendo um proxy do desktop, o teste
//! esta errado — a promessa e que o celular funcione com o PC desligado.

use std::net::SocketAddr;
use std::path::Path;

use mos_core::{CaptureRepository, LifecycleState, WorkRepository};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::DeviceRepository;
use mos_sync_server::{Estado as EstadoHub, Hub};

const TOKEN: &str = "segredo-de-teste-com-tamanho-suficiente";

async fn servir_hub() -> SocketAddr {
    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_sync_server::rotas(EstadoHub::novo(Hub::em_memoria().unwrap(), TOKEN));
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    endereco
}

/// Sobe o `mos-web` de verdade, com banco proprio, apontado para o hub.
async fn servir_web(pasta: &Path, hub: SocketAddr) -> SocketAddr {
    let backups = pasta.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let estado = mos_web::estado::Estado::abrir(
        pasta.join("web.db").to_str().unwrap(),
        backups.to_str().unwrap(),
        Some(mos_web::estado::Hub {
            url: format!("http://{hub}"),
            token: TOKEN.to_owned(),
        }),
        // Sem push: estes testes conferem o laco de dados, e uma chave VAPID
        // aqui so acrescentaria uma ida a rede que nao tem para onde ir.
        None,
        // Sem porta: quem confere a porta e o `tests/a_porta.rs`, e exigir
        // sessao aqui obrigaria todo teste de dados a fazer login antes.
        None,
    )
    .unwrap();

    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_web::api::rotas().with_state(estado);
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    endereco
}

/// O outro aparelho: um M/OS comum.
fn outro_aparelho(pasta: &Path) -> SqliteStorage {
    let backups = pasta.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(pasta.join("mos.db"), &backups).unwrap();
    let device = storage.este_dispositivo("PC", "windows", "0.3.1").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    storage
}

fn sincronizar(storage: &SqliteStorage, hub: SocketAddr) -> mos_sync::Rodada {
    let transporte = mos_sync_http::HttpTransport::novo(format!("http://{hub}"), TOKEN).unwrap();
    let agora = (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
    storage.sincronizar_agora(&transporte, agora, 100).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_captura_do_bolso_chega_no_pc() {
    let hub = servir_hub().await;
    let pasta_web = tempfile::tempdir().unwrap();
    let pasta_pc = tempfile::tempdir().unwrap();
    let web = servir_web(pasta_web.path(), hub).await;

    // Pela rota que o navegador vai chamar.
    let resposta = reqwest::Client::new()
        .post(format!("http://{web}/api/capturar"))
        .json(&serde_json::json!({ "texto": "testar aquela biblioteca depois" }))
        .send()
        .await
        .unwrap();
    assert!(resposta.status().is_success(), "capturar falhou");

    // A resposta NAO espera o sync, entao aqui e preciso dar tempo ao empurrao
    // de segundo plano. Espera por uma CONDICAO, e nao por um numero fixo:
    // dormir "o bastante" e como um teste instavel comeca.
    let caminho_pc = pasta_pc.path().to_path_buf();
    let pc = tokio::task::spawn_blocking(move || {
        let pc = outro_aparelho(&caminho_pc);
        for _ in 0..50 {
            if sincronizar(&pc, hub).recebidas > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        pc
    })
    .await
    .unwrap();

    let capturas = pc.by_lifecycle(LifecycleState::Active, 50).unwrap();
    assert_eq!(capturas.len(), 1, "a captura nao chegou no PC");
    assert_eq!(capturas[0].content, "testar aquela biblioteca depois");
}

/// E a volta: a Task criada no PC aparece na lista do bolso.
#[tokio::test(flavor = "multi_thread")]
async fn a_task_do_pc_aparece_no_bolso() {
    let hub = servir_hub().await;
    let pasta_web = tempfile::tempdir().unwrap();
    let pasta_pc = tempfile::tempdir().unwrap();
    let web = servir_web(pasta_web.path(), hub).await;

    let caminho_pc = pasta_pc.path().to_path_buf();
    tokio::task::spawn_blocking(move || {
        let pc = outro_aparelho(&caminho_pc);
        let nova = mos_core::NewTask::create("Comprar cimento", "", None).unwrap();
        pc.create_task(nova).unwrap();
        assert_eq!(sincronizar(&pc, hub).enviadas, 1);
    })
    .await
    .unwrap();

    // O laco de fundo do `mos-web` roda a cada minuto; o teste nao espera por
    // ele. Cada captura dispara o empurrao, que PUXA junto — o mesmo gatilho que
    // o uso real tem.
    let mut encontrada = false;
    for _ in 0..50 {
        let itens: serde_json::Value = reqwest::get(format!("http://{web}/api/tasks"))
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if itens.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            assert_eq!(itens[0]["title"], "Comprar cimento");
            encontrada = true;
            break;
        }
        let _ = reqwest::Client::new()
            .post(format!("http://{web}/api/capturar"))
            .json(&serde_json::json!({ "texto": "ping" }))
            .send()
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(encontrada, "a Task do PC nao apareceu no bolso");
}

/// Escrever duas vezes seguidas nao trava o servidor.
///
/// # O defeito que este teste guarda
///
/// O `SqliteStorage` tem dois cadeados — a conexao e o relogio logico — e os
/// dois caminhos os pegavam em ORDEM CONTRARIA: uma escrita fecha a conexao e
/// depois pede o relogio; uma rodada de sync fecha o relogio e depois pede a
/// conexao. Duas ordens contrarias sao um abraco mortal, e como TODA escrita
/// dispara uma rodada em segundo plano, bastava a segunda escrita cair dentro da
/// rodada da primeira.
///
/// Isso nao era um caso de borda: era capturar duas coisas seguidas. O servidor
/// travava **para sempre**, calado — sem log, sem erro, sem 500 —, e continuava
/// travado depois de fechar e abrir o app. Ver `estado::Estado::vez`.
///
/// Seis escritas seguidas, e nao duas, porque o encontro depende de a segunda
/// cair DENTRO da rodada da primeira: uma so tentativa poderia passar por sorte
/// de tempo, e um teste que passa por sorte nao guarda nada.
#[tokio::test(flavor = "multi_thread")]
async fn escrever_em_rajada_nao_trava() {
    let hub = servir_hub().await;
    let pasta = tempfile::tempdir().unwrap();
    let web = servir_web(pasta.path(), hub).await;
    let cliente = reqwest::Client::new();

    for numero in 0..6 {
        let resposta = cliente
            .post(format!("http://{web}/api/capturar"))
            .json(&serde_json::json!({ "texto": format!("ideia {numero}") }))
            .send()
            .await
            .unwrap();
        assert!(resposta.status().is_success(), "a captura {numero} falhou");
    }

    // E o banco tem as seis: travar nao e o unico jeito de perder escrita.
    let inbox: serde_json::Value = cliente
        .get(format!("http://{web}/api/inbox"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(inbox.as_array().map(Vec::len), Some(6));
}

/// A PWA sai de dentro do binario.
///
/// Sem isto o servidor sobe, a API responde e a pessoa ve uma pagina em branco —
/// o pior sintoma possivel, porque nada indica onde procurar. O teste falha na
/// compilacao se a pasta `static/` nao existir, e falha aqui se ela existir
/// vazia.
#[tokio::test(flavor = "multi_thread")]
async fn a_pagina_vem_embutida() {
    let hub = servir_hub().await;
    let pasta = tempfile::tempdir().unwrap();
    let backups = pasta.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let estado = mos_web::estado::Estado::abrir(
        pasta.path().join("web.db").to_str().unwrap(),
        backups.to_str().unwrap(),
        Some(mos_web::estado::Hub {
            url: format!("http://{hub}"),
            token: TOKEN.to_owned(),
        }),
        // Sem push: estes testes conferem o laco de dados, e uma chave VAPID
        // aqui so acrescentaria uma ida a rede que nao tem para onde ir.
        None,
        // Sem porta: quem confere a porta e o `tests/a_porta.rs`, e exigir
        // sessao aqui obrigaria todo teste de dados a fazer login antes.
        None,
    )
    .unwrap();

    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_web::api::rotas_com_pagina().with_state(estado);
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });

    let raiz = reqwest::get(format!("http://{endereco}/")).await.unwrap();
    assert!(raiz.status().is_success());
    let html = raiz.text().await.unwrap();
    assert!(html.contains("<div id=\"raiz\">"), "o index.html nao veio");
    assert!(
        html.contains("manifest.webmanifest"),
        "sem manifest nao ha 'Adicionar a Tela de Inicio'"
    );

    // Um caminho que nao existe devolve a pagina, e nao 404: a PWA e uma pagina
    // so, e um app instalado recarrega numa rota interna o tempo todo.
    let interna = reqwest::get(format!("http://{endereco}/tasks"))
        .await
        .unwrap();
    assert!(interna.status().is_success(), "rota interna deu 404");

    let manifest = reqwest::get(format!("http://{endereco}/manifest.webmanifest"))
        .await
        .unwrap();
    assert!(manifest.status().is_success());
    assert!(manifest.text().await.unwrap().contains("standalone"));
}
