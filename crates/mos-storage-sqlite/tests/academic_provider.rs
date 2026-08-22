//! A sincronizacao com um AVA externo, contra um banco de verdade.
//!
//! O que so aqui se prova: que rodar duas vezes nao duplica, que a decisao
//! pessoal sobrevive a republicacao do portal, que a media oficial nao invade a
//! media propria e que o que some fica marcado em vez de sumir.

use mos_core::academic_sync::{
    ExternalAcademicContext, ExternalAssessment, ExternalAssessmentStatus, ExternalAssignment,
    ExternalAssignmentStatus, ExternalMaterial, ExternalSemester, ExternalSubject,
    ProviderConnection, ProviderSnapshot, SyncOutcome, PROVIDER_UNIVIRTUS,
};
use mos_core::{AcademicRepository, AssignmentStatus, Day, LifecycleState, Priority};
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

fn semestre() -> ExternalSemester {
    ExternalSemester {
        external_id: "2026B2".into(),
        name: "2026B2".into(),
        institution: "UNINTER".into(),
        starts_on: Day::parse("2026-07-01").unwrap(),
        ends_on: Day::parse("2026-08-31").unwrap(),
        current: true,
    }
}

fn disciplina(nota_oficial: Option<f64>) -> ExternalSubject {
    ExternalSubject {
        external_id: "905706".into(),
        semester_external_id: "2026B2".into(),
        name: "Projeto Arquitetônico".into(),
        code: "905706".into(),
        teacher: String::new(),
        situation: "EM CURSO".into(),
        official_grade: nota_oficial,
    }
}

fn avaliacao(external: &str, quando: OffsetDateTime, nota: Option<f64>) -> ExternalAssessment {
    ExternalAssessment {
        external_id: external.into(),
        subject_external_id: "905706".into(),
        title: "Prova Objetiva (Regular)".into(),
        category: "Prova Objetiva".into(),
        available_at: None,
        due_at: quando,
        weight: 30.0,
        max_score: Some(100.0),
        score: nota,
        status: if nota.is_some() {
            ExternalAssessmentStatus::Graded
        } else {
            ExternalAssessmentStatus::Pending
        },
    }
}

fn trabalho(external: &str, prazo: OffsetDateTime) -> ExternalAssignment {
    ExternalAssignment {
        external_id: external.into(),
        subject_external_id: "905706".into(),
        title: "Atividade Prática Presencial".into(),
        description: "Trabalho".into(),
        due_at: Some(prazo),
        submitted_at: None,
        weight: 0.0,
        max_score: None,
        score: None,
        status: ExternalAssignmentStatus::Pending,
    }
}

fn material() -> ExternalMaterial {
    ExternalMaterial {
        external_id: "60634399".into(),
        subject_external_id: "905706".into(),
        title: "PLANO DE ENSINO.pdf".into(),
        extension: "pdf".into(),
        complementary: true,
        temporary_url: Some("https://cdn.example/x?Signature=aaa&Expires=1".into()),
    }
}

fn retrato() -> ProviderSnapshot {
    ProviderSnapshot {
        provider: PROVIDER_UNIVIRTUS.into(),
        context: Some(ExternalAcademicContext {
            course_name: "BACHARELADO EM ENGENHARIA CIVIL - DISTÂNCIA".into(),
            course_external_id: "5359".into(),
            enrollment_status: "ATIVO".into(),
        }),
        semesters: vec![semestre()],
        subjects: vec![disciplina(None)],
        assessments: vec![
            avaliacao("2713956", datetime!(2026-08-24 23:59 UTC), Some(100.0)),
            avaliacao("2713958", datetime!(2026-09-14 23:59 UTC), None),
        ],
        assignments: vec![trabalho("352876:394147", datetime!(2026-08-24 23:59 UTC))],
        materials: vec![material()],
        warnings: Vec::new(),
    }
}

fn agora() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

// ===========================================================================
// A primeira sincronizacao
// ===========================================================================

