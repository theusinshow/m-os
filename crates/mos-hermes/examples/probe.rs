//! Sonda do caminho completo do Hermes, pela MESMA biblioteca que o app usa.
//!
//! Existe porque o app desktop nao mostra log: quando a ponte falha, o usuario
//! ve "Desconectado" e mais nada. Esta sonda percorre status, login, ticket,
//! socket e sessao imprimindo cada etapa, e depois fica ouvindo os quadros.
//!
//! Foi ela que isolou o bug do `gateway.ready`: o socket cru recebia os quadros
//! e o caminho da Bridge fechava na hora, o que apontou o dedo para o nosso
//! codigo em vez da rede.
//!
//! Nao inventa credencial: le a mesma entrada do Credential Manager que o app,
//! entao prova o caminho real e nao um caminho parecido.
//!
//!   cargo run -p mos-hermes --example probe

use std::time::Duration;

use mos_hermes::{Bridge, Credentials, Gateway};

// O crate nao habilita rt-multi-thread: a ponte roda no runtime do Tauri.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let base_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:9119".to_owned());
    println!("gateway: {base_url}");

    let gateway = match Gateway::new(&base_url) {
        Ok(gateway) => gateway,
        Err(error) => return println!("FALHA ao criar cliente: {error}"),
    };
    let status = match gateway.status().await {
        Ok(status) => status,
        Err(error) => return println!("FALHA em /api/status: {error}"),
    };
    println!(
        "status: auth_required={} providers={:?}",
        status.auth_required, status.auth_providers
    );

    if status.auth_required {
        let credentials = match Credentials::load() {
            Ok(credentials) => credentials,
            Err(error) => return println!("FALHA ao ler credencial: {error}"),
        };
        println!("credencial: usuario '{}'", credentials.username);
        let provider = status
            .auth_providers
            .first()
            .cloned()
            .unwrap_or_else(|| "basic".to_owned());
        if let Err(error) = gateway.login(&credentials, &provider).await {
            return println!("FALHA no login ({provider}): {error}");
        }
        println!("login: ok");
    }

    let ticket = match gateway.mint_ticket().await {
        Ok(ticket) => ticket,
        Err(error) => return println!("FALHA ao cunhar ticket: {error}"),
    };
    println!("ticket: {} caracteres", ticket.len());

    let channels = match mos_hermes::connect(&gateway.websocket_url(&ticket)).await {
        Ok(channels) => channels,
        Err(error) => return println!("FALHA ao abrir o socket: {error}"),
    };
    println!("socket: aberto");

    let mut bridge = Bridge::new(channels);
    if let Err(error) = bridge.open_session(None).await {
        return println!("FALHA ao abrir sessao: {error}");
    }
    println!("session.create enviado. Ouvindo 15s...\n");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            println!("\nestado final: {:?}", bridge.state());
            println!("sessao: {:?}", bridge.session_id());
            return;
        }
        match tokio::time::timeout(remaining, bridge.next()).await {
            Err(_) => continue,
            Ok(None) => return println!("\nSOCKET FECHADO pelo servidor."),
            Ok(Some(outcome)) => println!("quadro: {outcome:?}"),
        }
    }
}
