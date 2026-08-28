//! A Daily Session contra um banco de verdade.
//!
//! O que se prova aqui e o que so o banco pode desmentir: a unicidade do dia, a
//! atomicidade do inicio e do fim, o principal unico, a corrente de carry-over,
//! a conclusao automatica saindo do Kanban e o estado sobrevivendo ao
//! fechamento do aplicativo.
//!
//! As regras PURAS — o que conta como progresso, o que vira carry-over, quando
//! uma Task fecha um objetivo — sao testadas em `mos-core::daily`. Aqui se
//! testa o que acontece quando elas encontram o SQLite.

use mos_core::{
    DailyObjectiveId, DailyRepository, DailySessionId, Day, EndDayInput, LinkKind,
    NewDailyObjective, NewDailyReflection, NewDailySession, NewProject, NewTask, ObjectiveLink,
    ObjectivePriority, ObjectiveResolution, ObjectiveStatus, SearchRequest, SessionStatus,
    TaskState, WorkRepository,
};
use mos_storage_sqlite::SqliteStorage;
use time::macros::datetime;
use time::{Duration, OffsetDateTime};

fn banco() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let storage = abrir(&dir);
    (dir, storage)
}

fn abrir(dir: &tempfile::TempDir) -> SqliteStorage {
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap()
}

fn com_sync() -> (tempfile::TempDir, SqliteStorage) {
    use mos_sync::DeviceRepository;
    let (dir, storage) = banco();
    let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    (dir, storage)
}

fn agora() -> OffsetDateTime {
    datetime!(2026-08-21 09:08 -03:00)
}

fn dia(valor: &str) -> Day {
    Day::parse(valor).unwrap()
}

/// Comeca um dia com um principal e os secundarios dados.
fn comecar(
    storage: &SqliteStorage,
    day: &str,
    principal: &str,
    secundarios: &[&str],
    quando: OffsetDateTime,
) -> DailySessionId {
    let sessao = NewDailySession::create(dia(day), "", quando).unwrap();
    let id = sessao.id;
    let mut objetivos = vec![NewDailyObjective::create(
        id,
        principal,
        "",
        None,
        ObjectivePriority::Main,
        0,
        quando,
    )
    .unwrap()];
    for (posicao, titulo) in secundarios.iter().enumerate() {
        objetivos.push(
            NewDailyObjective::create(
                id,
                titulo,
                "",
                None,
                ObjectivePriority::Secondary,
                posicao as i64 + 1,
                quando,
            )
            .unwrap(),
        );
    }
    storage.start_day(sessao, objetivos, quando).unwrap();
    id
}

// ------------------------------------------------------------------ criacao

#[test]
fn comecar_o_dia_grava_a_sessao_e_os_objetivos_juntos() {
    let (_dir, storage) = banco();
    let id = comecar(
        &storage,
        "2026-08-21",
        "Finalizar planta de formas",
        &["Revisar memorial"],
        agora(),
    );

    let sessao = DailyRepository::session(&storage, id).unwrap();
    assert_eq!(sessao.status, SessionStatus::Active);
    assert_eq!(sessao.day.as_str(), "2026-08-21");
    assert!(sessao.ended_at.is_none(), "dia aberto nao tem hora de fim");

    let objetivos = storage.objectives(id).unwrap();
    assert_eq!(objetivos.len(), 2);
    assert_eq!(objetivos[0].title, "Finalizar planta de formas");
    assert_eq!(
        objetivos[0].priority,
        ObjectivePriority::Main,
        "o principal vem primeiro"
    );
    assert!(objetivos
        .iter()
        .all(|objetivo| objetivo.status == ObjectiveStatus::Pending));
}

#[test]
fn dois_inicios_no_mesmo_dia_sao_recusados() {
    // Duas sessoes na mesma data partiriam o dia em dois placares, e a pergunta
    // "quantos objetivos eu concluí hoje?" passaria a ter duas respostas.
    let (_dir, storage) = banco();
    comecar(&storage, "2026-08-21", "primeiro", &[], agora());

    let segunda = NewDailySession::create(dia("2026-08-21"), "", agora()).unwrap();
    let erro = storage.start_day(segunda, Vec::new(), agora()).unwrap_err();
    assert!(
        erro.message.contains("ja foi iniciado"),
        "veio: {}",
        erro.message
    );

    assert_eq!(storage.sessions(10).unwrap().len(), 1);
}

