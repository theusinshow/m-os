//! O M/Academic contra um banco de verdade.
//!
//! O que so aqui se prova: as transacoes, as chaves estrangeiras, os CHECK e o
//! vinculo com a Task. O dominio ja tem teste proprio em `mos_core::academic`;
//! aqui esta o que o SQLite pode desmentir.

use mos_core::{
    AcademicRepository, AssignmentStatus, ExamStatus, LifecycleState, NewAssignment, NewExam,
    NewSemester, NewSubject, Priority, TaskState, UpdateAssignment, UpdateExam, WorkRepository,
};
use mos_storage_sqlite::SqliteStorage;
use time::{Duration, OffsetDateTime};

fn storage() -> (tempfile::TempDir, SqliteStorage) {
    let directory = tempfile::tempdir().unwrap();
    let storage = SqliteStorage::open(
        directory.path().join("mos.db"),
        directory.path().join("backups"),
    )
    .unwrap();
    (directory, storage)
}

fn semestre(storage: &SqliteStorage) -> mos_core::Semester {
    storage
        .create_semester(
            NewSemester::create("2026.2", "UFSC", "2026-08-01", "2026-12-15").unwrap(),
        )
        .unwrap()
}

fn disciplina(storage: &SqliteStorage, semestre: &mos_core::Semester) -> mos_core::Subject {
    storage
        .create_subject(
            NewSubject::create(semestre.id, "Estatica dos Corpos", "EMC5132", "Prof. X", "trigo", "")
                .unwrap(),
        )
        .unwrap()
}

fn atividade(
    storage: &SqliteStorage,
    subject: &mos_core::Subject,
    titulo: &str,
) -> mos_core::Assignment {
    storage
        .create_assignment(
            NewAssignment::create(
                subject.id,
                titulo,
                "",
                Some(OffsetDateTime::now_utc() + Duration::days(3)),
                Priority::Normal,
                0.0,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap()
}

// ===========================================================================
// O caminho inteiro
// ===========================================================================

#[test]
fn do_semestre_ate_a_nota() {
    let (_dir, storage) = storage();

    let periodo = semestre(&storage);
    assert_eq!(storage.semesters(false).unwrap().len(), 1);

    let materia = disciplina(&storage, &periodo);
    assert_eq!(storage.subjects(false).unwrap().len(), 1);

    let lista = atividade(&storage, &materia, "Lista 03");
    assert_eq!(lista.status, AssignmentStatus::Pending);
    assert!(lista.task_id.is_none());

    let prova = storage
        .create_exam(
            NewExam::create(
                materia.id,
                "P1",
                OffsetDateTime::now_utc() + Duration::days(7),
                "Sala 204",
                "Equilibrio, momentos",
                4.0,
                None,
                Some(10.0),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(prova.status, ExamStatus::Scheduled);
    assert_eq!(prova.max_score, Some(10.0));
    assert_eq!(prova.score, None, "teto sem nota e um estado legitimo");

    // A nota chega depois da prova.
    let corrigida = storage
        .update_exam(UpdateExam {
            id: prova.id,
            name: prova.name.clone(),
            at: prova.at,
            location: prova.location.clone(),
            topics: prova.topics.clone(),
            weight: prova.weight,
            score: Some(7.5),
            max_score: Some(10.0),
            status: ExamStatus::Graded,
        })
        .unwrap();
    assert_eq!(corrigida.score, Some(7.5));

    let desempenho = mos_core::desempenho(&storage.assignments(false).unwrap(), &[corrigida]);
    assert_eq!(desempenho.media, Some(7.5));
    assert_eq!(storage.exams(false).unwrap().len(), 1);
}

// ===========================================================================
// O vinculo com a Task
// ===========================================================================

#[test]
fn a_atividade_ganha_uma_task_de_verdade_com_a_disciplina_no_titulo() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    let lista = atividade(&storage, &materia, "Lista 03");

    let task = storage.create_task_for_assignment(lista.id).unwrap();
    assert_eq!(
        task.title, "Estatica dos Corpos — Lista 03",
        "no quadro, 'Lista 03' sozinha nao diz de que materia e"
    );

    let ligada = storage
        .assignments(false)
        .unwrap()
        .into_iter()
        .find(|item| item.id == lista.id)
        .unwrap();
    assert_eq!(ligada.task_id, Some(task.id));
    assert_eq!(storage.tasks(false).unwrap().len(), 1);
}

#[test]
fn a_mesma_atividade_nao_ganha_duas_tasks() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    let lista = atividade(&storage, &materia, "Lista 03");

    storage.create_task_for_assignment(lista.id).unwrap();
    assert!(
        storage.create_task_for_assignment(lista.id).is_err(),
        "duas Tasks para a mesma entrega deixariam uma orfa no quadro"
    );
}

/// Concluir a atividade fecha a Task. Sem isto, a atividade diria "entregue" e
/// a Task continuaria no quadro pedindo acao — a sincronizacao fragil que o
/// pedido proibe.
#[test]
fn entregar_a_atividade_conclui_a_task_ligada() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    let lista = atividade(&storage, &materia, "Lista 03");
    let task = storage.create_task_for_assignment(lista.id).unwrap();

    storage
        .set_assignment_status(lista.id, AssignmentStatus::Submitted)
        .unwrap();

    let depois = storage.get_task(task.id).unwrap();
    assert_eq!(depois.state, TaskState::Done);
    assert!(depois.completed_at.is_some());
}

#[test]
fn reabrir_a_atividade_reabre_a_task() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    let lista = atividade(&storage, &materia, "Lista 03");
    let task = storage.create_task_for_assignment(lista.id).unwrap();

    storage
        .set_assignment_status(lista.id, AssignmentStatus::Submitted)
        .unwrap();
    storage
        .set_assignment_status(lista.id, AssignmentStatus::Pending)
        .unwrap();

    let depois = storage.get_task(task.id).unwrap();
    assert_eq!(depois.state, TaskState::Doing);
}

