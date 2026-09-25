//! Escrever enquanto uma rodada de sincronizacao corre.
//!
//! # O defeito que este arquivo guarda
//!
//! O `SqliteStorage` tem dois cadeados, e os dois caminhos que mexem nos dois
//! os pegavam em ordem CONTRARIA:
//!
//! | caminho | pegava primeiro | depois |
//! |---|---|---|
//! | uma escrita que emite operacao | a conexao | o relogio logico |
//! | uma rodada de sincronizacao | o relogio logico | a conexao |
//!
//! Duas ordens contrarias sao um abraco mortal esperando o encontro, e o
//! encontro chegava sozinho: no `mos-web` uma rodada dispara depois de toda
//! escrita, entao bastava capturar duas coisas seguidas. O servidor prendia
//! **para sempre** — sem log, sem erro, sem 500 —, e continuava preso depois de
//! fechar e abrir o app, porque quem estava travado era o processo.
//!
//! Nada nos 326 testes do crate escrevia DURANTE uma rodada, e por isso o
//! defeito atravessou o projeto inteiro sem aparecer.
//!
//! # Por que o hub e lento de proposito
//!
//! O abraco so acontece se a escrita cair dentro da janela em que a rodada
//! segura o relogio. Com um hub instantaneo essa janela e de microssegundos, e
//! o teste passaria por sorte na maioria das vezes — que e a pior especie de
//! teste, porque ele fica verde ate o dia em que a maquina esta ocupada.
//!
//! O atraso no transporte alarga a janela para dezenas de milissegundos, e ai o
//! encontro deixa de ser sorte e vira certeza. Contra o codigo ANTES do portao,
//! isto trava na primeira volta.
//!
//! # Por que ha um cao de guarda
//!
//! Porque a falha deste teste nao e um `assert` que estoura: e um travamento. Um
//! teste que trava nao falha — ele pendura o CI ate o limite de tempo do
//! workflow, e o que aparece na tela nao diz nada sobre o que aconteceu. O canal
//! com prazo transforma o travamento numa mensagem.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mos_core::{CaptureRepository, CaptureSource, NewCapture};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::{DeviceRepository, Lote, Op, Resultado, Transport};
use uuid::Uuid;

/// Quanto o teste espera antes de declarar travamento.
///
/// Folgado de proposito: a maquina de CI e lenta e compartilhada, e um limite
/// apertado transformaria o guarda num gerador de falha intermitente — que
/// custaria mais confianca do que o defeito que ele vigia.
const PRAZO: Duration = Duration::from_secs(30);

/// Quanto tempo cada ida ao "servidor" demora.
const ATRASO: Duration = Duration::from_millis(40);

/// Quantas voltas de escrita concorrem com as rodadas.
const VOLTAS: usize = 40;

/// O outro lado, em memoria e devagar. Ver o cabecalho.
#[derive(Default)]
struct HubLento {
    log: Mutex<Vec<Op>>,
}

impl Transport for &HubLento {
    fn push(&self, _contrato: u32, ops: &[Op]) -> Resultado<Vec<Uuid>> {
        std::thread::sleep(ATRASO);
        let mut log = self.log.lock().unwrap();
        let mut aceitas = Vec::new();
        for op in ops {
            if !log.iter().any(|existente| existente.id == op.id) {
                log.push(op.clone());
            }
            aceitas.push(op.id);
        }
        Ok(aceitas)
    }

    fn pull(&self, _contrato: u32, cursor: &str, limite: usize) -> Resultado<Lote> {
        std::thread::sleep(ATRASO);
        let log = self.log.lock().unwrap();
        let de: usize = cursor.parse().unwrap_or(0);
        let ate = (de + limite).min(log.len());
        Ok(Lote {
            ops: log[de.min(log.len())..ate].to_vec(),
            proximo_cursor: ate.to_string(),
            tem_mais: ate < log.len(),
        })
    }
}