#[test]
fn a_primeira_sincronizacao_traz_tudo() {
    let (_dir, storage) = storage();
    let relatorio = storage
        .apply_provider_snapshot(&retrato(), agora())
        .unwrap();

    assert_eq!(relatorio.outcome, SyncOutcome::Completed);
    assert_eq!(relatorio.semesters.created, 1);
    assert_eq!(relatorio.subjects.created, 1);
    assert_eq!(relatorio.assessments.created, 2);
    assert_eq!(relatorio.assignments.created, 1);
    assert_eq!(relatorio.materials.created, 1);

    assert_eq!(storage.semesters(false).unwrap().len(), 1);
    assert_eq!(storage.subjects(false).unwrap().len(), 1);
    assert_eq!(storage.exams(false).unwrap().len(), 2);
    assert_eq!(storage.assignments(false).unwrap().len(), 1);

    let disciplina = storage.subjects(false).unwrap().pop().unwrap();
    assert_eq!(storage.subject_resources(disciplina.id).unwrap().len(), 1);
}

/// A exigencia central do §25: rodar N vezes sem mudanca no portal termina com
/// o mesmo banco.
#[test]
fn rodar_quatro_vezes_nao_cria_uma_linha_a_mais() {
    let (_dir, storage) = storage();
    let retrato = retrato();
    for _ in 0..4 {
        storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    }
    assert_eq!(storage.semesters(false).unwrap().len(), 1);
    assert_eq!(storage.subjects(false).unwrap().len(), 1);
    assert_eq!(storage.exams(false).unwrap().len(), 2);
    assert_eq!(storage.assignments(false).unwrap().len(), 1);

    let ultimo = storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    assert_eq!(ultimo.subjects.created, 0);
    assert_eq!(ultimo.assessments.created, 0);
    assert_eq!(ultimo.assessments.unchanged, 2);
    assert_eq!(ultimo.materials.created, 0);
    assert_eq!(ultimo.resumo(), "", "nada mudou, nada a anunciar");
}

/// As cinco provas nao iniciadas chegam do portal com `id: 0`. Elas sao cinco
/// linhas, e continuam cinco depois de sincronizar de novo.
#[test]
fn cinco_avaliacoes_com_id_zero_viram_cinco_provas_e_nao_uma() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    retrato.assessments = ["2713958", "2713960", "2713961", "2713962", "2713963"]
        .iter()
        .map(|id| avaliacao(id, datetime!(2026-09-14 23:59 UTC), None))
        .collect();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    assert_eq!(storage.exams(false).unwrap().len(), 5);
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    assert_eq!(storage.exams(false).unwrap().len(), 5);
}

// ===========================================================================
// A atualizacao
// ===========================================================================

#[test]
fn prazo_que_muda_atualiza_a_prova_existente() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    let antes = storage.exams(false).unwrap();

    retrato.assessments[1] = avaliacao("2713958", datetime!(2026-09-30 23:59 UTC), None);
    let relatorio = storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    assert_eq!(relatorio.assessments.created, 0);
    assert_eq!(relatorio.assessments.updated, 1);
    let depois = storage.exams(false).unwrap();
    assert_eq!(depois.len(), antes.len());
    assert!(depois.iter().any(|e| e.at == datetime!(2026-09-30 23:59 UTC)));
}

/// A regra do §14, verificada ate o banco: `peso` virou teto, `pesoMedia` virou
/// peso. Se alguem trocasse os dois, a prova entraria pesando 100.
#[test]
fn o_teto_e_o_peso_chegam_nas_colunas_certas() {
    let (_dir, storage) = storage();
    storage
        .apply_provider_snapshot(&retrato(), agora())
        .unwrap();
    let prova = storage
        .exams(false)
        .unwrap()
        .into_iter()
        .find(|e| e.score == Some(100.0))
        .unwrap();
    assert_eq!(prova.max_score, Some(100.0), "peso -> max_score");
    assert_eq!(prova.weight, 30.0, "pesoMedia -> weight");
}

// ===========================================================================
// A fronteira com a decisao pessoal (§32)
// ===========================================================================

