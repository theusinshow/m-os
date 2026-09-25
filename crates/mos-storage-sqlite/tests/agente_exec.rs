//! F4.1 do MOS-OVERHAUL-PLAN (ADR-080): o executor do Hermes roda sem janela.
//!
//! Mesmos servicos, mesmo SQLite, nenhum Tauri — e o que o bolso vai chamar.
//! Se isto passa, desktop e servidor executam a MESMA regra: o desktop so
//! acrescenta o aviso a janela e a ponte do M-Finance.

use std::sync::{Arc, Mutex};

use mos_core::agente_exec::{desfazer_acao, executar_acao, Capacidades, Contexto};
use mos_core::{
    AttentionService, CaptureService, ConversationService, DailyService, MeetingService,
    MemoryService, Servicos, SystemClock, TrackingService, UndoStep, WorkService,
};
use mos_storage_sqlite::SqliteStorage;
use time::macros::datetime;

#[derive(Default)]
struct Registro {
    avisos: Mutex<Vec<String>>,
    lembretes: Mutex<usize>,
}

impl Capacidades for Registro {
    fn avisar(&self, evento: &str, tipo: &str) {
        self.avisos.lock().unwrap().push(format!("{evento}:{tipo}"));
    }
    fn lembretes_mudaram(&self) {
        *self.lembretes.lock().unwrap() += 1;
    }
}

fn servicos(dir: &tempfile::TempDir) -> Servicos {
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = Arc::new(SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap());
    Servicos {
        captures: CaptureService::new(storage.clone()),
        work: WorkService::new(storage.clone()),
        memory: MemoryService::new(storage.clone()),
        conversations: ConversationService::new(storage.clone()),
        tracking: TrackingService::new(storage.clone()),
        meetings: MeetingService::new(storage.clone(), Arc::new(SystemClock)),
        attention: AttentionService::new(storage.clone(), Arc::new(SystemClock)),
        daily: DailyService::new(storage.clone(), Arc::new(SystemClock)),
    }
}

fn acao(json: &str, agora_local: time::OffsetDateTime) -> mos_core::ActionArgs {
    mos_core::parse_action_at(json, agora_local).unwrap()
}

#[test]
fn o_executor_do_core_cria_comeca_e_desfaz_sem_janela() {
    let dir = tempfile::tempdir().unwrap();
    let s = servicos(&dir);
    let registro = Registro::default();
    let agora_local = datetime!(2026-09-24 10:00 -03:00);
    let ctx = Contexto {
        servicos: &s,
        capacidades: &registro,
        agora: agora_local.to_offset(time::UtcOffset::UTC),
        agora_local,
    };

    let criada = executar_acao(
        &ctx,
        &acao(
            r#"{"action":"mos.task.create","args":{"title":"Revisar memorial"}}"#,
            agora_local,
        ),
    )
    .unwrap();
    assert!(criada.message.contains("Revisar memorial"));

    let comecada = executar_acao(
        &ctx,
        &acao(
            r#"{"action":"mos.task.start","args":{"task":"Revisar memorial"}}"#,
            agora_local,
        ),
    )
    .unwrap();
    assert!(registro
        .avisos
        .lock()
        .unwrap()
        .contains(&"data-changed:task".to_owned()));
    let Some(UndoStep::UndoTaskStart(passo)) = comecada.undo.clone() else {
        panic!("comecar devolve a inversa inteira: {:?}", comecada.undo)
    };
    let nota = desfazer_acao(&ctx, UndoStep::UndoTaskStart(passo)).unwrap();
    assert!(nota.is_empty(), "{nota}");
    let task = s.work.tasks(false).unwrap().pop().unwrap();
    assert!(task.started_at.is_none());

    // O lembrete acorda o agendador do aparelho, seja ele qual for.
    executar_acao(
        &ctx,
        &acao(
            r#"{"action":"mos.reminder.create","args":{"title":"Cobrar Victor","at":"2099-09-24T15:00:00-03:00"}}"#,
            agora_local,
        ),
    )
    .unwrap();
    assert_eq!(*registro.lembretes.lock().unwrap(), 1);
}

#[test]
fn acao_sem_capacidade_no_aparelho_diz_que_esta_indisponivel() {
    let dir = tempfile::tempdir().unwrap();
    let s = servicos(&dir);
    let agora_local = datetime!(2026-09-24 10:00 -03:00);
    let ctx = Contexto {
        servicos: &s,
        capacidades: &mos_core::agente_exec::SemCapacidades,
        agora: agora_local.to_offset(time::UtcOffset::UTC),
        agora_local,
    };
    let erro = executar_acao(
        &ctx,
        &mos_core::ActionArgs::MFinanceCreateBill {
            amount_cents: 1000,
            description: "Conta de luz".into(),
            due_day: Some(10),
            is_recurring: false,
        },
    )
    .unwrap_err();
    assert!(erro.message.contains("M-Finance"), "{}", erro.message);
}
