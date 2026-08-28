//! O guardião bloqueia? — a pergunta que ficou sem resposta por semanas.
//!
//! O `auth.rs` existia, compilava no CI, e **não estava montado em rota
//! nenhuma**. Um `cargo check` verde não sabe a diferença entre um guardião
//! montado e um guardião esquecido numa gaveta; só uma requisição sabe.
//!
//! Então este arquivo sobe o `mos-web` de verdade — pelo mesmo `api::servidor`
//! que o `main.rs` chama, e não por um `Router` montado à mão aqui, que é como
//! um teste passa a testar um caminho que o programa não usa — e bate nas
//! rotas. Sem cookie tem que levar 401. Com sessão válida tem que passar.
//!
//! Ele roda no Windows, sem OpenSSL e sem VPS, porque a metade que decide quem
//! entra não está atrás da feature `passkey`. Ver o topo de `porta.rs`.

use std::net::SocketAddr;
use std::path::Path;

use mos_web::estado::{Estado, Porta};

/// Sobe o `mos-web` com porta, e devolve (endereço, sessões) — as sessões para
/// o teste poder forjar um login sem precisar de WebAuthn.
async fn servir(pasta: &Path) -> (SocketAddr, std::sync::Arc<mos_web::porta::Sessoes>) {
    let backups = pasta.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let estado = Estado::abrir(
        pasta.join("web.db").to_str().unwrap(),
        backups.to_str().unwrap(),
        None,
        None,
        Some(Porta {
            banco: pasta.join("porta.db").to_str().unwrap().to_string(),
            origem: String::from("https://mos.exemplo"),
            convite: String::from("convite-de-teste"),
        }),
    )
    .unwrap();
    let sessoes = estado.sessoes.clone().expect("a porta foi configurada");

    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_web::api::servidor(estado);
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    (endereco, sessoes)
}

/// As rotas que guardam ou mexem em dado. Todas, e não uma amostra: a forma
/// mais comum de uma rota ficar aberta é alguém acrescentá-la depois, e uma
/// lista curta aqui não notaria.
const PROTEGIDAS: &[(&str, &str)] = &[
    ("GET", "/api/inbox"),
    ("GET", "/api/tasks"),
    ("GET", "/api/estado"),
    ("POST", "/api/capturar"),
    ("POST", "/api/tasks"),
    ("POST", "/api/push/assinar"),
    ("POST", "/api/push/testar"),
];

fn pedido(cliente: &reqwest::Client, metodo: &str, url: String) -> reqwest::RequestBuilder {
    match metodo {
        "POST" => cliente.post(url).json(&serde_json::json!({})),
        _ => cliente.get(url),
    }
}

#[tokio::test]
async fn sem_sessao_a_api_inteira_responde_401() {
    let pasta = tempfile::tempdir().unwrap();
    let (web, _) = servir(pasta.path()).await;
    let cliente = reqwest::Client::new();

    for (metodo, caminho) in PROTEGIDAS {
        let resposta = pedido(&cliente, metodo, format!("http://{web}{caminho}"))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resposta.status(),
            401,
            "{metodo} {caminho} respondeu {} sem sessao",
            resposta.status()
        );
    }
}

#[tokio::test]
async fn com_sessao_valida_a_api_responde() {
    let pasta = tempfile::tempdir().unwrap();
    let (web, sessoes) = servir(pasta.path()).await;
    let token = sessoes
        .criar("credencial-de-teste", time::OffsetDateTime::now_utc())
        .unwrap();
    let cliente = reqwest::Client::new();

    let resposta = cliente
        .get(format!("http://{web}/api/inbox"))
        .header("Cookie", format!("{}={token}", mos_web::porta::COOKIE))
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), 200);
}

/// Uma sessão vencida não é meia sessão.
#[tokio::test]
async fn uma_sessao_vencida_nao_passa() {
    let pasta = tempfile::tempdir().unwrap();
    let (web, sessoes) = servir(pasta.path()).await;
    let token = sessoes
        .criar(
            "antiga",
            time::OffsetDateTime::now_utc() - time::Duration::days(mos_web::porta::SESSAO_DIAS + 1),
        )
        .unwrap();
    let cliente = reqwest::Client::new();

    let resposta = cliente
        .get(format!("http://{web}/api/inbox"))
        .header("Cookie", format!("{}={token}", mos_web::porta::COOKIE))
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), 401);
}