/// O portal republica a mesma atividade. A prioridade que a pessoa escolheu e a
/// Task que ela criou continuam de pe.
#[test]
fn a_republicacao_do_portal_nao_desfaz_a_decisao_da_pessoa() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let atividade = storage.assignments(false).unwrap().pop().unwrap();
    // A pessoa marca como urgente e cria a Task.
    storage
        .update_assignment(mos_core::UpdateAssignment {
            id: atividade.id,
            title: atividade.title.clone(),
            description: atividade.description.clone(),
            due_at: atividade.due_at,
            priority: Priority::Urgent,
            status: atividade.status,
            weight: atividade.weight,
            score: atividade.score,
            max_score: atividade.max_score,
        })
        .unwrap();
    let task = storage.create_task_for_assignment(atividade.id).unwrap();

    // O portal muda o prazo e sincroniza de novo.
    retrato.assignments[0] = trabalho("352876:394147", datetime!(2026-08-30 23:59 UTC));
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let depois = storage.assignments(false).unwrap().pop().unwrap();
    assert_eq!(depois.due_at, Some(datetime!(2026-08-30 23:59 UTC)), "o prazo e do portal");
    assert_eq!(depois.priority, Priority::Urgent, "a prioridade e da pessoa");
    assert_eq!(depois.task_id, Some(task.id), "a Task continua ligada");
}

/// Accent e observacoes sao a organizacao de quem estuda. O sync nunca os toca.
#[test]
fn accent_e_observacoes_sobrevivem_a_sincronizacao() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let materia = storage.subjects(false).unwrap().pop().unwrap();
    storage
        .update_subject(materia.id, &materia.name, &materia.code, "Prof. Ana", "musgo", "sentar na frente")
        .unwrap();

    retrato.subjects[0].name = "Projeto Arquitetônico II".into();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let depois = storage.subjects(false).unwrap().pop().unwrap();
    assert_eq!(depois.name, "Projeto Arquitetônico II", "o nome e do portal");
    assert_eq!(depois.accent, "musgo", "a cor e da pessoa");
    assert_eq!(depois.notes, "sentar na frente", "a observacao e da pessoa");
    assert_eq!(
        depois.teacher, "Prof. Ana",
        "o Univirtus manda professor vazio, e vazio nao apaga"
    );
}

/// A prova nao tem local no portal. Quem escreve onde ela acontece e a pessoa.
#[test]
fn o_local_da_prova_e_da_pessoa_e_nao_do_portal() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let prova = storage.exams(false).unwrap().pop().unwrap();
    storage
        .update_exam(mos_core::UpdateExam {
            id: prova.id,
            name: prova.name.clone(),
            at: prova.at,
            location: "Polo Centro, sala 4".into(),
            topics: prova.topics.clone(),
            status: prova.status,
            weight: prova.weight,
            score: prova.score,
            max_score: prova.max_score,
        })
        .unwrap();

    retrato.assessments[0] = avaliacao("2713956", datetime!(2026-08-25 23:59 UTC), Some(100.0));
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let depois = storage
        .exams(false)
        .unwrap()
        .into_iter()
        .find(|e| e.id == prova.id)
        .unwrap();
    assert_eq!(depois.location, "Polo Centro, sala 4");
}

// ===========================================================================
// A media oficial (§16)
// ===========================================================================

/// `aproveitamentoMD` entra como fato do provedor, e NAO como nota de avaliacao
/// nenhuma. A media do M/OS continua sendo derivada.
#[test]
fn a_media_oficial_nao_invade_a_media_calculada() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    retrato.subjects[0] = disciplina(Some(7.4));
    // Uma unica avaliacao, com 100 de 100 e peso 30.
    retrato.assessments = vec![avaliacao("2713956", datetime!(2026-08-24 23:59 UTC), Some(100.0))];
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let materia = storage.subjects(false).unwrap().pop().unwrap();
    let fatos = storage.provider_subject_facts(PROVIDER_UNIVIRTUS).unwrap();
    let fato = fatos.iter().find(|f| f.subject_id == materia.id).unwrap();
    assert_eq!(fato.official_grade, Some(7.4));
    assert_eq!(fato.situation, "EM CURSO");

    // A media propria e 10,0 — a nota cheia, na escala de 0 a 10 em que o
    // `desempenho` do M/OS trabalha. A oficial e 7,4, porque a UNINTER conta
    // exame e recuperacao que o M/OS nao modela. As duas discordam, e e para
    // discordar: nenhuma sobrescreve a outra.
    let provas: Vec<_> = storage.exams(false).unwrap();
    let calculada = mos_core::desempenho(&[], &provas);
    assert_eq!(calculada.media, Some(10.0));
    assert_ne!(calculada.media, fato.official_grade);
}

