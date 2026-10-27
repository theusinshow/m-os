//! O Workspace dos itens soltos (migration 0043, ADR-093).
//!
//! A Task, a Capture e a conversa guardam o Workspace em que nasceram, a Task
//! triada herda o da Capture, e o campo atravessa para o outro aparelho.

use std::sync::Mutex;

use mos_core::{
    CaptureRepository, CaptureSource, ConversationRepository, NewCapture, NewConversation, NewTask,
    NewWorkspace, WorkRepository,
};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::{DeviceRepository, Lote, Op, Resultado, Transport};
use uuid::Uuid;

#[derive(Default)]
struct HubLocal {
    log: Mutex<Vec<Op>>,
}

impl Transport for &HubLocal {
    fn push(&self, _contrato: u32, ops: &[Op]) -> Resultado<Vec<Uuid>> {
        let mut log = self.log.lock().unwrap();
        for op in ops {
            if !log.iter().any(|e| e.id == op.id) {
                log.push(op.clone());
            }
        }
        Ok(ops.iter().map(|op| op.id).collect())
    }

    fn pull(&self, _contrato: u32, cursor: &str, limite: usize) -> Resultado<Lote> {
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

fn aparelho(nome: &str, com_sync: bool) -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap();
    if com_sync {
        let eu = storage.este_dispositivo(nome, "windows", "0.7.2").unwrap();
        storage.habilitar_sync(eu.id).unwrap();
    }
    (dir, storage)
}

fn rodada(storage: &SqliteStorage, hub: &HubLocal) {
    let agora = time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
    storage.sincronizar_agora(&hub, agora as i64, 100).unwrap();
}

#[test]
fn task_capture_e_conversa_guardam_o_workspace() {
    let (_d, pc) = aparelho("PC", false);
    let prosul = pc
        .create_workspace(NewWorkspace::create("PROSUL", "").unwrap())
        .unwrap();

    let task = pc
        .create_task(
            NewTask::create("Revisar memorial", "", None)
                .unwrap()
                .with_workspace(Some(prosul.id)),
        )
        .unwrap();
    assert_eq!(task.workspace_id, Some(prosul.id));
    assert_eq!(pc.get_task(task.id).unwrap().workspace_id, Some(prosul.id));

    let solta = pc
        .create_task(NewTask::create("Sem casa", "", None).unwrap())
        .unwrap();
    assert_eq!(solta.workspace_id, None, "sem Workspace e o padrao");

    let capture = CaptureRepository::create(
        &pc,
        NewCapture::create("ligar para o fornecedor", CaptureSource::Home)
            .unwrap()
            .with_workspace(Some(prosul.id)),
    )
    .unwrap();
    assert_eq!(capture.workspace_id, Some(prosul.id));

    let conversa = pc
        .create_conversation(NewConversation::create().with_workspace(Some(prosul.id)))
        .unwrap();
    assert_eq!(conversa.workspace_id, Some(prosul.id));
}

#[test]
fn a_task_triada_herda_o_workspace_da_capture() {
    let (_d, pc) = aparelho("PC", false);
    let prosul = pc
        .create_workspace(NewWorkspace::create("PROSUL", "").unwrap())
        .unwrap();
    let dev = pc
        .create_workspace(NewWorkspace::create("DEV", "").unwrap())
        .unwrap();
    let capture = CaptureRepository::create(
        &pc,
        NewCapture::create("orcamento da ponte", CaptureSource::Home)
            .unwrap()
            .with_workspace(Some(prosul.id)),
    )
    .unwrap();

    let herdada = pc
        .create_task_from_capture(
            capture.id,
            NewTask::create("Orcamento da ponte", "", None).unwrap(),
        )
        .unwrap();
    assert_eq!(herdada.workspace_id, Some(prosul.id));

    let outra = CaptureRepository::create(
        &pc,
        NewCapture::create("refatorar o sync", CaptureSource::Home)
            .unwrap()
            .with_workspace(Some(prosul.id)),
    )
    .unwrap();
    let explicita = pc
        .create_task_from_capture(
            outra.id,
            NewTask::create("Refatorar o sync", "", None)
                .unwrap()
                .with_workspace(Some(dev.id)),
        )
        .unwrap();
    assert_eq!(
        explicita.workspace_id,
        Some(dev.id),
        "o Workspace dito vence o herdado"
    );
}

#[test]
fn o_workspace_atravessa_para_o_outro_aparelho() {
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC", true);
    let (_b, notebook) = aparelho("Notebook", true);

    let prosul = pc
        .create_workspace(NewWorkspace::create("PROSUL", "").unwrap())
        .unwrap();
    let task = pc
        .create_task(
            NewTask::create("Revisar memorial", "", None)
                .unwrap()
                .with_workspace(Some(prosul.id)),
        )
        .unwrap();
    let capture = CaptureRepository::create(
        &pc,
        NewCapture::create("ligar para o fornecedor", CaptureSource::Home)
            .unwrap()
            .with_workspace(Some(prosul.id)),
    )
    .unwrap();

    rodada(&pc, &hub);
    rodada(&notebook, &hub);

    assert_eq!(
        notebook.get_task(task.id).unwrap().workspace_id,
        Some(prosul.id)
    );
    assert_eq!(
        CaptureRepository::get(&notebook, capture.id)
            .unwrap()
            .workspace_id,
        Some(prosul.id)
    );
}

#[test]
fn o_escopo_le_o_banco_e_aplica_a_regra() {
    use std::collections::HashSet;
    let (_d, pc) = aparelho("PC", false);
    let prosul = pc
        .create_workspace(NewWorkspace::create("PROSUL", "").unwrap())
        .unwrap();
    let dev = pc
        .create_workspace(NewWorkspace::create("DEV", "").unwrap())
        .unwrap();
    let obra = pc
        .create_project(mos_core::NewProject::create("Ponte", "", "").unwrap())
        .unwrap();
    pc.set_project_workspace(obra.id, prosul.id, true).unwrap();

    let da_obra = pc
        .create_task(NewTask::create("Memorial", "", Some(obra.id)).unwrap())
        .unwrap();
    let de_dev = pc
        .create_task(
            NewTask::create("Refatorar", "", None)
                .unwrap()
                .with_workspace(Some(dev.id)),
        )
        .unwrap();
    let solta = pc
        .create_task(NewTask::create("Solta", "", None).unwrap())
        .unwrap();

    let servico = mos_core::WorkService::new(std::sync::Arc::new(pc));
    let visto = servico.escopo_do_workspace(&prosul.id.to_string()).unwrap();
    let tasks: HashSet<String> = visto.tasks.into_iter().collect();
    assert!(tasks.contains(&da_obra.id.to_string()), "herda do Project");
    assert!(
        tasks.contains(&solta.id.to_string()),
        "sem Workspace aparece"
    );
    assert!(!tasks.contains(&de_dev.id.to_string()), "a de DEV nao");
    assert!(visto.projects.contains(&obra.id.to_string()));

    let movida = servico
        .set_task_workspace(&de_dev.id.to_string(), Some(&prosul.id.to_string()))
        .unwrap();
    assert_eq!(movida.workspace_id, Some(prosul.id));
    let visto = servico.escopo_do_workspace(&dev.id.to_string()).unwrap();
    assert!(!visto.tasks.contains(&de_dev.id.to_string()), "saiu de DEV");

    let devolvida = servico
        .set_task_workspace(&de_dev.id.to_string(), Some(""))
        .unwrap();
    assert_eq!(
        devolvida.workspace_id, None,
        "vazio devolve ao sem Workspace"
    );
}

#[test]
fn mover_a_capture_atravessa_para_o_outro_aparelho() {
    let hub = HubLocal::default();
    let (_a, pc) = aparelho("PC", true);
    let (_b, notebook) = aparelho("Notebook", true);
    let mmd = pc
        .create_workspace(NewWorkspace::create("MMD", "").unwrap())
        .unwrap();
    let capture = CaptureRepository::create(
        &pc,
        NewCapture::create("planta do terreo", CaptureSource::Home).unwrap(),
    )
    .unwrap();
    rodada(&pc, &hub);
    rodada(&notebook, &hub);
    assert_eq!(
        CaptureRepository::get(&notebook, capture.id)
            .unwrap()
            .workspace_id,
        None
    );

    pc.set_capture_workspace(capture.id, Some(mmd.id)).unwrap();
    rodada(&pc, &hub);
    rodada(&notebook, &hub);
    assert_eq!(
        CaptureRepository::get(&notebook, capture.id)
            .unwrap()
            .workspace_id,
        Some(mmd.id)
    );
}
