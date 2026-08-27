//! O laco que mantem esta superficie em dia.
//!
//! # Por que automatico aqui, e manual no desktop
//!
//! Nao e incoerencia. No desktop voce ve a tela e clica; no celular voce abre o
//! app por dez segundos para tirar uma ideia da cabeca e fecha. Um botao de
//! sincronizar ali seria uma tarefa a mais entre a ideia e o registro — que e
//! exatamente o atrito que esta superficie existe para remover.
//!
//! # O que ele NAO faz
//!
//! Nao reconcilia e nao decide nada: chama `sincronizar_agora`, o mesmo caminho
//! que o desktop usa, com o mesmo motor. Se um dia houver logica de sync aqui,
//! ela esta no lugar errado.

use std::sync::Arc;
use std::time::Duration;

use mos_storage_sqlite::SqliteStorage;

use crate::estado::Hub;

/// De quanto em quanto tempo uma rodada acontece sozinha.
///
/// Um minuto: o suficiente para o que voce escreveu no onibus estar no PC
/// quando voce chegar, e raro o bastante para nao ser um dreno. A rodada tambem
/// e disparada por escrita — ver `agora`.
const INTERVALO: Duration = Duration::from_secs(60);

/// Quantas operacoes por passada. O laco de `sincronizar_agora` repete ate
/// esvaziar, entao isto e o tamanho do lote, e nao o teto do que sobe.
const LIMITE: usize = 100;

/// Dispara uma rodada em segundo plano.
///
/// Chamado depois de cada escrita. A resposta ao usuario NAO espera por ele: a
/// captura ja esta gravada no banco local quando a tela responde, e a subida e
/// consequencia. Fazer o contrario ligaria "tirar da cabeca" a ter sinal.
pub fn agora(storage: Arc<SqliteStorage>, hub: Arc<Hub>) {
    tokio::task::spawn_blocking(move || {
        rodar(&storage, &hub);
    });
}

/// O laco de fundo.
pub fn iniciar(storage: Arc<SqliteStorage>, hub: Arc<Hub>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(INTERVALO).await;
            let storage = Arc::clone(&storage);
            let hub = Arc::clone(&hub);
            // `spawn_blocking` porque o transporte e bloqueante — chamar de
            // dentro de um worker do tokio derruba o processo na hora. Ver o
            // topo do `mos-sync-http`.
            let _ = tokio::task::spawn_blocking(move || rodar(&storage, &hub)).await;
        }
    });
}

fn rodar(storage: &SqliteStorage, hub: &Hub) {
    let transporte = match mos_sync_http::HttpTransport::novo(&hub.url, &hub.token) {
        Ok(transporte) => transporte,
        Err(causa) => {
            eprintln!("[web] transporte: {}", causa.mensagem);
            return;
        }
    };
    let agora_ms = (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
    match storage.sincronizar_agora(&transporte, agora_ms, LIMITE) {
        // O erro vai para o log e nao para a tela: aqui ninguem clicou, entao
        // nao ha a quem responder. O que a tela mostra e a fila — se ela nao
        // baixa, algo esta errado, e o log diz o que.
        Ok(rodada) => {
            if let Some(erro) = rodada.erro {
                eprintln!("[web] rodada parou: {erro}");
            } else if rodada.enviadas > 0 || rodada.recebidas > 0 {
                println!(
                    "[web] sync: {} enviadas, {} recebidas",
                    rodada.enviadas, rodada.recebidas
                );
            }
        }
        Err(causa) => eprintln!("[web] sync: {}", causa.message),
    }
}
