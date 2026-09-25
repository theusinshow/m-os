//! O planejamento da Task atravessa dispositivos, e a saude do sync sobrevive
//! a rodada que falhou.
//!
//! Tres coisas que a migration 0041 prometeu, provadas com dois bancos SQLite
//! de verdade e um hub em memoria:
//!
//! 1. planejar, comecar e adiar emitem SO o campo tocado — `due_at` nao viaja
//!    junto, entao mover para amanha num aparelho nao apaga o prazo posto no
//!    outro;
//! 2. o que foi planejado num PC aparece planejado no outro;
//! 3. uma rodada contra um hub caido sobe a escada do backoff, e a rodada
//!    seguinte que funciona zera — persistido, e nao em memoria.

use std::collections::BTreeMap;
use std::sync::Mutex;

use mos_core::{Day, NewTask, TaskState, WorkRepository};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::{DeviceRepository, Lote, Op, OpBody, OutboxRepository, Resultado, Transport};
use time::macros::datetime;
use uuid::Uuid;

#[derive(Default)]
struct HubLocal {
    log: Mutex<Vec<Op>>,
    caido: Mutex<bool>,
}

impl Transport for &HubLocal {
    fn push(&self, _contrato: u32, ops: &[Op]) -> Resultado<Vec<Uuid>> {
        if *self.caido.lock().unwrap() {
            return Err(mos_sync::SyncError::novo(
                "Sem alcancar o hub: connection refused",
                true,
            ));
        }
        let mut log = self.log.lock().unwrap();
        let mut aceitas = Vec::new();
        for op in ops {
            if !log.iter().any(|e| e.id == op.id) {
                log.push(op.clone());
            }
            aceitas.push(op.id);
        }
        Ok(aceitas)
    }

    fn pull(&self, _contrato: u32, cursor: &str, limite: usize) -> Resultado<Lote> {
        if *self.caido.lock().unwrap() {
            return Err(mos_sync::SyncError::novo(
                "Sem alcancar o hub: connection refused",
                true,
            ));
        }
        let log = self.log.lock().unwrap();
        let desde: usize = cursor.parse().unwrap_or(0);
        let ops: Vec<Op> = log.iter().skip(desde).take(limite).cloned().collect();
        let proximo = desde + ops.len();
        Ok(Lote {
            ops,
            proximo_cursor: proximo.to_string(),
            tem_mais: proximo < log.len(),
        })
    }
}

fn aparelho(nome: &str) -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap();
    let eu = storage.este_dispositivo(nome, "windows", "0.5.1").unwrap();
    storage.habilitar_sync(eu.id).unwrap();
    (dir, storage)
}

fn rodada(storage: &SqliteStorage, hub: &HubLocal) -> mos_sync::Rodada {
    let agora = time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
    storage.sincronizar_agora(&hub, agora as i64, 100).unwrap()
}

fn campos(op: &Op) -> BTreeMap<String, serde_json::Value> {
    match &op.body {
        OpBody::Create { fields } | OpBody::Update { fields } => {
            fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        }
        outro => panic!("esperava campos, veio {outro:?}"),
    }
}