#[test]
fn o_dia_sobrevive_ao_fechamento_do_aplicativo() {
    // O que o M/OS promete: fechar o app no meio do dia nao apaga o dia.
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let storage = abrir(&dir);
        comecar(&storage, "2026-08-21", "planta", &["memorial"], agora())
    };

    let storage = abrir(&dir);
    let hoje = storage.session_on(&dia("2026-08-21")).unwrap().unwrap();
    assert_eq!(hoje.id, id);
    assert_eq!(hoje.status, SessionStatus::Active);
    assert_eq!(storage.objectives(id).unwrap().len(), 2);
}

// ------------------------------------------------------------- o principal

#[test]
fn so_existe_um_principal_por_dia() {
    let (_dir, storage) = banco();
    let id = comecar(
        &storage,
        "2026-08-21",
        "planta",
        &["memorial", "arquivos"],
        agora(),
    );
    let objetivos = storage.objectives(id).unwrap();
    let memorial = objetivos
        .iter()
        .find(|objetivo| objetivo.title == "memorial")
        .unwrap();

    let depois = storage.set_main_objective(memorial.id, agora()).unwrap();
    let principais: Vec<_> = depois
        .iter()
        .filter(|objetivo| objetivo.priority == ObjectivePriority::Main)
        .collect();
    assert_eq!(
        principais.len(),
        1,
        "promover tem de rebaixar o anterior na mesma transacao"
    );
    assert_eq!(principais[0].title, "memorial");
    assert_eq!(
        depois[0].title, "memorial",
        "e o principal continua sendo o primeiro da lista"
    );
}

#[test]
fn acrescentar_um_principal_rebaixa_o_anterior() {
    let (_dir, storage) = banco();
    let id = comecar(&storage, "2026-08-21", "planta", &[], agora());

    storage
        .add_objective(
            NewDailyObjective::create(
                id,
                "mudou de ideia",
                "",
                None,
                ObjectivePriority::Main,
                1,
                agora(),
            )
            .unwrap(),
        )
        .unwrap();

    let objetivos = storage.objectives(id).unwrap();
    assert_eq!(
        objetivos
            .iter()
            .filter(|o| o.priority == ObjectivePriority::Main)
            .count(),
        1
    );
    assert_eq!(objetivos[0].title, "mudou de ideia");
}

// --------------------------------------------------------- vinculo com Task

#[test]
fn concluir_a_task_conclui_o_objetivo_que_e_ela() {
    let (_dir, storage) = banco();
    let project = storage
        .create_project(NewProject::create("063-26", "", "").unwrap())
        .unwrap();
    let task = storage
        .create_task(NewTask::create("Finalizar planta de formas", "", Some(project.id)).unwrap())
        .unwrap();

    let sessao = NewDailySession::create(dia("2026-08-21"), "", agora()).unwrap();
    let id = sessao.id;
    let da_task = NewDailyObjective::create(
        id,
        "Finalizar planta de formas",
        "",
        Some(ObjectiveLink::new(LinkKind::Task, &task.id.to_string()).unwrap()),
        ObjectivePriority::Main,
        0,
        agora(),
    )
    .unwrap();
    let do_project = NewDailyObjective::create(
        id,
        "Avancar o 063-26",
        "",
        Some(ObjectiveLink::new(LinkKind::Project, &project.id.to_string()).unwrap()),
        ObjectivePriority::Secondary,
        1,
        agora(),
    )
    .unwrap();
    storage
        .start_day(sessao, vec![da_task, do_project], agora())
        .unwrap();

    storage.set_task_state(task.id, TaskState::Done).unwrap();

    let objetivos = storage.objectives(id).unwrap();
    assert_eq!(
        objetivos[0].status,
        ObjectiveStatus::Completed,
        "o objetivo que E a Task fecha junto"
    );
    assert!(
        objetivos[0].completed_at.is_some(),
        "concluido carimba a hora"
    );
    assert_eq!(
        objetivos[1].status,
        ObjectiveStatus::Pending,
        "um objetivo de Project e maior que uma Task dele, e nao fecha sozinho"
    );
}

