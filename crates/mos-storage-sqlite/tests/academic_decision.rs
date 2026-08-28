//! A decisao da pessoa, contra um banco de verdade.
//!
//! O que so aqui se prova: que "ja entreguei" sobrevive a proxima
//! sincronizacao, que "nao vou fazer" nao volta a cobrar, e que o plano de
//! execucao e do M/OS mesmo quando o prazo e do portal.

use mos_core::academic_sync::{
    ExternalAssignment, ExternalAssignmentStatus, ExternalSemester, ExternalSubject,
    ProviderSnapshot, PROVIDER_UNIVIRTUS,
};
use mos_core::{
    AcademicRepository, AssignmentStatus, Day, Decision, NewAssignment, NewExam, NewSemester,
    NewSubject, Plano, Priority,
};
use mos_storage_sqlite::{AcademicProviderRepository, SqliteStorage};
use time::macros::datetime;
use time::OffsetDateTime;

fn storage() -> (tempfile::TempDir, SqliteStorage) {
    let directory = tempfile::tempdir().unwrap();
    let storage = SqliteStorage::open(
        directory.path().join("mos.db"),
        directory.path().join("backups"),
    )
    .unwrap();
    (directory, storage)
}

fn cenario(storage: &SqliteStorage) -> (mos_core::Subject, mos_core::Assignment, mos_core::Exam) {
    let semestre = storage
        .create_semester(
            NewSemester::create("2026B2", "UNINTER", "2026-07-01", "2026-08-31").unwrap(),
        )
        .unwrap();
    let materia = storage
        .create_subject(
            NewSubject::create(
                semestre.id,
                "Estatica dos Corpos",
                "906216",
                "",
                "trigo",
                "",
            )
            .unwrap(),
        )
        .unwrap();
    let atividade = storage
        .create_assignment(
            NewAssignment::create(
                materia.id,
                "APOL 3",
                "",
                Some(datetime!(2026-08-24 23:59 UTC)),
                Priority::Normal,
                15.0,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    let prova = storage
        .create_exam(
            NewExam::create(
                materia.id,
                "Prova Objetiva",
                datetime!(2026-09-14 23:59 UTC),
                "",
                "",
                30.0,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();
    (materia, atividade, prova)
}

// ===========================================================================
// O ciclo da decisao
// ===========================================================================

#[test]
fn a_decisao_nasce_indefinida() {
    let (_dir, storage) = storage();
    let (_m, atividade, prova) = cenario(&storage);
    assert_eq!(atividade.decision, Decision::None);
    assert!(atividade.decided_at.is_none());
    assert_eq!(prova.decision, Decision::None);
}

#[test]
fn ja_entreguei_marca_e_carimba_a_hora() {
    let (_dir, storage) = storage();
    let (_m, atividade, _p) = cenario(&storage);
    let depois = storage
        .set_assignment_decision(atividade.id, Decision::Done)
        .unwrap();
    assert_eq!(depois.decision, Decision::Done);
    assert!(
        depois.decided_at.is_some(),
        "a hora da decisao vira historico"
    );
}

#[test]
fn nao_vou_fazer_e_reversivel() {
    let (_dir, storage) = storage();
    let (_m, atividade, _p) = cenario(&storage);
    storage
        .set_assignment_decision(atividade.id, Decision::Skipped)
        .unwrap();
    let volta = storage
        .set_assignment_decision(atividade.id, Decision::None)
        .unwrap();
    assert_eq!(volta.decision, Decision::None);
    assert!(
        volta.decided_at.is_none(),
        "desfazer apaga a hora: guardar a hora de uma decisao que nao existe mais \
         faria o historico contar um evento que nao houve"
    );
}

#[test]
fn os_quatro_caminhos_da_decisao_funcionam() {
    let (_dir, storage) = storage();
    let (_m, atividade, _p) = cenario(&storage);
    for (de, para) in [
        (Decision::None, Decision::Done),
        (Decision::Done, Decision::None),
        (Decision::None, Decision::Skipped),
        (Decision::Skipped, Decision::None),
    ] {
        storage.set_assignment_decision(atividade.id, de).unwrap();
        let depois = storage.set_assignment_decision(atividade.id, para).unwrap();
        assert_eq!(depois.decision, para);
    }
}

#[test]
fn a_prova_tambem_aceita_decisao() {
    let (_dir, storage) = storage();
    let (_m, _a, prova) = cenario(&storage);
    let depois = storage.set_exam_decision(prova.id, Decision::Done).unwrap();
    assert_eq!(depois.decision, Decision::Done);
}

// ===========================================================================
// A decisao nao mexe no fato academico
// ===========================================================================

/// "Ja entreguei" e uma frase sobre MIM, e nao sobre o portal. O `status`
/// continua dizendo o que o Univirtus registra.
#[test]
fn decidir_nao_altera_o_status_do_portal() {
    let (_dir, storage) = storage();
    let (_m, atividade, _p) = cenario(&storage);
    assert_eq!(atividade.status, AssignmentStatus::Pending);
    let depois = storage
        .set_assignment_decision(atividade.id, Decision::Done)
        .unwrap();
    assert_eq!(
        depois.status,
        AssignmentStatus::Pending,
        "o status e do portal; a decisao e minha"
    );
    assert_eq!(depois.decision, Decision::Done);
}

// ===========================================================================
// O plano
// ===========================================================================

#[test]
fn planejar_grava_quando_e_por_quanto_tempo() {
    let (_dir, storage) = storage();
    let (_m, atividade, _p) = cenario(&storage);
    let plano = Plano::novo(datetime!(2026-08-22 19:30 UTC), 60).unwrap();
    let depois = storage.plan_assignment(atividade.id, Some(plano)).unwrap();
    assert_eq!(depois.planned_at, Some(datetime!(2026-08-22 19:30 UTC)));
    assert_eq!(depois.planned_minutes, 60);
}

/// O plano e o prazo sao duas datas diferentes, e confundi-las e o erro que faz
/// o calendario mostrar "entregar APOL" as 23h59 quando a pessoa vai escrever
/// na quarta.
#[test]
fn o_plano_nao_toca_no_prazo() {
    let (_dir, storage) = storage();
    let (_m, atividade, _p) = cenario(&storage);
    let plano = Plano::novo(datetime!(2026-08-22 19:30 UTC), 60).unwrap();
    let depois = storage.plan_assignment(atividade.id, Some(plano)).unwrap();
    assert_eq!(depois.due_at, Some(datetime!(2026-08-24 23:59 UTC)));
    assert_ne!(depois.planned_at, depois.due_at);
}

#[test]
fn desplanejar_limpa_data_e_duracao() {
    let (_dir, storage) = storage();
    let (_m, atividade, _p) = cenario(&storage);
    storage
        .plan_assignment(
            atividade.id,
            Some(Plano::novo(datetime!(2026-08-22 19:30 UTC), 60).unwrap()),
        )
        .unwrap();
    let limpo = storage.plan_assignment(atividade.id, None).unwrap();
    assert!(limpo.planned_at.is_none());
    assert_eq!(limpo.planned_minutes, 0);
}

// ===========================================================================
// A regra central: o sync nao apaga decisao
// ===========================================================================

fn retrato_com_atividade(decisao_do_portal: ExternalAssignmentStatus) -> ProviderSnapshot {
    ProviderSnapshot {
        provider: PROVIDER_UNIVIRTUS.into(),
        context: None,
        semesters: vec![ExternalSemester {
            external_id: "2026B2".into(),
            name: "2026B2".into(),
            institution: "UNINTER".into(),
            starts_on: Day::parse("2026-07-01").unwrap(),
            ends_on: Day::parse("2026-08-31").unwrap(),
            current: true,
        }],
        subjects: vec![ExternalSubject {
            external_id: "906216".into(),
            semester_external_id: "2026B2".into(),
            name: "Estatica dos Corpos".into(),
            code: "906216".into(),
            teacher: String::new(),
            situation: "EM CURSO".into(),
            official_grade: None,
        }],
        assessments: Vec::new(),
        assignments: vec![ExternalAssignment {
            external_id: "352876:394147".into(),
            subject_external_id: "906216".into(),
            title: "Atividade Pratica".into(),
            description: "Trabalho".into(),
            due_at: Some(datetime!(2026-08-24 23:59 UTC)),
            submitted_at: None,
            weight: 0.0,
            max_score: None,
            score: None,
            status: decisao_do_portal,
        }],
        materials: Vec::new(),
        warnings: Vec::new(),
    }
}

/// O caso que a camada inteira existe para resolver: a pessoa entrega as 23h, o
/// portal so atualiza no dia seguinte, e ate la o M/OS nao pode continuar
/// cobrando.
#[test]
fn ja_entreguei_sobrevive_ao_sync_que_ainda_diz_pendente() {
    let (_dir, storage) = storage();
    let retrato = retrato_com_atividade(ExternalAssignmentStatus::Pending);
    storage
        .apply_provider_snapshot(&retrato, OffsetDateTime::now_utc())
        .unwrap();

    let atividade = storage.assignments(false).unwrap().pop().unwrap();
    storage
        .set_assignment_decision(atividade.id, Decision::Done)
        .unwrap();

    // O portal continua dizendo pendente, e sincroniza de novo.
    storage
        .apply_provider_snapshot(&retrato, OffsetDateTime::now_utc())
        .unwrap();

    let depois = storage.assignments(false).unwrap().pop().unwrap();
    assert_eq!(
        depois.status,
        AssignmentStatus::Pending,
        "o portal continua com a palavra dele"
    );
    assert_eq!(depois.decision, Decision::Done, "e eu continuo com a minha");
}

/// O item descartado nao pode voltar a cobrar a cada rodada — e o §64 do
/// pedido, e a diferenca entre uma lista util e uma que se ignora.
#[test]
fn nao_vou_fazer_nao_volta_depois_de_tres_sincronizacoes() {
    let (_dir, storage) = storage();
    let retrato = retrato_com_atividade(ExternalAssignmentStatus::Pending);
    storage
        .apply_provider_snapshot(&retrato, OffsetDateTime::now_utc())
        .unwrap();
    let atividade = storage.assignments(false).unwrap().pop().unwrap();
    storage
        .set_assignment_decision(atividade.id, Decision::Skipped)
        .unwrap();

    for _ in 0..3 {
        storage
            .apply_provider_snapshot(&retrato, OffsetDateTime::now_utc())
            .unwrap();
    }

    let depois = storage.assignments(false).unwrap().pop().unwrap();
    assert_eq!(depois.decision, Decision::Skipped);
    assert_eq!(storage.assignments(false).unwrap().len(), 1);
}

/// Mudar o prazo no portal e legitimo. Apagar o plano pessoal junto nao e.
#[test]
fn o_prazo_muda_e_o_plano_pessoal_fica_de_pe() {
    let (_dir, storage) = storage();
    let mut retrato = retrato_com_atividade(ExternalAssignmentStatus::Pending);
    storage
        .apply_provider_snapshot(&retrato, OffsetDateTime::now_utc())
        .unwrap();

    let atividade = storage.assignments(false).unwrap().pop().unwrap();
    storage
        .plan_assignment(
            atividade.id,
            Some(Plano::novo(datetime!(2026-08-22 19:30 UTC), 60).unwrap()),
        )
        .unwrap();

    retrato.assignments[0].due_at = Some(datetime!(2026-08-30 23:59 UTC));
    storage
        .apply_provider_snapshot(&retrato, OffsetDateTime::now_utc())
        .unwrap();

    let depois = storage.assignments(false).unwrap().pop().unwrap();
    assert_eq!(
        depois.due_at,
        Some(datetime!(2026-08-30 23:59 UTC)),
        "o prazo novo e do portal"
    );
    assert_eq!(
        depois.planned_at,
        Some(datetime!(2026-08-22 19:30 UTC)),
        "quando eu vou fazer continua sendo minha decisao"
    );
    assert_eq!(depois.planned_minutes, 60);
}

/// E o sync tambem nao pode marcar como decidido algo que ninguem decidiu.
#[test]
fn o_sync_nao_inventa_decisao() {
    let (_dir, storage) = storage();
    let mut retrato = retrato_com_atividade(ExternalAssignmentStatus::Pending);
    retrato.assignments[0].status = ExternalAssignmentStatus::Graded;
    retrato.assignments[0].score = Some(9.5);
    retrato.assignments[0].max_score = Some(10.0);
    storage
        .apply_provider_snapshot(&retrato, OffsetDateTime::now_utc())
        .unwrap();

    let atividade = storage.assignments(false).unwrap().pop().unwrap();
    assert_eq!(atividade.status, AssignmentStatus::Graded);
    assert_eq!(
        atividade.decision,
        Decision::None,
        "o portal dar nota nao e a pessoa dizer que resolveu"
    );
}