#[test]
fn planejar_emite_so_o_dia_e_adiar_conta() {
    let (_d, pc) = aparelho("PC");
    let task = pc
        .create_task(NewTask::create("Revisar Caixa 01", "", None).unwrap())
        .unwrap();
    // Limpa a criacao da fila para olhar so o que vem depois.
    let ids: Vec<_> = pc.pendentes(10).unwrap().iter().map(|op| op.id).collect();
    pc.confirmar(&ids).unwrap();

    let hoje = Day::parse("2026-09-15").unwrap();
    let planejada = pc.plan_task(task.id, Some(hoje.clone()), false).unwrap();
    assert_eq!(planejada.scheduled_for, Some(hoje));
    assert_eq!(
        planejada.postponed_count, 0,
        "planejar pela primeira vez nao e adiar"
    );

    let ops = pc.pendentes(10).unwrap();
    assert_eq!(ops.len(), 1);
    let c = campos(&ops[0]);
    assert_eq!(
        c.get("scheduledFor"),
        Some(&serde_json::json!("2026-09-15"))
    );
    assert!(
        !c.contains_key("dueAt"),
        "o prazo nao viaja junto do planejamento"
    );
    assert!(!c.contains_key("postponedCount"));
    pc.confirmar(&[ops[0].id]).unwrap();

    let amanha = Day::parse("2026-09-16").unwrap();
    let adiada = pc.plan_task(task.id, Some(amanha), true).unwrap();
    assert_eq!(adiada.postponed_count, 1);
    let ops = pc.pendentes(10).unwrap();
    let c = campos(&ops[0]);
    assert_eq!(c.get("postponedCount"), Some(&serde_json::json!(1)));
    pc.confirmar(&[ops[0].id]).unwrap();
    let repetida = pc
        .plan_task(task.id, adiada.scheduled_for.clone(), true)
        .unwrap();
    assert_eq!(repetida.postponed_count, 1, "retry não é um novo adiamento");
    assert!(pc.pendentes(10).unwrap().is_empty());
}

#[test]
fn trocar_foco_valida_antes_de_parar_e_sincroniza_as_duas_pontas() {
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC");
    let (_b, celular) = aparelho("Bolso");
    let a = pc
        .create_task(NewTask::create("Primeira", "", None).unwrap())
        .unwrap();
    let b = pc
        .create_task(NewTask::create("Próxima", "", None).unwrap())
        .unwrap();
    let inicio = datetime!(2026-09-24 12:00 UTC);
    pc.set_task_started(a.id, Some(inicio)).unwrap();
    assert!(pc
        .set_task_started(mos_core::TaskId::new(), Some(inicio))
        .is_err());
    assert_eq!(pc.get_task(a.id).unwrap().started_at, Some(inicio));
    pc.set_task_state(b.id, TaskState::Done).unwrap();
    assert!(pc.set_task_started(b.id, Some(inicio)).is_err());
    assert_eq!(pc.get_task(a.id).unwrap().started_at, Some(inicio));
    pc.set_task_state(b.id, TaskState::Backlog).unwrap();
    pc.set_task_started(b.id, Some(inicio)).unwrap();
    rodada(&pc, &hub);
    rodada(&celular, &hub);
    for device in [&pc, &celular] {
        assert!(device.get_task(a.id).unwrap().started_at.is_none());
        assert_eq!(device.get_task(b.id).unwrap().started_at, Some(inicio));
    }
}

#[test]
fn reordenacao_de_outra_task_nao_emite_mudanca_fantasma() {
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC");
    let (_b, celular) = aparelho("Bolso");
    let a = pc
        .create_task(NewTask::create("A", "", None).unwrap())
        .unwrap();
    let b = pc
        .create_task(NewTask::create("B", "", None).unwrap())
        .unwrap();
    let item = mos_core::NewChecklistItem::create(b.id, "Pertence a B").unwrap();
    let item_id = item.id;
    pc.add_checklist_item(item).unwrap();
    rodada(&pc, &hub);
    rodada(&celular, &hub);
    pc.reorder_checklist(a.id, &[mos_core::ChecklistItemId::new(), item_id])
        .unwrap();
    assert!(!pc
        .pendentes(20)
        .unwrap()
        .iter()
        .any(|op| op.entity.id == item_id.as_uuid()));
    rodada(&pc, &hub);
    rodada(&celular, &hub);
    for device in [&pc, &celular] {
        assert_eq!(device.task_detail(b.id).unwrap().checklist[0].position, 0);
    }
}