#[test]
fn tirar_a_task_do_done_devolve_o_objetivo_a_pendente() {
    // Estados divergentes sao o que o §11 do pedido manda evitar: uma Task de
    // volta em `review` com o objetivo do dia marcado como concluido diria duas
    // coisas contrarias sobre o mesmo trabalho.
    let (_dir, storage) = banco();
    let task = storage
        .create_task(NewTask::create("enviar arquivos", "", None).unwrap())
        .unwrap();
    let sessao = NewDailySession::create(dia("2026-08-21"), "", agora()).unwrap();
    let id = sessao.id;
    storage
        .start_day(
            sessao,
            vec![NewDailyObjective::create(
                id,
                "enviar arquivos",
                "",
                Some(ObjectiveLink::new(LinkKind::Task, &task.id.to_string()).unwrap()),
                ObjectivePriority::Main,
                0,
                agora(),
            )
            .unwrap()],
            agora(),
        )
        .unwrap();

    storage.set_task_state(task.id, TaskState::Done).unwrap();
    storage.set_task_state(task.id, TaskState::Review).unwrap();

    let objetivo = &storage.objectives(id).unwrap()[0];
    assert_eq!(objetivo.status, ObjectiveStatus::Pending);
    assert!(
        objetivo.completed_at.is_none(),
        "sair de concluido limpa o carimbo"
    );
}

#[test]
fn concluir_uma_task_nao_reescreve_o_placar_de_um_dia_encerrado() {
    let (_dir, storage) = banco();
    let task = storage
        .create_task(NewTask::create("enviar", "", None).unwrap())
        .unwrap();
    let sessao = NewDailySession::create(dia("2026-08-20"), "", agora()).unwrap();
    let ontem = sessao.id;
    storage
        .start_day(
            sessao,
            vec![NewDailyObjective::create(
                ontem,
                "enviar",
                "",
                Some(ObjectiveLink::new(LinkKind::Task, &task.id.to_string()).unwrap()),
                ObjectivePriority::Main,
                0,
                agora(),
            )
            .unwrap()],
            agora(),
        )
        .unwrap();
    storage.end_day(ontem, &[], None, agora()).unwrap();

    storage.set_task_state(task.id, TaskState::Done).unwrap();

    assert_eq!(
        storage.objectives(ontem).unwrap()[0].status,
        ObjectiveStatus::Pending,
        "historico e registro, e registro nao muda sozinho depois"
    );
}

#[test]
fn objetivo_vinculado_a_entidade_apagada_continua_legivel() {
    // O §23 pede este caso por nome. Perder a linha inteira por causa de um
    // ponteiro seria apagar o registro do que importou naquele dia.
    let (_dir, storage) = banco();
    let task = storage
        .create_task(NewTask::create("some daqui", "", None).unwrap())
        .unwrap();
    let sessao = NewDailySession::create(dia("2026-08-21"), "", agora()).unwrap();
    let id = sessao.id;
    storage
        .start_day(
            sessao,
            vec![NewDailyObjective::create(
                id,
                "some daqui",
                "",
                Some(ObjectiveLink::new(LinkKind::Task, &task.id.to_string()).unwrap()),
                ObjectivePriority::Main,
                0,
                agora(),
            )
            .unwrap()],
            agora(),
        )
        .unwrap();

    storage
        .set_task_lifecycle(task.id, mos_core::LifecycleState::Archived)
        .unwrap();
    storage
        .set_task_lifecycle(task.id, mos_core::LifecycleState::Trashed)
        .unwrap();
    storage.delete_task(task.id).unwrap();

    let objetivos = storage.objectives(id).unwrap();
    assert_eq!(objetivos.len(), 1);
    assert_eq!(objetivos[0].title, "some daqui");
    assert!(
        objetivos[0].link.is_some(),
        "o par continua gravado; quem some e o alvo"
    );
}

// ------------------------------------------------------------ fim do dia

#[test]
fn encerrar_resolve_pendentes_grava_reflexao_e_fecha_a_sessao() {
    let (_dir, storage) = banco();
    let id = comecar(
        &storage,
        "2026-08-21",
        "planta",
        &["memorial", "daily session"],
        agora(),
    );
    let objetivos = storage.objectives(id).unwrap();

    let entrada = EndDayInput {
        resolutions: vec![
            ObjectiveResolution {
                objective_id: objetivos[0].id.to_string(),
                status: "completed".into(),
            },
            ObjectiveResolution {
                objective_id: objetivos[1].id.to_string(),
                status: "carried_over".into(),
            },
            ObjectiveResolution {
                objective_id: objetivos[2].id.to_string(),
                status: "dropped".into(),
            },
        ],
        mood: "blocked".into(),
        summary: "o 063-26 tomou mais tempo que o esperado".into(),
    };
    let destinos = entrada.parsed_resolutions().unwrap();
    let reflexao = entrada.reflection().unwrap().unwrap().for_session(id);
    let fechada = storage
        .end_day(id, &destinos, Some(reflexao), agora() + Duration::hours(8))
        .unwrap();

    assert_eq!(fechada.status, SessionStatus::Completed);
    assert!(fechada.ended_at.is_some());

    let depois = storage.objectives(id).unwrap();
    assert_eq!(depois[0].status, ObjectiveStatus::Completed);
    assert!(depois[0].completed_at.is_some());
    assert_eq!(depois[1].status, ObjectiveStatus::CarriedOver);
    assert!(depois[1].completed_at.is_none());
    assert_eq!(depois[2].status, ObjectiveStatus::Dropped);

    let guardada = storage.reflection(id).unwrap().unwrap();
    assert_eq!(guardada.mood, Some(mos_core::DayMood::Blocked));
    assert!(guardada.summary.starts_with("o 063-26"));
}

