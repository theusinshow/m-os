//! A Task criada num M/OS aparece como Task no outro.
//!
//! Os outros testes provam o LACO: a operacao sai, viaja e chega. Este prova o
//! que faltava para isso valer alguma coisa — que a operacao que chega vira uma
//! LINHA na tabela `tasks` do outro dispositivo, com o titulo certo, e que quem
//! le pelo caminho normal (`tasks()`) a encontra.
//!
//! Nenhuma operacao e montada a mao aqui. A Task e criada pela API de sempre,
//! `create_task`, e a emissao acontece por dentro — porque e assim que vai ser
//! no app, e um teste que monta a operacao a mao provaria o transporte e nao a
//! integracao.

use std::net::SocketAddr;

use mos_core::{NewTask, WorkRepository};
use mos_storage_sqlite::{ProjecaoSqlite, SqliteStorage};
use mos_sync::{
    carregar_relogio, sincronizar, Deposito, DeviceRepository, HlcClock, OutboxRepository,
};
use mos_sync_http::HttpTransport;
use mos_sync_server::{Estado, Hub};

const TOKEN: &str = "segredo-de-teste-com-tamanho-suficiente";

async fn servir() -> SocketAddr {
    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_sync_server::rotas(Estado::novo(Hub::em_memoria().unwrap(), TOKEN));
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    endereco
}

/// Um M/OS inteiro: banco real, migrations reais, emissao ligada.
struct Aparelho {
    _dir: tempfile::TempDir,
    storage: SqliteStorage,
    relogio: HlcClock,
    hora: i64,
}

impl Aparelho {
    fn novo(nome: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join("backups");
        std::fs::create_dir_all(&backups).unwrap();
        let storage = SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap();
        let device = storage.este_dispositivo(nome, "windows", "0.3.1").unwrap();
        // Sem isto o M/OS funciona igual e nao emite nada — a sincronizacao e
        // uma camada por cima, e nao um requisito para o sistema existir.
        storage.habilitar_sync(device.id).unwrap();
        let relogio = carregar_relogio(&storage, device.id).unwrap();
        Self {
            _dir: dir,
            storage,
            relogio,
            hora: 1_000,
        }
    }

    fn sincronizar(&mut self, transporte: &HttpTransport) -> mos_sync::Rodada {
        self.hora += 10;
        let mut projecao = ProjecaoSqlite::nova(&self.storage);
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
            &mut projecao,
            self.hora,
            100,
        )
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_task_criada_num_aparelho_vira_task_no_outro() {
    let endereco = servir().await;

    tokio::task::spawn_blocking(move || {
        let rede = HttpTransport::novo(format!("http://{endereco}"), TOKEN).unwrap();

        let mut pc = Aparelho::novo("PC");
        let mut outro = Aparelho::novo("Outro");

        // Pela API de sempre. Ninguem aqui sabe que existe sincronizacao.
        let nova = NewTask::create("Refatorar a navbar", "do projeto Escadas Minarum", None)
            .expect("Task valida");
        let id = nova.id;
        pc.storage.create_task(nova).unwrap();

        assert_eq!(
            pc.storage.pendentes(10).unwrap().len(),
            1,
            "criar a Task emitiu a operacao na mesma transacao"
        );

        let subida = pc.sincronizar(&rede);
        assert_eq!(subida.enviadas, 1);
        assert_eq!(subida.pendentes, 0);

        // O outro aparelho nao sabia que esta Task existia.
        assert!(outro.storage.tasks(false).unwrap().is_empty());

        let descida = outro.sincronizar(&rede);
        assert_eq!(descida.recebidas, 1);

        // E o teste que importa: a leitura NORMAL do outro M/OS encontra a Task.
        let tasks = outro.storage.tasks(false).unwrap();
        assert_eq!(tasks.len(), 1, "a operacao virou linha na tabela");
        assert_eq!(tasks[0].id, id);
        assert_eq!(tasks[0].title, "Refatorar a navbar");
        assert_eq!(tasks[0].description, "do projeto Escadas Minarum");
    })
    .await
    .unwrap();
}

/// Mover no Kanban de um lado e renomear do outro terminam com as DUAS coisas.
///
/// E o §8 exercitado pela API de verdade: `set_task_state` e `update_task`
/// emitem campos diferentes, e a reconciliacao por campo tem que preservar os
/// dois lados depois de atravessar a rede.
#[tokio::test(flavor = "multi_thread")]
async fn mover_num_aparelho_e_renomear_no_outro_convivem() {
    let endereco = servir().await;

    tokio::task::spawn_blocking(move || {
        let rede = HttpTransport::novo(format!("http://{endereco}"), TOKEN).unwrap();

        let mut pc = Aparelho::novo("PC");
        let mut outro = Aparelho::novo("Outro");

        let nova = NewTask::create("Titulo original", "", None).unwrap();
        let id = nova.id;
        pc.storage.create_task(nova).unwrap();
        pc.sincronizar(&rede);
        outro.sincronizar(&rede);
        assert_eq!(outro.storage.tasks(false).unwrap().len(), 1);

        // Cada aparelho mexe num campo diferente, sem se falarem.
        pc.storage.update_task(id, "Titulo novo", "", None).unwrap();
        outro
            .storage
            .set_task_state(id, mos_core::TaskState::Doing)
            .unwrap();

        outro.sincronizar(&rede);
        pc.sincronizar(&rede);
        outro.sincronizar(&rede);

        for (quem, aparelho) in [("pc", &pc), ("outro", &outro)] {
            let task = aparelho.storage.get_task(id).unwrap();
            assert_eq!(task.title, "Titulo novo", "{quem} perdeu o titulo");
            assert_eq!(
                task.state,
                mos_core::TaskState::Doing,
                "{quem} perdeu o movimento no Kanban"
            );
        }
    })
    .await
    .unwrap();
}