#[test]
fn contexto_do_hermes_contem_os_passos_e_a_espera_reais() {
    let (_a, pc) = aparelho("PC");
    let task = pc
        .create_task(NewTask::create("Caixa 02", "Memorial", None).unwrap())
        .unwrap();
    let mut edit = mos_core::EditTask::from_task(&task);
    edit.waiting_for = "Victor enviar bases".into();
    edit.follow_up_at = Some(datetime!(2026-09-25 12:00 UTC));
    pc.update_task(task.id, edit).unwrap();
    pc.add_checklist_item(mos_core::NewChecklistItem::create(task.id, "Conferir cotas").unwrap())
        .unwrap();
    let context = mos_core::task_context(&pc.task_detail(task.id).unwrap());
    assert!(context.contains("- [ ] Conferir cotas"));
    assert!(context.contains("Aguardando: Victor enviar bases"));
    assert!(context.contains("Follow-up:"));
}

#[test]
fn comecar_poe_em_doing_e_concluir_limpa_o_inicio() {
    let (_d, pc) = aparelho("PC");
    let task = pc
        .create_task(NewTask::create("Enviar arquivos", "", None).unwrap())
        .unwrap();
    let inicio = datetime!(2026-09-15 13:00 UTC);
    let comecada = pc.set_task_started(task.id, Some(inicio)).unwrap();
    assert_eq!(comecada.started_at, Some(inicio));
    assert_eq!(comecada.state, TaskState::Doing);

    let parada = pc.set_task_started(task.id, None).unwrap();
    assert_eq!(parada.started_at, None);
    assert_eq!(
        parada.state,
        TaskState::Doing,
        "parar nao desfaz 'estou nisto'"
    );

    pc.set_task_started(task.id, Some(inicio)).unwrap();
    let feita = pc.set_task_state(task.id, TaskState::Done).unwrap();
    assert_eq!(feita.started_at, None, "concluir encerra a comecada");
    let ultima = pc.pendentes(20).unwrap().pop().unwrap();
    let c = campos(&ultima);
    assert_eq!(c.get("startedAt"), Some(&serde_json::Value::Null));
}

#[test]
fn o_planejamento_atravessa_e_o_prazo_do_outro_lado_fica() {
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC");
    let (_b, celular) = aparelho("iPhone");

    let task = pc
        .create_task(NewTask::create("Trabalho da faculdade", "", None).unwrap())
        .unwrap();
    rodada(&pc, &hub);
    rodada(&celular, &hub);
    assert!(celular.get_task(task.id).is_ok());

    // O PC poe o prazo; o celular, offline, planeja o dia. Os dois gestos
    // tocam campos diferentes e precisam conviver.
    let mut edicao = mos_core::EditTask::from_task(&task);
    edicao.due_at = Some(datetime!(2026-09-20 17:00 UTC));
    pc.update_task(task.id, edicao).unwrap();
    celular
        .plan_task(task.id, Some(Day::parse("2026-09-16").unwrap()), false)
        .unwrap();

    rodada(&pc, &hub);
    rodada(&celular, &hub);
    rodada(&pc, &hub);

    let no_pc = pc.get_task(task.id).unwrap();
    let no_celular = celular.get_task(task.id).unwrap();
    assert_eq!(
        no_pc.scheduled_for.as_ref().map(|d| d.as_str().to_owned()),
        Some("2026-09-16".into())
    );
    assert_eq!(no_celular.due_at, Some(datetime!(2026-09-20 17:00 UTC)));
    assert_eq!(no_pc.due_at, no_celular.due_at);
    assert_eq!(no_pc.scheduled_for, no_celular.scheduled_for);
}

