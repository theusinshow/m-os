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
use mos_storage_sqlite::SqliteStorage;
use mos_sync::{DeviceRepository, OutboxRepository};
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
        Self {
            _dir: dir,
            storage,
            hora: 1_000,
        }
    }

    /// A MESMA porta que o app usa.
    ///
    /// Montar o `Deposito` e a `Projecao` a mao aqui foi um erro real: o teste
    /// passava por um caminho que o M/OS nao usa, e por isso nao exercitava a
    /// retentativa de materializacao — a prova ficava invisivel e o teste dizia
    /// que estava tudo bem. Um teste que constroi o proprio caminho testa o
    /// caminho que ele construiu.
    fn sincronizar(&mut self, transporte: &HttpTransport) -> mos_sync::Rodada {
        self.sincronizar_com_limite(transporte, 100)
    }

    fn sincronizar_com_limite(
        &mut self,
        transporte: &HttpTransport,
        limite: usize,
    ) -> mos_sync::Rodada {
        self.hora += 10;
        self.storage
            .sincronizar_agora(transporte, self.hora, limite)
            .unwrap()
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

/// Os outros tipos, e nao so Task.
///
/// Cada um destes cobre uma armadilha diferente:
///
/// - **Capture** tem `CHECK (length(trim(content)) > 0)`. Um provisorio vazio
///   faria o `INSERT` ser recusado, e a Capture sumiria em vez de aparecer
///   incompleta.
/// - **Resource** e o unico que chega com `url` e `kind` decididos pelo
///   dominio, e nao digitados.
/// - **Prova** tem colunas NUMERICAS (`weight`, `score`, `max_score`) numa
///   tabela `STRICT`. Mandar `"5"` como texto para uma coluna `REAL` nao
///   arredonda: derruba a rodada inteira.
#[tokio::test(flavor = "multi_thread")]
async fn capture_resource_e_prova_atravessam_inteiros() {
    use mos_core::{
        AcademicRepository, CaptureRepository, CaptureSource, LifecycleState, NewCapture, NewExam,
        NewResource, NewSemester, NewSubject, ResourceKind, ResourceRepository,
    };

    let endereco = servir().await;

    tokio::task::spawn_blocking(move || {
        let rede = HttpTransport::novo(format!("http://{endereco}"), TOKEN).unwrap();
        let mut pc = Aparelho::novo("PC");
        let mut outro = Aparelho::novo("Outro");

        // ---- Capture
        let captura = NewCapture::create("tirar isso da cabeca", CaptureSource::Home).unwrap();
        let captura_id = captura.id;
        CaptureRepository::create(&pc.storage, captura).unwrap();

        // ---- Resource
        let recurso = NewResource::create(
            ResourceKind::Site,
            "Referencia de Web Design",
            "https://example.com/ref",
            "guardar para o Escadas Minarum",
            None,
        )
        .unwrap();
        let recurso_id = recurso.id;
        pc.storage.create_resource(recurso).unwrap();

        // ---- Prova, com os numeros
        let semestre =
            NewSemester::create("2026B2", "UNINTER", "2026-07-01", "2026-08-31").unwrap();
        let semestre_id = semestre.id;
        pc.storage.create_semester(semestre).unwrap();
        let disciplina =
            NewSubject::create(semestre_id, "Estatica dos Corpos", "906216", "", "", "").unwrap();
        let disciplina_id = disciplina.id;
        pc.storage.create_subject(disciplina).unwrap();
        let prova = NewExam::create(
            disciplina_id,
            "Prova 2",
            time::OffsetDateTime::now_utc(),
            "Sala 3",
            "treliças, cortante",
            2.5,
            Some(9.5),
            Some(10.0),
        )
        .unwrap();
        let prova_id = prova.id;
        pc.storage.create_exam(prova).unwrap();

        // Uma rodada leva tudo; outra traz tudo.
        let subida = pc.sincronizar(&rede);
        assert!(subida.erro.is_none(), "subida falhou: {:?}", subida.erro);
        let descida = outro.sincronizar(&rede);
        assert!(descida.erro.is_none(), "descida falhou: {:?}", descida.erro);

        let capturas = outro
            .storage
            .by_lifecycle(LifecycleState::Active, 50)
            .unwrap();
        assert_eq!(capturas.len(), 1, "a Capture nao materializou");
        assert_eq!(capturas[0].id, captura_id);
        assert_eq!(
            capturas[0].content, "tirar isso da cabeca",
            "o provisorio nao foi substituido pelo conteudo real"
        );

        let recursos = outro.storage.resources(false).unwrap();
        assert_eq!(recursos.len(), 1);
        assert_eq!(recursos[0].id, recurso_id);
        assert_eq!(recursos[0].title, "Referencia de Web Design");
        assert_eq!(recursos[0].url, "https://example.com/ref");

        let provas = outro.storage.exams(false).unwrap();
        assert_eq!(provas.len(), 1, "a Prova nao materializou");
        assert_eq!(provas[0].id, prova_id);
        assert_eq!(provas[0].name, "Prova 2");
        // Os numeros chegaram como NUMEROS.
        assert_eq!(provas[0].weight, 2.5);
        assert_eq!(provas[0].score, Some(9.5));
        assert_eq!(provas[0].max_score, Some(10.0));
    })
    .await
    .unwrap();
}

/// A aresta do Knowledge Graph, que nao e linha de tabela propria.
///
/// Duas coisas sao provadas aqui, e a segunda e a que justifica o desenho:
///
/// 1. ligar um Resource a um Project num aparelho liga no outro;
/// 2. **desligar e religar termina LIGADO**. `linked` e um campo, e o merge por
///    campo decide pelo instante. Se desligar fosse `OpBody::Delete`, a
///    semantica de "apagar ganha de editar" faria o contrario — certa para uma
///    Task, errada para um interruptor.
#[tokio::test(flavor = "multi_thread")]
async fn o_vinculo_atravessa_e_o_ultimo_gesto_vence() {
    use mos_core::{NewProject, NewResource, ResourceKind, ResourceRepository, WorkRepository};

    let endereco = servir().await;

    tokio::task::spawn_blocking(move || {
        let rede = HttpTransport::novo(format!("http://{endereco}"), TOKEN).unwrap();
        let mut pc = Aparelho::novo("PC");
        let mut outro = Aparelho::novo("Outro");

        let projeto = NewProject::create("Escadas Minarum", "", "").unwrap();
        let projeto_id = projeto.id;
        pc.storage.create_project(projeto).unwrap();
        let recurso = NewResource::create(
            ResourceKind::Site,
            "Memorial descritivo",
            "https://example.com/memorial",
            "",
            None,
        )
        .unwrap();
        let recurso_id = recurso.id;
        pc.storage.create_resource(recurso).unwrap();
        pc.storage
            .set_resource_project(recurso_id, projeto_id, true)
            .unwrap();

        let subida = pc.sincronizar(&rede);
        assert!(subida.erro.is_none(), "subida falhou: {:?}", subida.erro);
        let descida = outro.sincronizar(&rede);
        assert!(descida.erro.is_none(), "descida falhou: {:?}", descida.erro);

        let vinculos = outro.storage.resource_projects().unwrap();
        assert_eq!(vinculos.len(), 1, "o vinculo nao atravessou");
        assert_eq!(vinculos[0].resource_id, recurso_id);
        assert_eq!(vinculos[0].project_id, projeto_id);

        // O interruptor: desliga num aparelho, religa no outro DEPOIS.
        outro
            .storage
            .set_resource_project(recurso_id, projeto_id, false)
            .unwrap();
        outro.sincronizar(&rede);
        pc.sincronizar(&rede);
        assert!(
            pc.storage.resource_projects().unwrap().is_empty(),
            "desligar nao atravessou"
        );

        pc.storage
            .set_resource_project(recurso_id, projeto_id, true)
            .unwrap();
        pc.sincronizar(&rede);
        outro.sincronizar(&rede);

        for (quem, aparelho) in [("pc", &pc), ("outro", &outro)] {
            assert_eq!(
                aparelho.storage.resource_projects().unwrap().len(),
                1,
                "{quem}: religar tinha que terminar ligado — o ultimo gesto vence"
            );
        }
    })
    .await
    .unwrap();
}

/// Um clique esvazia a fila, e nao manda so um lote.
///
/// O botao diz "sincronizar". Uma passada do motor empurra UM lote e puxa UM
/// lote — com 370 na fila e limite 100, um clique deixava 270 para tras e a tela
/// mostrava o numero certo com a impressao errada: parecia que tinha acabado.
///
/// O limite aqui e 3 de proposito, para o laco precisar de varias passadas com
/// poucas entidades. O que se prova nao e o numero, e o fim: fila zerada de um
/// lado, tudo materializado do outro.
#[tokio::test(flavor = "multi_thread")]
async fn um_clique_esvazia_a_fila_inteira() {
    use mos_core::{NewTask, WorkRepository};

    let endereco = servir().await;

    tokio::task::spawn_blocking(move || {
        let rede = HttpTransport::novo(format!("http://{endereco}"), TOKEN).unwrap();
        let mut pc = Aparelho::novo("PC");
        let mut outro = Aparelho::novo("Outro");

        for i in 0..10 {
            let tarefa = NewTask::create(&format!("Tarefa {i}"), "", None).unwrap();
            pc.storage.create_task(tarefa).unwrap();
        }
        assert_eq!(
            pc.storage.quantidade_pendente().unwrap(),
            10,
            "as dez operacoes entraram na fila"
        );

        let subida = pc.sincronizar_com_limite(&rede, 3);
        assert!(subida.erro.is_none(), "subida falhou: {:?}", subida.erro);
        assert_eq!(subida.enviadas, 10, "o clique mandou tudo, e nao um lote");
        assert_eq!(subida.pendentes, 0, "a fila ficou vazia");

        let descida = outro.sincronizar_com_limite(&rede, 3);
        assert!(descida.erro.is_none(), "descida falhou: {:?}", descida.erro);
        assert_eq!(descida.recebidas, 10, "o clique trouxe tudo");
        assert!(!descida.tem_mais, "e sabe que acabou");
        assert_eq!(outro.storage.tasks(false).unwrap().len(), 10);
    })
    .await
    .unwrap();
}