#[test]
fn escrever_durante_uma_rodada_nao_trava() {
    let (avisar, esperar) = mpsc::channel();

    // O trabalho corre numa thread propria para o cao de guarda poder desistir
    // dela. Um `join` aqui seria o mesmo travamento com outro nome.
    std::thread::spawn(move || {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join("backups");
        std::fs::create_dir_all(&backups).unwrap();
        let storage = Arc::new(SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap());
        let device = storage.este_dispositivo("PC", "windows", "0.3.1").unwrap();
        storage.habilitar_sync(device.id).unwrap();

        let hub = Arc::new(HubLento::default());

        // Uma semente na fila, para a primeira rodada ter o que empurrar — uma
        // rodada com a fila vazia mal segura o relogio, e nao serve de janela.
        storage
            .create(NewCapture::create("semente", CaptureSource::Home).unwrap())
            .unwrap();

        let sincronizador = {
            let storage = Arc::clone(&storage);
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || {
                for _ in 0..VOLTAS {
                    let agora =
                        (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
                    // O erro nao interessa aqui: o que este teste mede e se a
                    // chamada VOLTA, e nao o que ela devolve.
                    let _ = storage.sincronizar_agora(&hub.as_ref(), agora, 100);
                }
            })
        };

        // E, ao mesmo tempo, o aplicativo continua escrevendo — que e o uso
        // normal, e nao um caso de borda.
        for numero in 0..VOLTAS {
            storage
                .create(
                    NewCapture::create(&format!("ideia {numero}"), CaptureSource::QuickCapture)
                        .unwrap(),
                )
                .unwrap();
        }

        sincronizador.join().unwrap();

        // Travar nao e o unico jeito de perder escrita: as 40 mais a semente
        // precisam estar la.
        let guardadas = storage.inbox(200).unwrap();
        avisar.send(guardadas.len()).unwrap();
    });

    match esperar.recv_timeout(PRAZO) {
        Ok(quantas) => assert_eq!(
            quantas,
            VOLTAS + 1,
            "as escritas nao travaram, mas algumas se perderam"
        ),
        Err(_) => panic!(
            "escrita e rodada travaram uma na outra: {PRAZO:?} sem terminar. \
             E o abraco mortal entre a conexao e o relogio — ver SqliteStorage::portao."
        ),
    }
}

// ------------------------------------------------ F3.1: rede fora do portão

/// Um hub que DEMORA — o celular no 3G, o servidor atrás de um timeout.
struct HubTravado {
    atraso: Duration,
    log: Mutex<Vec<Op>>,
}

impl Transport for &HubTravado {
    fn push(&self, _contrato: u32, ops: &[Op]) -> Resultado<Vec<Uuid>> {
        std::thread::sleep(self.atraso);
        let mut log = self.log.lock().unwrap();
        for op in ops {
            if !log.iter().any(|existente| existente.id == op.id) {
                log.push(op.clone());
            }
        }
        Ok(ops.iter().map(|op| op.id).collect())
    }

    fn pull(&self, _contrato: u32, cursor: &str, limite: usize) -> Resultado<Lote> {
        std::thread::sleep(self.atraso);
        let log = self.log.lock().unwrap();
        let de: usize = cursor.parse().unwrap_or(0);
        let ate = (de + limite).min(log.len());
        Ok(Lote {
            ops: log[de.min(log.len())..ate].to_vec(),
            proximo_cursor: ate.to_string(),
            tem_mais: ate < log.len(),
        })
    }
}

/// Capturar não espera a rede. Com o hub levando 1,5 s por ida, uma captura
/// no meio da rodada precisa voltar em uma fração disso — antes, ela esperava
/// a rodada inteira. E nada se perde: a captura feita durante o envio sobe na
/// rodada seguinte, e o ack atrasado não confirma o que ele não levou.
#[test]
fn capturar_nao_espera_a_rede_da_rodada() {
    let dir = tempfile::tempdir().unwrap();
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = Arc::new(SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap());
    let device = storage.este_dispositivo("PC", "windows", "0.6.0").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    storage
        .create(NewCapture::create("semente", CaptureSource::Home).unwrap())
        .unwrap();
    let hub = Arc::new(HubTravado {
        atraso: Duration::from_millis(1500),
        log: Mutex::new(Vec::new()),
    });

    let rodada = {
        let storage = Arc::clone(&storage);
        let hub = Arc::clone(&hub);
        std::thread::spawn(move || {
            let agora = (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
            storage
                .sincronizar_agora(&hub.as_ref(), agora, 100)
                .unwrap()
        })
    };
    std::thread::sleep(Duration::from_millis(300));
    let inicio = std::time::Instant::now();
    storage
        .create(NewCapture::create("no meio da rodada", CaptureSource::QuickCapture).unwrap())
        .unwrap();
    let esperou = inicio.elapsed();
    let primeira = rodada.join().unwrap();
    assert!(
        esperou < Duration::from_millis(700),
        "a captura esperou {esperou:?} pela rede da rodada"
    );
    assert!(primeira.erro.is_none(), "{:?}", primeira.erro);
    assert_eq!(primeira.enviadas, 2);

    // Nada se perde e nada e confirmado sem ter ido: a captura do meio subiu
    // numa passada seguinte da mesma rodada, e a fila so zera porque TODAS as
    // operacoes estao no hub — o ack de um push nao confirma o que ele nao
    // levou.
    use mos_sync::OutboxRepository;
    assert_eq!(storage.quantidade_pendente().unwrap(), 0);
    let no_hub = hub.log.lock().unwrap().len();
    assert_eq!(no_hub, 2, "semente e captura do meio, cada uma uma vez");
    assert_eq!(storage.inbox(10).unwrap().len(), 2);
}