/// A página tem que carregar sem sessão — ela é a tela de entrar.
///
/// Ela não expõe dado nenhum: o dado está atrás da API, que o teste acima prova
/// estar fechada.
#[tokio::test]
async fn a_pagina_carrega_sem_sessao() {
    let pasta = tempfile::tempdir().unwrap();
    let (web, _) = servir(pasta.path()).await;

    let resposta = reqwest::get(format!("http://{web}/")).await.unwrap();
    assert_eq!(resposta.status(), 200);
    assert!(resposta.text().await.unwrap().contains("<div id=\"raiz\">"));
}

/// O `index.html` tem que ser revalidado, e o bundle com hash pode ser eterno.
///
/// Sem isto, um `index.html` guardado pelo Safari aponta para um bundle que o
/// binario novo nao tem mais — e a PWA instalada abre em branco, sem nada
/// dizendo o que houve.
#[tokio::test]
async fn o_cache_deixa_o_html_envelhecer_e_o_bundle_nao() {
    let pasta = tempfile::tempdir().unwrap();
    let (web, _) = servir(pasta.path()).await;
    let cliente = reqwest::Client::new();

    for caminho in ["/", "/index.html", "/sw.js", "/manifest.webmanifest"] {
        let resposta = cliente
            .get(format!("http://{web}{caminho}"))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resposta.headers()["cache-control"],
            "no-cache",
            "{caminho} tem nome fixo: ele PRECISA ser revalidado"
        );
    }

    // O nome do bundle muda a cada build, entao guardar para sempre e seguro.
    let html = cliente
        .get(format!("http://{web}/"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let bundle = html
        .split("/assets/")
        .nth(1)
        .and_then(|resto| resto.split('"').next())
        .expect("o index.html referencia um bundle em /assets/");
    let resposta = cliente
        .get(format!("http://{web}/assets/{bundle}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), 200, "/assets/{bundle}");
    assert!(
        resposta.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("immutable"),
        "o bundle com hash pode ser guardado para sempre"
    );
}

/// A rota que a tela consulta antes de qualquer login precisa ser livre — e
/// precisa dizer o mínimo.
#[tokio::test]
async fn a_rota_da_porta_e_livre_e_diz_o_minimo() {
    let pasta = tempfile::tempdir().unwrap();
    let (web, _) = servir(pasta.path()).await;

    let corpo: serde_json::Value = reqwest::get(format!("http://{web}/api/porta/estado"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(corpo["porta"], true);
    assert_eq!(corpo["registrado"], false, "nenhum aparelho ainda");
    assert!(
        corpo.get("convite").is_none(),
        "o convite NAO pode aparecer numa rota livre: {corpo}"
    );
}

/// Sair invalida a sessão de verdade, e não só no navegador.
///
/// Um "sair" que apaga o cookie sem apagar a linha deixa a sessão viva para
/// quem tiver copiado o valor — que é o caso em que sair importa.
#[tokio::test]
async fn sair_invalida_a_sessao_no_servidor() {
    let pasta = tempfile::tempdir().unwrap();
    let (web, sessoes) = servir(pasta.path()).await;
    let token = sessoes
        .criar("credencial-de-teste", time::OffsetDateTime::now_utc())
        .unwrap();
    let cookie = format!("{}={token}", mos_web::porta::COOKIE);
    let cliente = reqwest::Client::new();

    cliente
        .post(format!("http://{web}/api/porta/sair"))
        .header("Cookie", &cookie)
        .send()
        .await
        .unwrap();

    let resposta = cliente
        .get(format!("http://{web}/api/inbox"))
        .header("Cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), 401);
}

/// Sem porta configurada — o `mos-web` de desenvolvimento, em localhost — nada
/// e bloqueado. O que impede isso de chegar a producao e `conferir_a_porta`, no
/// `main.rs`, que recusa subir publicado sem porta.
#[tokio::test]
async fn sem_porta_configurada_o_desenvolvimento_continua_solto() {
    let pasta = tempfile::tempdir().unwrap();
    let backups = pasta.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let estado = Estado::abrir(
        pasta.path().join("web.db").to_str().unwrap(),
        backups.to_str().unwrap(),
        None,
        None,
        None,
    )
    .unwrap();
    assert!(estado.sessoes.is_none());

    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let web = ouvinte.local_addr().unwrap();
    let rotas = mos_web::api::servidor(estado);
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });

    let resposta = reqwest::get(format!("http://{web}/api/inbox"))
        .await
        .unwrap();
    assert_eq!(resposta.status(), 200);
}
