//! O fecho da semana contra um banco de verdade.
//!
//! O que se prova aqui e o que so o banco pode desmentir: a unicidade por
//! semana, o upsert preservando o instante do fecho, a janela de sessoes nas
//! duas bordas, e a emissao de sync na mesma transacao.

use mos_core::{
    DailyRepository, Day, NewDailyObjective, NewDailySession, NewWeeklyReview, ObjectivePriority,
    Week,
};
use mos_storage_sqlite::SqliteStorage;
use time::macros::datetime;
use time::{Duration, OffsetDateTime};

fn banco() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap();
    (dir, storage)
}

fn com_sync() -> (tempfile::TempDir, SqliteStorage) {
    use mos_sync::DeviceRepository;
    let (dir, storage) = banco();
    let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    (dir, storage)
}

fn agora() -> OffsetDateTime {
    datetime!(2026-08-23 18:00 -03:00)
}

fn dia(valor: &str) -> Day {
    Day::parse(valor).unwrap()
}

fn semana(valor: &str) -> Week {
    Week::containing(&dia(valor)).unwrap()
}

/// Cria uma sessao com um objetivo, num dia.
fn sessao_em(storage: &SqliteStorage, day: &str, titulo: &str) {
    let quando = agora();
    let nova = NewDailySession::create(dia(day), "", quando).unwrap();
    let id = nova.id;
    let objetivo =
        NewDailyObjective::create(id, titulo, "", None, ObjectivePriority::Main, 0, quando)
            .unwrap();
    storage.start_day(nova, vec![objetivo], quando).unwrap();
}

#[test]
fn a_janela_da_semana_pega_as_duas_bordas() {
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-16", "domingo anterior");
    sessao_em(&storage, "2026-08-17", "segunda");
    sessao_em(&storage, "2026-08-23", "domingo");
    sessao_em(&storage, "2026-08-24", "segunda seguinte");

    let dentro = storage.sessions_between(&semana("2026-08-19")).unwrap();
    let dias: Vec<&str> = dentro.iter().map(|s| s.day.as_str()).collect();
    assert_eq!(
        dias,
        ["2026-08-17", "2026-08-23"],
        "as duas bordas entram, e so elas"
    );
}

#[test]
fn fechar_a_semana_grava_e_le_de_volta() {
    let (_dir, storage) = banco();
    let alvo = semana("2026-08-19");
    assert!(storage.weekly_review(&alvo).unwrap().is_none());

    let fechada = storage
        .save_weekly_review(
            NewWeeklyReview::create(alvo.clone(), "  o 063-26 tomou a semana  ", agora()),
            agora(),
        )
        .unwrap();
    assert_eq!(fechada.week, alvo);
    assert_eq!(fechada.summary, "o 063-26 tomou a semana");

    let lida = storage.weekly_review(&alvo).unwrap().unwrap();
    assert_eq!(lida.id, fechada.id);
}

#[test]
fn texto_vazio_ainda_fecha_a_semana() {
    // Fechar e o gesto; escrever e opcional. Difere da reflexao do dia, que e
    // acessorio do encerramento — aqui o texto e o unico campo, e a linha
    // precisa existir para a semana constar como fechada.
    let (_dir, storage) = banco();
    let alvo = semana("2026-08-19");
    storage
        .save_weekly_review(
            NewWeeklyReview::create(alvo.clone(), "   ", agora()),
            agora(),
        )
        .unwrap();
    let lida = storage.weekly_review(&alvo).unwrap().unwrap();
    assert_eq!(lida.summary, "");
}

#[test]
fn regravar_preserva_o_instante_do_fecho() {
    // Editar o texto na quarta nao pode dizer que a semana foi fechada na
    // quarta: quando ela foi fechada e um fato, e o texto e outro.
    let (_dir, storage) = banco();
    let alvo = semana("2026-08-19");
    let primeira = storage
        .save_weekly_review(
            NewWeeklyReview::create(alvo.clone(), "primeira", agora()),
            agora(),
        )
        .unwrap();

    let depois = agora() + Duration::days(3);
    let segunda = storage
        .save_weekly_review(
            NewWeeklyReview::create(alvo.clone(), "corrigida", depois),
            depois,
        )
        .unwrap();

    assert_eq!(segunda.summary, "corrigida");
    assert_eq!(segunda.closed_at, primeira.closed_at, "o fecho nao se move");
    assert_eq!(
        segunda.id, primeira.id,
        "e o registro continua sendo o mesmo"
    );
    assert!(segunda.updated_at > primeira.updated_at);
    assert_eq!(
        storage.weekly_reviews(10).unwrap().len(),
        1,
        "uma linha por semana"
    );
}

#[test]
fn as_semanas_vem_da_mais_recente_para_a_mais_antiga() {
    let (_dir, storage) = banco();
    for valor in ["2026-08-05", "2026-08-19", "2026-08-12"] {
        storage
            .save_weekly_review(NewWeeklyReview::create(semana(valor), "", agora()), agora())
            .unwrap();
    }
    let semanas: Vec<String> = storage
        .weekly_reviews(10)
        .unwrap()
        .iter()
        .map(|review| review.week.to_string())
        .collect();
    assert_eq!(semanas, ["2026-08-17", "2026-08-10", "2026-08-03"]);
}

#[test]
fn fechar_a_semana_emite_a_operacao() {
    use mos_sync::{OpBody, OutboxRepository};
    let (_dir, storage) = com_sync();
    let alvo = semana("2026-08-19");
    let fechada = storage
        .save_weekly_review(NewWeeklyReview::create(alvo, "foi boa", agora()), agora())
        .unwrap();

    let ops = storage.pendentes(10).unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].entity.kind.as_str(), "weekly_review");
    assert_eq!(ops[0].entity.id, fechada.id.as_uuid());
    let campos = match &ops[0].body {
        OpBody::Create { fields } | OpBody::Update { fields } => fields.clone(),
        outro => panic!("esperava campos, veio {outro:?}"),
    };
    assert_eq!(
        campos["weekStart"],
        serde_json::json!("2026-08-17"),
        "a semana precisa viajar sabendo de que semana ela e: o id e um UUID e nao diz nada"
    );
}