#[test]
fn objetivo_sem_destino_fica_pendente_e_nao_e_abandonado() {
    // Nao decidir e uma resposta valida. Transformar silencio em "abandonado"
    // seria o sistema escolhendo por quem nao escolheu — e o pendente e
    // exatamente o que reaparece no carry-over de amanha.
    let (_dir, storage) = banco();
    let id = comecar(&storage, "2026-08-21", "planta", &["memorial"], agora());
    storage.end_day(id, &[], None, agora()).unwrap();

    let depois = storage.objectives(id).unwrap();
    assert!(depois
        .iter()
        .all(|objetivo| objetivo.status == ObjectiveStatus::Pending));
    assert!(
        storage.reflection(id).unwrap().is_none(),
        "reflexao vazia nao vira linha"
    );
}

#[test]
fn reabrir_devolve_o_dia_e_recusa_dois_abertos() {
    let (_dir, storage) = banco();
    let id = comecar(&storage, "2026-08-21", "planta", &[], agora());
    storage.end_day(id, &[], None, agora()).unwrap();

    let reaberta = storage.reopen_day(id, agora()).unwrap();
    assert_eq!(reaberta.status, SessionStatus::Active);
    assert!(reaberta.ended_at.is_none(), "reabrir limpa a hora de fim");

    storage.end_day(id, &[], None, agora()).unwrap();
    let outro = comecar(
        &storage,
        "2026-08-22",
        "outro dia",
        &[],
        agora() + Duration::days(1),
    );
    let erro = storage.reopen_day(id, agora()).unwrap_err();
    assert!(
        erro.message.contains("dia aberto"),
        "veio: {}",
        erro.message
    );
    assert_eq!(
        DailyRepository::session(&storage, outro).unwrap().status,
        SessionStatus::Active
    );
}

// -------------------------------------------------- a sessao que ficou aberta

#[test]
fn comecar_hoje_fecha_o_dia_de_ontem_sem_apagar_o_que_ficou_pendente() {
    let (_dir, storage) = banco();
    let ontem = comecar(
        &storage,
        "2026-08-20",
        "planta",
        &["memorial"],
        agora() - Duration::days(1),
    );

    assert!(
        storage.stale_session(&dia("2026-08-21")).unwrap().is_some(),
        "antes de comecar hoje, ontem aparece como porta aberta"
    );

    comecar(&storage, "2026-08-21", "hoje", &[], agora());

    let fechada = DailyRepository::session(&storage, ontem).unwrap();
    assert_eq!(fechada.status, SessionStatus::Completed);
    assert!(fechada.ended_at.is_some());
    assert!(
        storage
            .objectives(ontem)
            .unwrap()
            .iter()
            .all(|o| o.status == ObjectiveStatus::Pending),
        "fechar por conta nao pode decidir o destino dos objetivos por ninguem"
    );
    assert!(
        storage.stale_session(&dia("2026-08-21")).unwrap().is_none(),
        "e depois nao ha mais porta aberta"
    );
}

// --------------------------------------------------------------- carry-over

