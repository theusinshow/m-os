//! Transporte WebSocket.
//!
//! O socket vive numa task dedicada e conversa com o resto por canais. A ponte
//! nunca fala com a rede direto — ela fala com um `Sender` e um `Receiver`, e o
//! teste injeta a outra ponta. E por isso que os cenarios da spec rodam sem
//! tocar a rede e sem precisar de um trait so para poder fingir.

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::HermesError;

/// As duas pontas que a ponte usa.
#[derive(Debug)]
pub struct Channels {
    /// Frames a enviar ao gateway.
    pub outgoing: mpsc::Sender<String>,
    /// Frames recebidos do gateway. `None` significa socket encerrado.
    pub incoming: mpsc::Receiver<Result<String, HermesError>>,
}

/// Abre o socket e devolve os canais.
///
/// Fecha por conta propria quando o gateway encerra, e o `Receiver` seca — a
/// ponte trata isso como queda de conexao, nao como fim de turno.
pub async fn connect(url: &str) -> Result<Channels, HermesError> {
    let (socket, _) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(classify)?;
    let (mut sink, mut stream) = socket.split();

    let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(32);
    let (incoming_tx, incoming_rx) = mpsc::channel::<Result<String, HermesError>>(256);

    tokio::spawn(async move {
        while let Some(frame) = outgoing_rx.recv().await {
            // O terminador vai junto: um servidor que reusa o leitor de linha do
            // stdio nunca veria o request terminar sem ele.
            if sink
                .send(Message::text(format!("{frame}\n")))
                .await
                .is_err()
            {
                break;
            }
        }
        let _ = sink.close().await;
    });

    tokio::spawn(async move {
        while let Some(message) = stream.next().await {
            let forwarded = match message {
                // O protocolo e newline-delimited: um frame de texto pode
                // carregar mais de uma mensagem. Entregar o bloco inteiro ao
                // parser derrubava TODAS as mensagens do lote como "frame
                // ilegivel" — e um lote e justamente o que chega quando o
                // servidor emite varios eventos juntos.
                Ok(Message::Text(text)) => {
                    for line in text.lines().filter(|line| !line.trim().is_empty()) {
                        if incoming_tx.send(Ok(line.to_owned())).await.is_err() {
                            return;
                        }
                    }
                    continue;
                }
                // O gateway so manda texto; binario seria divergencia de
                // contrato, e vale dizer isso em vez de ignorar em silencio.
                Ok(Message::Binary(_)) => Err(HermesError::Protocol(
                    "o gateway mandou frame binario, que o contrato nao preve".into(),
                )),
                // O codigo de fechamento carrega o motivo, e jogar fora fazia
                // uma recusa de credencial (4401) ou de politica (4403) chegar
                // ao usuario como "o tunel nao esta aberto" — culpando a rede
                // por um problema de autenticacao.
                Ok(Message::Close(frame)) => {
                    if let Some(frame) = frame {
                        let code: u16 = frame.code.into();
                        let reported = match code {
                            4401 => Some(HermesError::Unauthorized(
                                "o gateway recusou o ticket ao fechar a conexao".into(),
                            )),
                            4403 => Some(HermesError::Gateway(
                                "O gateway recusou a conexao por politica (chat embutido desligado ou origem nao permitida).".into(),
                            )),
                            _ => None,
                        };
                        if let Some(error) = reported {
                            let _ = incoming_tx.send(Err(error)).await;
                        }
                    }
                    break;
                }
                Ok(_) => continue,
                Err(error) => Err(classify(error)),
            };
            if incoming_tx.send(forwarded).await.is_err() {
                break;
            }
        }
    });

    Ok(Channels {
        outgoing: outgoing_tx,
        incoming: incoming_rx,
    })
}

/// Traduz a falha do socket para algo que diz o que fazer.
///
/// 4401 e 4403 nao sao "erro de rede" e nao podem ser apresentados como tal:
/// um pede credencial, o outro e recusa de politica e nao se resolve com
/// repeticao.
fn classify(error: tokio_tungstenite::tungstenite::Error) -> HermesError {
    use tokio_tungstenite::tungstenite::Error;

    match &error {
        Error::Http(response) => match response.status().as_u16() {
            401 => HermesError::Unauthorized("o gateway recusou o ticket".into()),
            403 => HermesError::Gateway(
                "O gateway recusou a conexao por politica (chat embutido desligado ou origem nao permitida).".into(),
            ),
            other => HermesError::Gateway(format!("O gateway respondeu {other} no upgrade.")),
        },
        Error::Io(io) => HermesError::Unreachable(io.to_string()),
        Error::ConnectionClosed | Error::AlreadyClosed => {
            HermesError::Unreachable("o socket fechou".into())
        }
        Error::Protocol(detail) => HermesError::Protocol(detail.to_string()),
        other => HermesError::Gateway(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::tungstenite::Error;

    #[test]
    fn io_failure_points_at_the_tunnel() {
        let error = classify(Error::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "connection refused",
        )));
        assert!(matches!(error, HermesError::Unreachable(_)));
        assert!(error.to_string().contains("tunel"));
    }

    #[tokio::test]
    async fn refuses_to_connect_when_nothing_listens() {
        // Porta reservada para testes, sem servico. Tunel fechado tem que
        // chegar como Unreachable, nao como falha generica.
        let error = connect("ws://127.0.0.1:9/api/ws?ticket=x")
            .await
            .unwrap_err();
        assert!(matches!(error, HermesError::Unreachable(_)));
    }
}