#[test]
fn corrigir_o_texto_emite_com_o_id_que_ficou_gravado() {
    // O caso que so o banco desmente: numa correcao o INSERT perde para o ON
    // CONFLICT, e o id sorteado pelo `NewWeeklyReview::create` nunca chega ao
    // banco. Emitir com ele criaria uma segunda entidade do outro lado — uma
    // que nao existe aqui.
    use mos_sync::OutboxRepository;
    let (_dir, storage) = com_sync();
    let alvo = semana("2026-08-19");
    let primeira = storage
        .save_weekly_review(
            NewWeeklyReview::create(alvo.clone(), "primeira", agora()),
            agora(),
        )
        .unwrap();
    storage
        .save_weekly_review(NewWeeklyReview::create(alvo, "corrigida", agora()), agora())
        .unwrap();

    let ops = storage.pendentes(10).unwrap();
    assert_eq!(ops.len(), 2);
    assert_eq!(
        ops[1].entity.id,
        primeira.id.as_uuid(),
        "as duas operacoes falam da MESMA entidade"
    );
}

#[test]
fn sem_sync_ligado_nada_e_emitido_e_nada_falha() {
    use mos_sync::OutboxRepository;
    let (_dir, storage) = banco();
    storage
        .save_weekly_review(
            NewWeeklyReview::create(semana("2026-08-19"), "x", agora()),
            agora(),
        )
        .unwrap();
    assert!(storage.pendentes(10).unwrap().is_empty());
}

#[test]
fn as_reflexoes_de_varias_sessoes_vem_numa_consulta() {
    use mos_core::{DayMood, EndDayInput};
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-17", "segunda");
    sessao_em(&storage, "2026-08-18", "terca");

    let sessoes = storage.sessions_between(&semana("2026-08-19")).unwrap();
    for (sessao, humor) in sessoes.iter().zip([DayMood::Blocked, DayMood::Productive]) {
        let entrada = EndDayInput {
            resolutions: Vec::new(),
            mood: humor.as_str().to_owned(),
            summary: String::new(),
        };
        let reflexao = entrada
            .reflection()
            .unwrap()
            .unwrap()
            .for_session(sessao.id);
        storage
            .end_day(sessao.id, &[], Some(reflexao), agora())
            .unwrap();
    }

    let ids: Vec<_> = sessoes.iter().map(|sessao| sessao.id).collect();
    let reflexoes = storage.reflections_of(&ids).unwrap();
    assert_eq!(reflexoes.len(), 2);
    assert!(reflexoes.iter().any(|r| r.mood == Some(DayMood::Blocked)));

    assert!(
        storage.reflections_of(&[]).unwrap().is_empty(),
        "lista vazia nao vira consulta"
    );
}

// ---------------------------------------------------------------- o servico

fn servico(storage: SqliteStorage) -> mos_core::DailyService {
    let storage = std::sync::Arc::new(storage);
    let clock: std::sync::Arc<dyn mos_core::Clock> = std::sync::Arc::new(mos_core::SystemClock);
    mos_core::DailyService::new(storage, clock)
}

#[test]
fn a_semana_pendente_e_a_mais_recente_sem_fecho() {
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-05", "semana de 03");
    sessao_em(&storage, "2026-08-12", "semana de 10");
    sessao_em(&storage, "2026-08-19", "semana de 17");
    let service = servico(storage);

    // A semana corrente e a de 24; as tres anteriores tiveram sessao.
    let corrente = semana("2026-08-26");
    assert_eq!(
        service.pending_week(&corrente).unwrap().unwrap(),
        semana("2026-08-19"),
        "a mais recente entre as candidatas"
    );

    // Fechada a de 17, a pendencia recua para a de 10.
    service.close_week(&semana("2026-08-19"), "").unwrap();
    assert_eq!(
        service.pending_week(&corrente).unwrap().unwrap(),
        semana("2026-08-12")
    );
}

#[test]
fn a_semana_corrente_nunca_e_pendente() {
    // Ela ainda esta acontecendo. Oferecer o fecho de uma semana em curso seria
    // pedir para revisar o que ainda nao terminou.
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-19", "hoje");
    let service = servico(storage);
    assert!(service
        .pending_week(&semana("2026-08-19"))
        .unwrap()
        .is_none());
}

#[test]
fn semana_sem_sessao_nenhuma_nao_e_pendente() {
    // Nao ha o que revisar, e a linha da Home nunca deve apontar para uma
    // semana vazia.
    let (_dir, storage) = banco();
    let service = servico(storage);
    assert!(service
        .pending_week(&semana("2026-08-26"))
        .unwrap()
        .is_none());
}

#[test]
fn o_resumo_da_semana_traz_o_fecho_quando_ele_existe() {
    let (_dir, storage) = banco();
    sessao_em(&storage, "2026-08-18", "planta");
    let service = servico(storage);
    let alvo = semana("2026-08-18");
    let sem_project = |_: &mos_core::ObjectiveLink| None;

    let antes = service.week(&alvo, &sem_project).unwrap();
    assert!(antes.review.is_none());
    assert_eq!(antes.days_with_session, 1);
    assert!(!antes.empty);

    service.close_week(&alvo, "foi uma semana").unwrap();
    let depois = service.week(&alvo, &sem_project).unwrap();
    assert_eq!(depois.review.unwrap().summary, "foi uma semana");
}