#[test]
fn a_corrente_de_carry_over_conta_os_elos() {
    let (_dir, storage) = banco();

    let mut anterior: Option<DailyObjectiveId> = None;
    for (indice, day) in ["2026-08-18", "2026-08-19", "2026-08-20"]
        .iter()
        .enumerate()
    {
        let quando = agora() - Duration::days(3 - indice as i64);
        let sessao = NewDailySession::create(dia(day), "", quando).unwrap();
        let id = sessao.id;
        let mut objetivo = NewDailyObjective::create(
            id,
            "Atualizar documentacao do M/OS",
            "",
            None,
            ObjectivePriority::Main,
            0,
            quando,
        )
        .unwrap();
        if let Some(origem) = anterior {
            objetivo = objetivo.carried_from(origem);
        }
        anterior = Some(objetivo.id);
        storage.start_day(sessao, vec![objetivo], quando).unwrap();
    }

    let ultimo = anterior.unwrap();
    assert_eq!(
        storage.carry_depth(ultimo).unwrap(),
        2,
        "dois elos atras dele"
    );

    let anterior_a_hoje = storage.session_before(&dia("2026-08-21")).unwrap().unwrap();
    assert_eq!(
        anterior_a_hoje.day.as_str(),
        "2026-08-20",
        "o carry-over vem do dia mais recente"
    );
}

#[test]
fn remover_o_elo_antigo_nao_derruba_o_novo() {
    let (_dir, storage) = banco();
    let ontem = comecar(
        &storage,
        "2026-08-20",
        "documentacao",
        &[],
        agora() - Duration::days(1),
    );
    let origem = storage.objectives(ontem).unwrap()[0].id;

    let sessao = NewDailySession::create(dia("2026-08-21"), "", agora()).unwrap();
    let hoje = sessao.id;
    storage
        .start_day(
            sessao,
            vec![NewDailyObjective::create(
                hoje,
                "documentacao",
                "",
                None,
                ObjectivePriority::Main,
                0,
                agora(),
            )
            .unwrap()
            .carried_from(origem)],
            agora(),
        )
        .unwrap();

    storage.remove_objective(origem).unwrap();

    let atual = &storage.objectives(hoje).unwrap()[0];
    assert_eq!(atual.title, "documentacao", "o objetivo de hoje sobrevive");
    assert!(
        atual.carried_from.is_none(),
        "a corrente perde o elo, e nao o objetivo"
    );
}

// ------------------------------------------------------------------ ordem

#[test]
fn reordenar_grava_a_lista_inteira_e_ignora_id_de_outra_sessao() {
    let (_dir, storage) = banco();
    let id = comecar(
        &storage,
        "2026-08-21",
        "principal",
        &["a", "b", "c"],
        agora(),
    );
    let outro = comecar(
        &storage,
        "2026-08-22",
        "outro",
        &[],
        agora() + Duration::days(1),
    );
    let intruso = storage.objectives(outro).unwrap()[0].id;

    let objetivos = storage.objectives(id).unwrap();
    let ordem: Vec<_> = [3usize, 2, 1, 0]
        .iter()
        .map(|indice| objetivos[*indice].id)
        .chain(std::iter::once(intruso))
        .collect();
    let depois = storage.reorder_objectives(id, &ordem, agora()).unwrap();

    // O principal continua primeiro na LEITURA — a ordem da lista e "principal,
    // depois posicao" — mas as posicoes gravadas seguem o pedido.
    let posicao = |titulo: &str| depois.iter().find(|o| o.title == titulo).unwrap().position;
    assert_eq!(posicao("c"), 0);
    assert_eq!(posicao("b"), 1);
    assert_eq!(posicao("a"), 2);
    assert_eq!(posicao("principal"), 3);

    assert_eq!(
        storage.objectives(outro).unwrap()[0].position,
        0,
        "id de outra sessao nao reordena o dia dela"
    );
}

// -------------------------------------------------------------- historico

#[test]
fn o_historico_traz_as_sessoes_da_mais_nova_para_a_mais_antiga() {
    let (_dir, storage) = banco();
    comecar(
        &storage,
        "2026-08-19",
        "a",
        &[],
        agora() - Duration::days(2),
    );
    comecar(
        &storage,
        "2026-08-20",
        "b",
        &[],
        agora() - Duration::days(1),
    );
    comecar(&storage, "2026-08-21", "c", &[], agora());

    let dias: Vec<_> = storage
        .sessions(10)
        .unwrap()
        .iter()
        .map(|sessao| sessao.day.to_string())
        .collect();
    assert_eq!(dias, ["2026-08-21", "2026-08-20", "2026-08-19"]);

    let ids: Vec<_> = storage.sessions(10).unwrap().iter().map(|s| s.id).collect();
    let todos = storage.objectives_of(&ids).unwrap();
    assert_eq!(
        todos.len(),
        3,
        "os objetivos de N dias vem numa consulta so"
    );
    assert!(storage.objectives_of(&[]).unwrap().is_empty());
}

// ----------------------------------------------------------------- busca