// ===========================================================================
// Ausencia (§33)
// ===========================================================================

#[test]
fn o_que_some_do_portal_e_marcado_e_a_prova_continua_no_banco() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    assert_eq!(storage.exams(false).unwrap().len(), 2);

    retrato.assessments.remove(1);
    let relatorio = storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    assert_eq!(relatorio.assessments.unavailable, 1);
    assert_eq!(
        storage.exams(false).unwrap().len(),
        2,
        "sumir do portal nao apaga do M/OS"
    );
}

#[test]
fn o_que_reaparece_deixa_de_estar_ausente() {
    let (_dir, storage) = storage();
    let completo = retrato();
    let mut parcial = retrato();
    parcial.assessments.remove(1);

    storage.apply_provider_snapshot(&completo, agora()).unwrap();
    storage.apply_provider_snapshot(&parcial, agora()).unwrap();
    let volta = storage.apply_provider_snapshot(&completo, agora()).unwrap();

    assert_eq!(volta.assessments.updated, 1, "reaparecer conta como update");
    assert_eq!(volta.assessments.unavailable, 0);
    assert_eq!(storage.exams(false).unwrap().len(), 2);
}

// ===========================================================================
// Materiais (§21)
// ===========================================================================

/// A URL assinada nunca vira identidade nem endereco do Resource. Ela e cache.
#[test]
fn a_url_assinada_nao_entra_no_resource_e_muda_sem_duplicar() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let materia = storage.subjects(false).unwrap().pop().unwrap();
    let recursos = storage.subject_resources(materia.id).unwrap();
    assert_eq!(recursos.len(), 1);
    assert_eq!(recursos[0].url, "", "a URL assinada nao vira endereco do Resource");
    assert_eq!(recursos[0].title, "PLANO DE ENSINO.pdf");

    // A assinatura muda, como muda a cada resposta do portal.
    retrato.materials[0].temporary_url =
        Some("https://cdn.example/x?Signature=bbb&Expires=2".into());
    let relatorio = storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    assert_eq!(relatorio.materials.created, 0, "URL nova nao cria material novo");
    assert_eq!(relatorio.materials.unchanged, 1);
    assert_eq!(storage.subject_resources(materia.id).unwrap().len(), 1);
    let url = storage
        .material_url(PROVIDER_UNIVIRTUS, "60634399")
        .unwrap()
        .unwrap();
    assert!(url.contains("Signature=bbb"), "o cache guarda a mais recente");
}

// ===========================================================================
// Estado da conexao (§30)
// ===========================================================================

#[test]
fn desconectado_e_o_estado_de_quem_nunca_conectou() {
    let (_dir, storage) = storage();
    let estado = storage.provider_status(PROVIDER_UNIVIRTUS).unwrap();
    assert_eq!(estado.connection, ProviderConnection::Disconnected);
    assert!(estado.last_sync_at.is_none());
    assert!(estado.tracked.is_empty());
}

#[test]
fn depois_de_sincronizar_a_conexao_esta_saudavel_e_contada() {
    let (_dir, storage) = storage();
    storage
        .apply_provider_snapshot(&retrato(), agora())
        .unwrap();
    let estado = storage.provider_status(PROVIDER_UNIVIRTUS).unwrap();
    assert_eq!(estado.connection, ProviderConnection::Connected);
    assert!(estado.last_sync_at.is_some());
    assert_eq!(estado.last_outcome, Some(SyncOutcome::Completed));
    assert!(estado.course_name.contains("ENGENHARIA CIVIL"));
    assert_eq!(estado.tracked.get("subject"), Some(&1));
    assert_eq!(estado.tracked.get("exam"), Some(&2));
}

/// Sessao expirada nao apaga dado. O M/Academic continua servindo o que ja tem.
#[test]
fn a_sessao_expirada_preserva_os_dados_sincronizados() {
    let (_dir, storage) = storage();
    storage
        .apply_provider_snapshot(&retrato(), agora())
        .unwrap();
    storage
        .set_provider_connection(PROVIDER_UNIVIRTUS, ProviderConnection::Expired)
        .unwrap();

    let estado = storage.provider_status(PROVIDER_UNIVIRTUS).unwrap();
    assert_eq!(estado.connection, ProviderConnection::Expired);
    assert_eq!(storage.subjects(false).unwrap().len(), 1);
    assert_eq!(storage.exams(false).unwrap().len(), 2);
    assert_eq!(storage.assignments(false).unwrap().len(), 1);
}