#[test]
fn a_saude_sobe_no_hub_caido_e_zera_quando_volta() {
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC");
    pc.create_task(NewTask::create("Qualquer", "", None).unwrap())
        .unwrap();

    *hub.caido.lock().unwrap() = true;
    let r = rodada(&pc, &hub);
    assert!(r.erro.is_some());
    assert!(r.erro_retriavel, "sem rede e passageiro");
    let agora = time::OffsetDateTime::now_utc();
    let registro = pc
        .registrar_rodada_de_sync(r.erro.as_deref().map(|m| (m, r.erro_retriavel)), agora)
        .unwrap();
    assert_eq!(registro.falhas_seguidas, 1);
    assert_eq!(registro.tipo_do_erro, Some(mos_sync::TipoDeFalha::Offline));
    assert_eq!(registro.espera(), std::time::Duration::from_secs(10));
    assert_eq!(
        pc.quantidade_pendente().unwrap(),
        1,
        "a operacao continua na fila"
    );

    // Reabrir o app le o mesmo registro: a escada nao mora em memoria.
    let reaberto = pc.saude_do_sync().unwrap();
    assert_eq!(reaberto, registro);

    *hub.caido.lock().unwrap() = false;
    let r = rodada(&pc, &hub);
    assert!(r.erro.is_none());
    let registro = pc
        .registrar_rodada_de_sync(None, agora + time::Duration::seconds(10))
        .unwrap();
    assert_eq!(registro.falhas_seguidas, 0);
    assert!(registro.ultimo_ok_em.is_some());
    assert_eq!(pc.quantidade_pendente().unwrap(), 0);
    let eu = pc
        .listar()
        .unwrap()
        .into_iter()
        .find(|d| d.is_this_device)
        .unwrap();
    assert!(
        !eu.last_sync_at.is_empty(),
        "o dispositivo carimba a ultima sincronizacao"
    );
}

// ---------------------------------------------------------------- F1.1: Undo

fn desfazer(
    storage: &SqliteStorage,
    efeito: &mos_core::TaskStartEffect,
) -> Result<mos_core::TaskStartUndone, mos_core::CoreError> {
    storage.undo_task_start(
        efeito.task.id,
        efeito.started_at,
        efeito.previous_state,
        efeito.previous_started_at,
        &efeito.interrupted,
    )
}

#[test]
fn desfazer_inicio_devolve_o_foco_e_converge_nas_duas_replicas() {
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC");
    let (_b, celular) = aparelho("Bolso");
    let a = pc
        .create_task(NewTask::create("A", "", None).unwrap())
        .unwrap();
    let b = pc
        .create_task(NewTask::create("B", "", None).unwrap())
        .unwrap();
    let inicio_a = datetime!(2026-09-24 09:00 UTC);
    pc.set_task_started(a.id, Some(inicio_a)).unwrap();

    let efeito = pc
        .start_task_recorded(b.id, datetime!(2026-09-24 10:00 UTC))
        .unwrap();
    assert_eq!(efeito.previous_state, b.state);
    assert_eq!(efeito.interrupted, vec![(a.id, inicio_a)]);

    let desfeito = desfazer(&pc, &efeito).unwrap();
    assert!(desfeito.notes.is_empty(), "{:?}", desfeito.notes);
    assert_eq!(desfeito.resumed, vec![a.id]);

    rodada(&pc, &hub);
    rodada(&celular, &hub);
    for device in [&pc, &celular] {
        let b = device.get_task(b.id).unwrap();
        assert!(b.started_at.is_none(), "B volta a não ter começado");
        assert_eq!(b.state, efeito.previous_state, "B volta à coluna de antes");
        assert_eq!(device.get_task(a.id).unwrap().started_at, Some(inicio_a));
    }

    // Repetir o desfazer não corrompe nada: B já não tem o instante gravado.
    let repetido = desfazer(&pc, &efeito).unwrap_err();
    assert_eq!(repetido.code, mos_core::ErrorCode::Conflict);
    assert_eq!(pc.get_task(a.id).unwrap().started_at, Some(inicio_a));
}

