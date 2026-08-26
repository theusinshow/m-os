//! Dois dispositivos, dois bancos SQLite, e um servidor HTTP de verdade no meio.
//!
//! O `sync_two_devices.rs` prova o laco contra o `HubLocal` em memoria. Este
//! prova o mesmo laco contra o hub que vai rodar na VPS, com o cliente que vai
//! rodar no aparelho — e o que ele checa e justamente o que aquele nao pode:
//! que serializar a operacao, mandar por HTTP, guardar em SQLite, devolver e
//! desserializar do outro lado nao perde nada pelo caminho.
//!
//! O motor nao sabe que mudou de transporte. Se soubesse, a fronteira estaria
//! no lugar errado.

use std::collections::BTreeMap;
use std::net::SocketAddr;

use mos_storage_sqlite::SqliteStorage;
use mos_sync::{
    aplicar, carregar_relogio, sincronizar, ClockRepository, Deposito, DeviceRepository, EntityRef,
    EstadoDaEntidade, HlcClock, Op, OpBody, OutboxRepository, Projecao, Resultado,
};
use mos_sync_http::HttpTransport;
use mos_sync_server::{Estado, Hub};
use serde_json::json;
use uuid::Uuid;

const TOKEN: &str = "segredo-de-teste-com-tamanho-suficiente";

/// Sobe o hub numa porta efemera. O `tokio` daqui e do TESTE, e nao do cliente:
/// o `HttpTransport` e bloqueante de proposito, e por isso cada dispositivo
/// roda numa thread propria.
async fn servir() -> SocketAddr {
    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_sync_server::rotas(Estado::novo(Hub::em_memoria().unwrap(), TOKEN));
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    endereco
}

// ------------------------------------------------------------- dispositivo

struct Dispositivo {
    _dir: tempfile::TempDir,
    storage: SqliteStorage,
    relogio: HlcClock,
    visao: Visao,
    hora: i64,
}

impl Dispositivo {
    fn novo(nome: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join("backups");
        std::fs::create_dir_all(&backups).unwrap();
        let storage = SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap();
        let device = storage.este_dispositivo(nome, "windows", "0.3.1").unwrap();
        let relogio = carregar_relogio(&storage, device.id).unwrap();
        Self {
            _dir: dir,
            storage,
            relogio,
            visao: Visao::default(),
            hora: 1_000,
        }
    }

    /// Uma mudanca local: grava a projecao e enfileira a operacao. Nenhuma das
    /// duas espera rede — e essa a promessa do local-first.
    fn mudar(&mut self, entidade: &EntityRef, campo: &str, valor: serde_json::Value) {
        self.hora += 10;
        let at = self.relogio.tick(self.hora);
        let op = Op::new(
            Uuid::now_v7(),
            entidade.clone(),
            OpBody::Update {
                fields: json!({ campo: valor }).as_object().unwrap().clone(),
            },
            at,
        );
        let base = self.visao.estado_de(&op);
        let reconciliado = aplicar(base, std::slice::from_ref(&op)).estado;
        self.visao.guardar(&op, &reconciliado).unwrap();
        self.storage.enfileirar(&op).unwrap();
        self.storage.guardar(self.relogio.ultimo()).unwrap();
    }

    fn sincronizar(&mut self, transporte: &HttpTransport) -> mos_sync::Rodada {
        self.hora += 10;
        let deposito = Deposito {
            outbox: &self.storage,
            conflitos: &self.storage,
            relogio: &self.storage,
            dispositivos: &self.storage,
        };
        sincronizar(
            &deposito,
            transporte,
            &mut self.relogio,
            &mut self.visao,
            self.hora,
            100,
        )
    }

    fn ver(&self, entidade: &EntityRef, campo: &str) -> Option<serde_json::Value> {
        self.visao
            .0
            .get(&(entidade.kind.as_str().to_owned(), entidade.id))
            .and_then(|estado| estado.campo(campo).cloned())
    }
}

#[derive(Clone, Default)]
struct Visao(BTreeMap<(String, Uuid), EstadoDaEntidade>);

impl Projecao for Visao {
    fn estado_de(&self, op: &Op) -> EstadoDaEntidade {
        self.0
            .get(&(op.entity.kind.as_str().to_owned(), op.entity.id))
            .cloned()
            .unwrap_or_default()
    }

    fn guardar(&mut self, op: &Op, estado: &EstadoDaEntidade) -> Resultado<()> {
        self.0.insert(
            (op.entity.kind.as_str().to_owned(), op.entity.id),
            estado.clone(),
        );
        Ok(())
    }
}

fn transporte(endereco: SocketAddr, token: &str) -> HttpTransport {
    HttpTransport::novo(format!("http://{endereco}"), token).unwrap()
}

// ------------------------------------------------------------------ testes