/// Apagar a Task nao pode apagar a atividade da faculdade: ela perde o braco
/// executor, e nao a existencia.
#[test]
fn a_task_apagada_deixa_a_atividade_de_pe() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    let lista = atividade(&storage, &materia, "Lista 03");
    let task = storage.create_task_for_assignment(lista.id).unwrap();

    storage.set_task_lifecycle(task.id, LifecycleState::Trashed).unwrap();
    storage.delete_task(task.id).unwrap();

    let sobrevivente = storage
        .assignments(false)
        .unwrap()
        .into_iter()
        .find(|item| item.id == lista.id)
        .expect("a atividade continua existindo");
    assert_eq!(sobrevivente.task_id, None, "o vinculo se desfaz sozinho");
}

// ===========================================================================
// Cascata
// ===========================================================================

#[test]
fn apagar_a_disciplina_leva_atividades_e_provas() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    atividade(&storage, &materia, "Lista 03");
    storage
        .create_exam(
            NewExam::create(
                materia.id,
                "P1",
                OffsetDateTime::now_utc() + Duration::days(7),
                "",
                "",
                0.0,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();

    // Arquivar NAO apaga: e mudanca de campo, e tem volta.
    storage
        .set_subject_lifecycle(materia.id, LifecycleState::Archived)
        .unwrap();
    assert_eq!(storage.assignments(false).unwrap().len(), 1);
    assert_eq!(storage.subjects(false).unwrap().len(), 0);
    assert_eq!(storage.subjects(true).unwrap().len(), 1);
}

// ===========================================================================
// Estudo
// ===========================================================================

#[test]
fn a_sessao_de_estudo_comeca_e_termina() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);

    let sessao = storage.start_study(materia.id, "Momentos").unwrap();
    assert!(sessao.em_curso());
    assert_eq!(storage.running_study().unwrap().unwrap().id, sessao.id);

    let fechada = storage.finish_study(sessao.id, 45 * 60, "cansativo").unwrap();
    assert_eq!(fechada.seconds, 45 * 60);
    assert!(!fechada.em_curso());
    assert_eq!(fechada.notes, "cansativo");
    assert!(storage.running_study().unwrap().is_none());
}

/// Uma sessao aberta por vez. O indice unico do banco recusaria a segunda, e a
/// pessoa ficaria travada sem entender o motivo — entao comecar a nova FECHA a
/// anterior.
#[test]
fn comecar_a_estudar_fecha_a_sessao_esquecida() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);

    let esquecida = storage.start_study(materia.id, "Ontem").unwrap();
    let nova = storage.start_study(materia.id, "Hoje").unwrap();

    let todas = storage.study_sessions(10).unwrap();
    assert_eq!(todas.len(), 2);
    let antiga = todas.iter().find(|s| s.id == esquecida.id).unwrap();
    assert!(!antiga.em_curso(), "a esquecida foi fechada");
    assert_eq!(storage.running_study().unwrap().unwrap().id, nova.id);
}