#[test]
fn desfazer_inicio_nao_sobrescreve_o_que_veio_depois() {
    let (_a, pc) = aparelho("PC");
    let a = pc
        .create_task(NewTask::create("A", "", None).unwrap())
        .unwrap();
    let b = pc
        .create_task(NewTask::create("B", "", None).unwrap())
        .unwrap();
    pc.set_task_started(a.id, Some(datetime!(2026-09-24 09:00 UTC)))
        .unwrap();
    let efeito = pc
        .start_task_recorded(b.id, datetime!(2026-09-24 10:00 UTC))
        .unwrap();

    // Concluir B depois de começar é uma decisão nova: o desfazer recusa.
    pc.set_task_state(b.id, TaskState::Done).unwrap();
    let recusa = desfazer(&pc, &efeito).unwrap_err();
    assert_eq!(recusa.code, mos_core::ErrorCode::Conflict);
    assert_eq!(pc.get_task(b.id).unwrap().state, TaskState::Done);
    assert!(pc.get_task(a.id).unwrap().started_at.is_none());
}

#[test]
fn desfazer_inicio_nao_religa_foco_quando_outra_replica_comecou_algo() {
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC");
    let (_b, celular) = aparelho("Bolso");
    let a = pc
        .create_task(NewTask::create("A", "", None).unwrap())
        .unwrap();
    let b = pc
        .create_task(NewTask::create("B", "", None).unwrap())
        .unwrap();
    let c = pc
        .create_task(NewTask::create("C", "", None).unwrap())
        .unwrap();
    pc.set_task_started(a.id, Some(datetime!(2026-09-24 09:00 UTC)))
        .unwrap();
    rodada(&pc, &hub);
    rodada(&celular, &hub);

    // Offline: o PC troca A por B; o celular começa C.
    let efeito = pc
        .start_task_recorded(b.id, datetime!(2026-09-24 10:00 UTC))
        .unwrap();
    celular
        .set_task_started(c.id, Some(datetime!(2026-09-24 10:05 UTC)))
        .unwrap();
    rodada(&celular, &hub);
    rodada(&pc, &hub);
    assert!(pc.get_task(c.id).unwrap().started_at.is_some());

    // Desfazer B no PC: B volta, mas A NÃO religa — C já está em andamento.
    let desfeito = desfazer(&pc, &efeito).unwrap();
    assert!(desfeito.resumed.is_empty());
    assert_eq!(desfeito.notes.len(), 1, "{:?}", desfeito.notes);
    assert!(desfeito.notes[0].contains("outra Task"));
    assert!(pc.get_task(b.id).unwrap().started_at.is_none());
    assert!(pc.get_task(a.id).unwrap().started_at.is_none());
}

// ------------------------------------------------------ F1.3: edição por campo

