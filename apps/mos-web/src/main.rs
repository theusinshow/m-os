//! O M/OS de bolso.
//!
//! Uma porta, e nao um segundo M/OS — ver o `Cargo.toml`.
//!
//! # A porta, e por que ela e uma feature
//!
//! `passkey` liga a porta de verdade (WebAuthn). Ela e opcional porque o
//! `webauthn-rs` depende de OpenSSL, que nao existe na maquina de
//! desenvolvimento — e como dependencia obrigatoria ela impediria de compilar
//! todo o RESTO: servidor, sync, superficie. Com feature, o resto anda aqui e a
//! porta e conferida onde OpenSSL existe.
//!
//! `porta-aberta` nao autentica nada e serve para desenvolver. O binario
//! RECUSA a subir com ela ligada se o bind nao for localhost — uma porta de
//! desenvolvimento que chega em producao e como a maioria dos vazamentos
//! comeca.

use std::net::SocketAddr;

use mos_web::api;
use mos_web::avisos;
use mos_web::estado::{Estado, Hub, Push};
use mos_web::push::Vapid;
use mos_web::sync;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Um subcomando, e nao uma geracao automatica na subida: um servidor que
    // sorteia a chave VAPID ao subir e um servidor que fica mudo a cada
    // reinicio, porque as assinaturas do aparelho ficam presas a chave antiga.
    // Ela nasce uma vez, a mao, e vai para o arquivo de ambiente.
    if std::env::args().any(|arg| arg == "--gerar-vapid") {
        let (privada, publica) = Vapid::gerar();
        println!("MOS_WEB_VAPID_PRIVADA={privada}");
        println!("# a publica e derivada da privada; esta linha e so para conferencia");
        println!("# publica={publica}");
        return Ok(());
    }

    let bind = std::env::var("MOS_WEB_BIND").unwrap_or_else(|_| String::from("127.0.0.1"));
    let porta: u16 = std::env::var("MOS_WEB_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9130);

    conferir_a_porta(&bind)?;

    let banco = std::env::var("MOS_WEB_DB").unwrap_or_else(|_| String::from("mos-web.db"));
    let backups = std::env::var("MOS_WEB_BACKUPS").unwrap_or_else(|_| String::from("backups"));

    // Hub opcional: sem ele o `mos-web` funciona sozinho, e isso e util para
    // desenvolver. Em producao, um `mos-web` sem hub e um caderno separado com
    // cara de M/OS — por isso ele DIZ, alto, quando sobe assim.
    let hub = match (std::env::var("MOS_WEB_HUB"), std::env::var("MOS_WEB_TOKEN")) {
        (Ok(url), Ok(token)) if !url.is_empty() && !token.is_empty() => Some(Hub { url, token }),
        _ => {
            eprintln!(
                "[web] AVISO: sem MOS_WEB_HUB/MOS_WEB_TOKEN. Nada sai deste aparelho nem \
                 chega nele."
            );
            None
        }
    };

    // O push tambem e opcional, e pela mesma logica do hub: sem chave o
    // `mos-web` sobe igual, so que mudo — e a tela DIZ isso, em vez de mostrar
    // um botao que nao faz nada.
    let push = match std::env::var("MOS_WEB_VAPID_PRIVADA") {
        Ok(privada) if !privada.is_empty() => Some(Push {
            banco: std::env::var("MOS_WEB_PUSH_DB").unwrap_or_else(|_| String::from("push.db")),
            privada,
            contato: std::env::var("MOS_WEB_VAPID_CONTATO")
                .unwrap_or_else(|_| String::from("mailto:mos@localhost")),
        }),
        _ => None,
    };

    let estado = Estado::abrir(&banco, &backups, hub, push)?;

    if let Some(hub) = &estado.hub {
        sync::iniciar(
            std::sync::Arc::clone(&estado.storage),
            std::sync::Arc::clone(hub),
            estado
                .push
                .as_ref()
                .map(|push| std::sync::Arc::clone(&push.avisador)),
        );
    }

    // O laco dos lembretes. Separado do sync de proposito: um lembrete vence na
    // hora dele mesmo que a rede esteja fora, e amarrar o aviso a uma rodada de
    // sync bem-sucedida faria o celular ficar quieto justamente no dia em que a
    // VPS estivesse com problema de rede.
    if let Some(push) = &estado.push {
        avisos::iniciar(
            std::sync::Arc::clone(&push.avisador),
            std::sync::Arc::clone(&estado.attention),
        );
    }

    let rotas = api::rotas_com_pagina().with_state(estado);
    let endereco: SocketAddr = format!("{bind}:{porta}").parse()?;
    println!("[web] {banco}, ouvindo {endereco}");

    let ouvinte = tokio::net::TcpListener::bind(endereco).await?;
    axum::serve(ouvinte, rotas)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            println!("[web] encerrando.");
        })
        .await?;
    Ok(())
}

/// Recusa combinacoes que nao deveriam existir.
///
/// As duas metades sao independentes e as duas importam: subir sem porta
/// nenhuma numa rede exposta entrega o cerebro do dono a quem achar a URL;
/// subir sem porta nenhuma em localhost e so desenvolver.
fn conferir_a_porta(bind: &str) -> Result<(), String> {
    let local = bind == "127.0.0.1" || bind == "localhost" || bind == "::1";

    #[cfg(feature = "porta-aberta")]
    if !local {
        return Err(format!(
            "Compilado com `porta-aberta` e ouvindo em {bind}. Isso publica o M/OS \
             sem autenticacao nenhuma. Recompile com `--features passkey`."
        ));
    }

    #[cfg(not(any(feature = "passkey", feature = "porta-aberta")))]
    if !local {
        return Err(format!(
            "Sem porta compilada e ouvindo em {bind}. Recompile com `--features passkey`."
        ));
    }

    #[cfg(not(feature = "passkey"))]
    if local {
        eprintln!("[web] AVISO: sem autenticacao. So vale porque o bind e local.");
    }

    Ok(())
}