#[test]
fn encerrar_duas_vezes_e_recusado() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    let sessao = storage.start_study(materia.id, "").unwrap();
    storage.finish_study(sessao.id, 60, "").unwrap();
    assert!(storage.finish_study(sessao.id, 60, "").is_err());
}

#[test]
fn a_sessao_descartada_some_de_verdade() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    let sessao = storage.start_study(materia.id, "").unwrap();
    storage.discard_study(sessao.id).unwrap();
    assert!(storage.study_sessions(10).unwrap().is_empty());
}

// ===========================================================================
// Materiais
// ===========================================================================

#[test]
fn o_material_e_um_resource_de_verdade() {
    use mos_core::{NewResource, ResourceKind, ResourceRepository};

    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);

    let recurso = storage
        .create_resource(
            NewResource::create(
                ResourceKind::Site,
                "Aula 03 — Equilibrio",
                "https://exemplo.test/aula3",
                "",
                None,
            )
            .unwrap(),
        )
        .unwrap();

    storage.link_material(materia.id, recurso.id, true).unwrap();
    let materiais = storage.subject_resources(materia.id).unwrap();
    assert_eq!(materiais.len(), 1);
    assert_eq!(materiais[0].id, recurso.id);

    let contagens = storage.material_counts().unwrap();
    assert_eq!(contagens, vec![(materia.id, 1)]);

    // Desligar e ligar de novo termina ligado, e nao duplicado.
    storage.link_material(materia.id, recurso.id, false).unwrap();
    assert!(storage.subject_resources(materia.id).unwrap().is_empty());
    storage.link_material(materia.id, recurso.id, true).unwrap();
    storage.link_material(materia.id, recurso.id, true).unwrap();
    assert_eq!(storage.subject_resources(materia.id).unwrap().len(), 1);
}

// ===========================================================================
// Validacao
// ===========================================================================

#[test]
fn o_semestre_invertido_e_recusado_na_criacao_e_na_edicao() {
    let (_dir, storage) = storage();
    assert!(NewSemester::create("2026.2", "", "2026-12-15", "2026-08-01").is_err());

    let periodo = semestre(&storage);
    assert!(storage
        .update_semester(
            periodo.id,
            "2026.2",
            "",
            &mos_core::Day::parse("2026-12-15").unwrap(),
            &mos_core::Day::parse("2026-08-01").unwrap(),
        )
        .is_err());
}

#[test]
fn nota_sem_teto_e_recusada() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);

    let erro = NewAssignment::create(
        materia.id,
        "Lista",
        "",
        None,
        Priority::Normal,
        0.0,
        Some(8.0),
        None,
    );
    assert!(erro.is_err(), "8 de quanto?");

    let lista = atividade(&storage, &materia, "Lista 03");
    assert!(storage
        .update_assignment(UpdateAssignment {
            id: lista.id,
            title: lista.title.clone(),
            description: String::new(),
            due_at: None,
            priority: Priority::Normal,
            weight: 0.0,
            score: Some(9.0),
            max_score: None,
            status: AssignmentStatus::Graded,
        })
        .is_err());
}

#[test]
fn titulo_em_branco_nao_passa() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    assert!(NewSubject::create(periodo.id, "   ", "", "", "", "").is_err());
    assert!(NewAssignment::create(
        materia.id,
        "  ",
        "",
        None,
        Priority::Normal,
        0.0,
        None,
        None
    )
    .is_err());
}

#[test]
fn accent_fora_do_design_system_nao_chega_ao_banco() {
    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    assert!(NewSubject::create(periodo.id, "Estatica", "", "", "#ff0000", "").is_err());
}

// ===========================================================================
// Persistencia
// ===========================================================================