#[test]
fn titulo_num_aparelho_e_conclusao_no_outro_sobrevivem_juntos() {
    use mos_core::{
        DailyRepository, NewDailyObjective, NewDailySession, ObjectiveEdit, ObjectivePriority,
        ObjectiveStatus,
    };
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC");
    let (_b, celular) = aparelho("Bolso");
    let quando = datetime!(2026-09-24 09:00 UTC);
    let sessao = NewDailySession::create(Day::parse("2026-09-24").unwrap(), "", quando).unwrap();
    let objetivo = NewDailyObjective::create(
        sessao.id,
        "Revisar memorial",
        "",
        None,
        ObjectivePriority::Main,
        0,
        quando,
    )
    .unwrap();
    let id = objetivo.id;
    pc.start_day(sessao, vec![objetivo], quando).unwrap();
    rodada(&pc, &hub);
    rodada(&celular, &hub);

    // Offline: o PC reescreve o título; o celular conclui.
    pc.edit_objective(
        id,
        &ObjectiveEdit::Text {
            title: "Revisar memorial da Caixa 02".into(),
            description: String::new(),
        },
        datetime!(2026-09-24 10:00 UTC),
    )
    .unwrap();
    celular
        .edit_objective(
            id,
            &ObjectiveEdit::Status(ObjectiveStatus::Completed),
            datetime!(2026-09-24 10:01 UTC),
        )
        .unwrap();

    // Cada gesto emitiu só o próprio campo.
    let do_pc = pc.pendentes(10).unwrap();
    assert_eq!(do_pc.len(), 1);
    assert!(campos(&do_pc[0]).contains_key("title"));
    assert!(!campos(&do_pc[0]).contains_key("status"));

    for _ in 0..2 {
        rodada(&pc, &hub);
        rodada(&celular, &hub);
    }
    for device in [&pc, &celular] {
        let visto = device.objective(id).unwrap();
        assert_eq!(visto.title, "Revisar memorial da Caixa 02");
        assert_eq!(visto.status, ObjectiveStatus::Completed);
        assert!(visto.completed_at.is_some());
    }

    // Um gesto que não muda nada não deixa operação na outbox.
    let antes = pc.pendentes(10).unwrap().len();
    pc.edit_objective(
        id,
        &ObjectiveEdit::Status(ObjectiveStatus::Completed),
        datetime!(2026-09-24 11:00 UTC),
    )
    .unwrap();
    assert_eq!(pc.pendentes(10).unwrap().len(), antes);
    // Objetivo inexistente: recusado, e nada na outbox.
    assert!(pc
        .edit_objective(
            mos_core::DailyObjectiveId::new(),
            &ObjectiveEdit::Priority(ObjectivePriority::Secondary),
            datetime!(2026-09-24 11:00 UTC),
        )
        .is_err());
    assert_eq!(pc.pendentes(10).unwrap().len(), antes);
}

// ------------------------------------------------- F3.2: foco entre réplicas

fn em_andamento(storage: &std::sync::Arc<SqliteStorage>) -> Vec<mos_core::TaskId> {
    mos_core::WorkService::new(storage.clone())
        .tasks(false)
        .unwrap()
        .into_iter()
        .filter(|task| task.started_at.is_some())
        .map(|task| task.id)
        .collect()
}

#[test]
fn inicios_offline_convergem_para_o_mais_recente_em_qualquer_ordem() {
    use std::sync::Arc;
    for pc_primeiro in [true, false] {
        let hub = HubLocal::default();
        let (_a, pc) = aparelho("PC");
        let (_b, celular) = aparelho("Bolso");
        let (pc, celular) = (Arc::new(pc), Arc::new(celular));
        let a = pc
            .create_task(NewTask::create("A", "", None).unwrap())
            .unwrap();
        let b = pc
            .create_task(NewTask::create("B", "", None).unwrap())
            .unwrap();
        rodada(&pc, &hub);
        rodada(&celular, &hub);

        // Offline: o PC começa A às 10h; o celular começa B às 10h05.
        pc.set_task_started(a.id, Some(datetime!(2026-09-24 10:00 UTC)))
            .unwrap();
        celular
            .set_task_started(b.id, Some(datetime!(2026-09-24 10:05 UTC)))
            .unwrap();
        let (primeiro, segundo) = if pc_primeiro {
            (&pc, &celular)
        } else {
            (&celular, &pc)
        };
        for _ in 0..2 {
            rodada(primeiro, &hub);
            rodada(segundo, &hub);
        }

        // As duas réplicas veem o mesmo foco: B, o mais recente. O início de A
        // continua gravado — histórico, não cronômetro.
        for device in [&pc, &celular] {
            assert_eq!(
                em_andamento(device),
                vec![b.id],
                "pc_primeiro={pc_primeiro}"
            );
            assert!(device.get_task(a.id).unwrap().started_at.is_some());
        }

        // Parar B no celular não ressuscita A em lugar nenhum.
        celular.set_task_started(b.id, None).unwrap();
        for _ in 0..2 {
            rodada(&celular, &hub);
            rodada(&pc, &hub);
        }
        for device in [&pc, &celular] {
            assert!(em_andamento(device).is_empty(), "pc_primeiro={pc_primeiro}");
            assert!(device.get_task(a.id).unwrap().started_at.is_none());
        }
    }
}