/// O teste que justifica o crate: criar de um lado aparece do outro, pela rede.
#[tokio::test(flavor = "multi_thread")]
async fn o_que_muda_no_pc_chega_no_outro_pela_rede() {
    let endereco = servir().await;

    // `spawn_blocking` porque o `HttpTransport` e bloqueante — e isto e o teste
    // da regra escrita no topo do crate, alem do teste do laco.
    tokio::task::spawn_blocking(move || {
        let rede = transporte(endereco, TOKEN);
        let tarefa = EntityRef::new("task", Uuid::now_v7());

        let mut pc = Dispositivo::novo("PC");
        let mut outro = Dispositivo::novo("Outro");

        pc.mudar(&tarefa, "titulo", json!("Refatorar a navbar"));
        let subida = pc.sincronizar(&rede);
        assert_eq!(subida.enviadas, 1);
        assert_eq!(subida.pendentes, 0, "confirmada, saiu da fila");

        let descida = outro.sincronizar(&rede);
        assert_eq!(descida.recebidas, 1);
        assert_eq!(
            outro.ver(&tarefa, "titulo"),
            Some(json!("Refatorar a navbar"))
        );
    })
    .await
    .unwrap();
}

/// Campos diferentes convivem: a reconciliacao por campo atravessa a rede
/// inteira. E o §8 — editar o titulo num aparelho e a data no outro tem que
/// resultar nas DUAS coisas.
#[tokio::test(flavor = "multi_thread")]
async fn campos_diferentes_convivem_atravessando_a_rede() {
    let endereco = servir().await;

    tokio::task::spawn_blocking(move || {
        let rede = transporte(endereco, TOKEN);
        let tarefa = EntityRef::new("task", Uuid::now_v7());

        let mut pc = Dispositivo::novo("PC");
        let mut outro = Dispositivo::novo("Outro");

        pc.mudar(&tarefa, "titulo", json!("Escadas Minarum"));
        pc.sincronizar(&rede);
        outro.sincronizar(&rede);

        // Cada um mexe num campo diferente, sem se falarem.
        pc.mudar(&tarefa, "titulo", json!("Escadas Minarum — bloco B"));
        outro.mudar(&tarefa, "prazo", json!("2026-09-10"));

        outro.sincronizar(&rede);
        pc.sincronizar(&rede);
        outro.sincronizar(&rede);

        for (quem, aparelho) in [("pc", &pc), ("outro", &outro)] {
            assert_eq!(
                aparelho.ver(&tarefa, "titulo"),
                Some(json!("Escadas Minarum — bloco B")),
                "{quem} perdeu o titulo"
            );
            assert_eq!(
                aparelho.ver(&tarefa, "prazo"),
                Some(json!("2026-09-10")),
                "{quem} perdeu o prazo"
            );
        }
    })
    .await
    .unwrap();
}

/// Offline nao perde nada: a fila segura, e a operacao sobe quando a rede volta.
#[tokio::test(flavor = "multi_thread")]
async fn o_que_foi_feito_offline_sobe_quando_a_rede_volta() {
    let endereco = servir().await;

    tokio::task::spawn_blocking(move || {
        let tarefa = EntityRef::new("task", Uuid::now_v7());
        let mut pc = Dispositivo::novo("PC");

        // Endereco errado: ninguem atende. E o que "sem rede" parece de dentro
        // do cliente.
        let caida = transporte("127.0.0.1:9".parse().unwrap(), TOKEN);
        pc.mudar(&tarefa, "titulo", json!("escrito no aviao"));
        let sem_rede = pc.sincronizar(&caida);
        assert_eq!(sem_rede.enviadas, 0);
        assert_eq!(sem_rede.pendentes, 1, "a fila segurou");
        assert!(sem_rede.erro.is_some());

        let rede = transporte(endereco, TOKEN);
        let voltou = pc.sincronizar(&rede);
        assert_eq!(voltou.enviadas, 1);
        assert_eq!(voltou.pendentes, 0);

        let mut outro = Dispositivo::novo("Outro");
        outro.sincronizar(&rede);
        assert_eq!(
            outro.ver(&tarefa, "titulo"),
            Some(json!("escrito no aviao"))
        );
    })
    .await
    .unwrap();
}

/// Credencial errada nao pede nova tentativa, e nao consome a fila.
///
/// As duas metades importam: insistir num 401 e bateria gasta para receber o
/// mesmo nao, e tirar da fila o que o hub recusou seria perder trabalho.
#[tokio::test(flavor = "multi_thread")]
async fn credencial_errada_para_a_rodada_e_preserva_a_fila() {
    let endereco = servir().await;

    tokio::task::spawn_blocking(move || {
        let errada = transporte(endereco, "token-que-nao-e-o-certo------------");
        let tarefa = EntityRef::new("task", Uuid::now_v7());

        let mut pc = Dispositivo::novo("PC");
        pc.mudar(&tarefa, "titulo", json!("nao devia subir"));
        let rodada = pc.sincronizar(&errada);

        assert_eq!(rodada.enviadas, 0);
        assert_eq!(rodada.pendentes, 1, "nada saiu da fila");
        assert!(rodada.erro.is_some());

        // E o hub continua vazio: um dispositivo novo com a credencial certa
        // nao encontra nada.
        let certa = transporte(endereco, TOKEN);
        let mut outro = Dispositivo::novo("Outro");
        assert_eq!(outro.sincronizar(&certa).recebidas, 0);
    })
    .await
    .unwrap();
}