#[test]
fn os_objetivos_entram_na_busca_com_o_dia_em_que_foram_escritos() {
    let (_dir, storage) = banco();
    comecar(
        &storage,
        "2026-08-20",
        "Revisar memorial descritivo",
        &[],
        agora() - Duration::days(1),
    );
    comecar(
        &storage,
        "2026-08-21",
        "Finalizar planta de formas",
        &["outra coisa"],
        agora(),
    );

    let achados = storage
        .search_objectives(SearchRequest {
            query: "memorial".into(),
            include_archived: false,
            limit: 10,
        })
        .unwrap();
    assert_eq!(achados.len(), 1);
    assert_eq!(achados[0].0.title, "Revisar memorial descritivo");
    assert_eq!(achados[0].1.as_str(), "2026-08-20");

    // `%` e `_` sao curingas do LIKE. Sem escape, procurar por "%" devolveria a
    // base inteira — e uma busca que devolve tudo e uma busca quebrada.
    assert!(storage
        .search_objectives(SearchRequest {
            query: "%".into(),
            include_archived: false,
            limit: 10
        })
        .unwrap()
        .is_empty());
}

// ------------------------------------------------------------------ sync

#[test]
fn comecar_o_dia_enfileira_a_sessao_e_cada_objetivo() {
    use mos_sync::{OpBody, OutboxRepository};
    let (_dir, storage) = com_sync();
    let id = comecar(&storage, "2026-08-21", "planta", &["memorial"], agora());

    let ops = storage.pendentes(10).unwrap();
    assert_eq!(ops.len(), 3, "uma sessao e dois objetivos");
    assert_eq!(ops[0].entity.kind.as_str(), "daily_session");
    assert_eq!(ops[0].entity.id, id.as_uuid());
    assert_eq!(ops[1].entity.kind.as_str(), "daily_objective");

    let campos = match &ops[0].body {
        OpBody::Create { fields } => fields.clone(),
        outro => panic!("esperava Create, veio {outro:?}"),
    };
    assert_eq!(
        campos["day"],
        serde_json::json!("2026-08-21"),
        "a sessao precisa viajar sabendo de que dia ela e: o id e um UUID e nao diz nada"
    );
}

#[test]
fn concluir_pelo_kanban_emite_a_task_e_o_objetivo() {
    use mos_sync::OutboxRepository;
    let (_dir, storage) = com_sync();
    let task = storage
        .create_task(NewTask::create("enviar", "", None).unwrap())
        .unwrap();
    let sessao = NewDailySession::create(dia("2026-08-21"), "", agora()).unwrap();
    let id = sessao.id;
    storage
        .start_day(
            sessao,
            vec![NewDailyObjective::create(
                id,
                "enviar",
                "",
                Some(ObjectiveLink::new(LinkKind::Task, &task.id.to_string()).unwrap()),
                ObjectivePriority::Main,
                0,
                agora(),
            )
            .unwrap()],
            agora(),
        )
        .unwrap();
    let antes = storage.pendentes(100).unwrap().len();

    storage.set_task_state(task.id, TaskState::Done).unwrap();

    let ops = storage.pendentes(100).unwrap();
    let novas = &ops[antes..];
    let tipos: Vec<_> = novas.iter().map(|op| op.entity.kind.as_str()).collect();
    assert_eq!(
        tipos,
        ["task", "daily_objective"],
        "a mudanca do dia sai na MESMA transacao da mudanca da Task"
    );
}

#[test]
fn remover_um_objetivo_emite_delete() {
    use mos_sync::{OpBody, OutboxRepository};
    let (_dir, storage) = com_sync();
    let id = comecar(&storage, "2026-08-21", "planta", &["some"], agora());
    let some = storage
        .objectives(id)
        .unwrap()
        .into_iter()
        .find(|o| o.title == "some")
        .unwrap();

    storage.remove_objective(some.id).unwrap();

    let ultima = storage.pendentes(100).unwrap().pop().unwrap();
    assert_eq!(ultima.entity.kind.as_str(), "daily_objective");
    assert!(matches!(ultima.body, OpBody::Delete));
}

#[test]
fn sem_sync_ligado_nada_e_emitido_e_nada_falha() {
    use mos_sync::OutboxRepository;
    let (_dir, storage) = banco();
    let id = comecar(&storage, "2026-08-21", "planta", &["memorial"], agora());
    storage
        .end_day(
            id,
            &[],
            NewDailyReflection::create(None, "foi bom"),
            agora(),
        )
        .unwrap();
    assert!(storage.pendentes(10).unwrap().is_empty());
}