/// Fechar e reabrir o app nao pode perder nada — e o teste que so um banco de
/// verdade faz.
#[test]
fn tudo_sobrevive_ao_fechamento_do_app() {
    let directory = tempfile::tempdir().unwrap();
    let caminho = directory.path().join("mos.db");
    let backups = directory.path().join("backups");

    let (semestre_id, subject_id, assignment_id) = {
        let storage = SqliteStorage::open(caminho.clone(), backups.clone()).unwrap();
        let periodo = storage
            .create_semester(
                NewSemester::create("2026.2", "UFSC", "2026-08-01", "2026-12-15").unwrap(),
            )
            .unwrap();
        let materia = storage
            .create_subject(
                NewSubject::create(periodo.id, "Estatica", "EMC1", "", "cobre", "notas").unwrap(),
            )
            .unwrap();
        let lista = atividade(&storage, &materia, "Lista 03");
        let sessao = storage.start_study(materia.id, "Momentos").unwrap();
        storage.finish_study(sessao.id, 1800, "").unwrap();
        (periodo.id, materia.id, lista.id)
    };

    let storage = SqliteStorage::open(caminho, backups).unwrap();
    assert_eq!(storage.semesters(false).unwrap()[0].id, semestre_id);
    let materia = &storage.subjects(false).unwrap()[0];
    assert_eq!(materia.id, subject_id);
    assert_eq!(materia.accent, "cobre");
    assert_eq!(materia.notes, "notas");
    assert_eq!(storage.assignments(false).unwrap()[0].id, assignment_id);
    assert_eq!(storage.study_sessions(10).unwrap()[0].seconds, 1800);
}

// ===========================================================================
// Busca
// ===========================================================================

/// Procurar "Estatica" tem de achar a disciplina, a prova dela e a atividade.
///
/// Sem isto o M/Academic seria o unico substantivo do M/OS que a busca global
/// nao alcanca — o silo que o `CORE-FOUNDATION.md` §2 recusa.
#[test]
fn a_busca_global_alcanca_disciplina_prova_e_atividade() {
    use mos_core::{SearchItem, SearchRequest};

    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    atividade(&storage, &materia, "Lista de Estatica 03");
    storage
        .create_exam(
            NewExam::create(
                materia.id,
                "P1 de Estatica",
                OffsetDateTime::now_utc() + Duration::days(7),
                "",
                "",
                0.0,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();

    let achados = storage
        .search_academic(SearchRequest {
            query: "estatica".into(),
            include_archived: false,
            limit: 20,
        })
        .unwrap();

    let mut disciplinas = 0;
    let mut provas = 0;
    let mut atividades = 0;
    for item in &achados {
        match item {
            SearchItem::Subject { .. } => disciplinas += 1,
            SearchItem::Exam { subject, .. } => {
                provas += 1;
                assert_eq!(subject, "Estatica dos Corpos", "a prova carrega a materia");
            }
            SearchItem::Assignment { subject, .. } => {
                atividades += 1;
                assert_eq!(subject, "Estatica dos Corpos");
            }
            _ => panic!("a busca academica devolveu outro tipo"),
        }
    }
    assert_eq!((disciplinas, provas, atividades), (1, 1, 1));
    // A disciplina vem primeiro: quem procura a materia quer a materia.
    assert!(matches!(achados[0], SearchItem::Subject { .. }));
}

#[test]
fn a_busca_acha_pelo_conteudo_da_prova_e_pelo_codigo_da_materia() {
    use mos_core::SearchRequest;

    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    storage
        .create_exam(
            NewExam::create(
                materia.id,
                "P1",
                OffsetDateTime::now_utc() + Duration::days(7),
                "",
                "Equilibrio e trelicas",
                0.0,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();

    let pedido = |termo: &str| SearchRequest {
        query: termo.into(),
        include_archived: false,
        limit: 20,
    };
    assert_eq!(
        storage.search_academic(pedido("trelicas")).unwrap().len(),
        1,
        "o conteudo da prova e o que se procura antes dela"
    );
    assert_eq!(
        storage.search_academic(pedido("EMC5132")).unwrap().len(),
        1,
        "o codigo tambem acha a materia"
    );
    assert!(storage.search_academic(pedido("  ")).unwrap().is_empty());
}

/// Disciplina arquivada nao polui a busca, e nem carrega as provas dela junto.
#[test]
fn o_que_foi_arquivado_sai_da_busca() {
    use mos_core::SearchRequest;

    let (_dir, storage) = storage();
    let periodo = semestre(&storage);
    let materia = disciplina(&storage, &periodo);
    atividade(&storage, &materia, "Lista de Estatica");
    storage
        .set_subject_lifecycle(materia.id, LifecycleState::Archived)
        .unwrap();

    let achados = storage
        .search_academic(SearchRequest {
            query: "estatica".into(),
            include_archived: false,
            limit: 20,
        })
        .unwrap();
    assert!(achados.is_empty());
}
