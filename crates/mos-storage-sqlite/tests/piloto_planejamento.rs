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
        c.get("dueAt").is_none(),
        "o prazo nao viaja junto do planejamento"
    );
    assert!(c.get("postponedCount").is_none());
    pc.confirmar(&[ops[0].id]).unwrap();

    let amanha = Day::parse("2026-09-16").unwrap();
    let adiada = pc.plan_task(task.id, Some(amanha), true).unwrap();
    assert_eq!(adiada.postponed_count, 1);
    let ops = pc.pendentes(10).unwrap();
    let c = campos(&ops[0]);
    assert_eq!(c.get("postponedCount"), Some(&serde_json::json!(1)));
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