/// Reconectar e sincronizar de novo nao duplica o que ja estava la.
#[test]
fn reconectar_e_sincronizar_nao_duplica() {
    let (_dir, storage) = storage();
    let retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    storage
        .set_provider_connection(PROVIDER_UNIVIRTUS, ProviderConnection::Expired)
        .unwrap();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    assert_eq!(storage.exams(false).unwrap().len(), 2);
    let estado = storage.provider_status(PROVIDER_UNIVIRTUS).unwrap();
    assert_eq!(estado.connection, ProviderConnection::Connected);
}

/// Desconectar para de trazer — e nao apaga o semestre de ninguem.
#[test]
fn desconectar_preserva_as_entidades_academicas() {
    let (_dir, storage) = storage();
    storage
        .apply_provider_snapshot(&retrato(), agora())
        .unwrap();
    storage.forget_provider(PROVIDER_UNIVIRTUS).unwrap();

    assert_eq!(storage.subjects(false).unwrap().len(), 1);
    assert_eq!(storage.exams(false).unwrap().len(), 2);
    let estado = storage.provider_status(PROVIDER_UNIVIRTUS).unwrap();
    assert_eq!(estado.connection, ProviderConnection::Disconnected);
    assert!(estado.tracked.is_empty());
}

/// Depois de desconectar, reconectar reaproveita as entidades? Nao — sem as
/// referencias, o provedor nao sabe mais quais eram dele, e cria de novo. Este
/// teste registra a consequencia em vez de fingir que ela nao existe.
#[test]
fn reconectar_depois_de_desconectar_recria_e_isso_esta_documentado() {
    let (_dir, storage) = storage();
    let retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    storage.forget_provider(PROVIDER_UNIVIRTUS).unwrap();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();
    assert_eq!(
        storage.exams(false).unwrap().len(),
        4,
        "desconectar corta o fio; reconectar traz um conjunto novo ao lado do antigo"
    );
}

// ===========================================================================
// Falha isolada (§47)
// ===========================================================================

#[test]
fn disciplina_que_falhou_vira_aviso_e_nao_derruba_o_resto() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    retrato
        .warnings
        .push("Estática dos corpos: o portal nao respondeu.".into());
    let relatorio = storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    assert_eq!(relatorio.outcome, SyncOutcome::CompletedWithWarnings);
    assert_eq!(relatorio.warnings.len(), 1);
    assert_eq!(relatorio.subjects.created, 1, "o que deu certo entrou");
    assert_eq!(relatorio.assessments.created, 2);
}

// ===========================================================================
// Ciclo de vida
// ===========================================================================

/// Arquivar e decisao da pessoa. Sincronizar de novo nao desarquiva.
#[test]
fn o_sync_nao_desarquiva_o_que_a_pessoa_arquivou() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let materia = storage.subjects(false).unwrap().pop().unwrap();
    storage
        .set_subject_lifecycle(materia.id, LifecycleState::Archived)
        .unwrap();

    retrato.subjects[0].name = "Outro nome".into();
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    assert!(storage.subjects(false).unwrap().is_empty());
    let arquivada = storage
        .subjects(true)
        .unwrap()
        .into_iter()
        .find(|s| s.id == materia.id)
        .unwrap();
    assert_eq!(arquivada.lifecycle_state, LifecycleState::Archived);
    assert_eq!(arquivada.name, "Outro nome", "o dado do portal ainda atualiza");
}

#[test]
fn a_atividade_entregue_no_portal_chega_como_entregue() {
    let (_dir, storage) = storage();
    let mut retrato = retrato();
    retrato.assignments[0].submitted_at = Some(datetime!(2026-08-20 10:00 UTC));
    retrato.assignments[0].status = ExternalAssignmentStatus::Submitted;
    storage.apply_provider_snapshot(&retrato, agora()).unwrap();

    let atividade = storage.assignments(false).unwrap().pop().unwrap();
    assert_eq!(atividade.status, AssignmentStatus::Submitted);
}
