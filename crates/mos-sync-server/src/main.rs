//! O hub como processo.
//!
//! Configuracao inteira por ambiente, e nenhum padrao para o segredo: um
//! servidor que sobe com token embutido sobe inseguro, e "eu troco depois" e
//! como toda porta aberta comeca. Sem `MOS_SYNC_TOKEN` ele recusa a subir e diz
//! o que falta.

use std::net::SocketAddr;

use mos_sync_server::{rotas, Estado, Hub};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = match std::env::var("MOS_SYNC_TOKEN") {
        Ok(valor) if valor.len() >= 32 => valor,
        Ok(_) => {
            eprintln!(
                "MOS_SYNC_TOKEN curto demais. Use ao menos 32 caracteres — este segredo e a \
                 unica coisa entre a internet e o seu banco."
            );
            std::process::exit(2);
        }
        Err(_) => {
            eprintln!("Faltou MOS_SYNC_TOKEN. Gere um segredo e exporte antes de subir.");
            std::process::exit(2);
        }
    };

    let banco = std::env::var("MOS_SYNC_DB").unwrap_or_else(|_| String::from("mos-sync.db"));
    let porta: u16 = std::env::var("MOS_SYNC_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9120);

    // 127.0.0.1 por padrao, e nao 0.0.0.0: o M/OS ja fala com a VPS por tunel
    // SSH (ver a copy do Hermes em Settings), e um hub que abre porta publica
    // por descuido e um banco pessoal exposto. Quem quiser expor diz isso em
    // voz alta pelo `MOS_SYNC_BIND`.
    let bind = std::env::var("MOS_SYNC_BIND").unwrap_or_else(|_| String::from("127.0.0.1"));
    let endereco: SocketAddr = format!("{bind}:{porta}").parse()?;

    let hub = Hub::abrir(&banco)?;
    println!("[sync] hub em {banco}, ouvindo {endereco}");

    let ouvinte = tokio::net::TcpListener::bind(endereco).await?;
    axum::serve(ouvinte, rotas(Estado::novo(hub, token)))
        .with_graceful_shutdown(desligar())
        .await?;
    Ok(())
}

/// Termina a rodada em andamento antes de sair.
///
/// Sem isto, um `systemctl restart` no meio de um `push` derruba a conexao
/// depois de o cliente ter enviado e antes de ele receber a confirmacao — e o
/// cliente, corretamente, reenvia. Funciona, mas gasta uma rodada por reinicio
/// sem precisar.
async fn desligar() {
    let _ = tokio::signal::ctrl_c().await;
    println!("[sync] encerrando.");
}
