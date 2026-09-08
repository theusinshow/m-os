//! Tasks e Projects emitindo operacoes.
//!
//! Mesmo padrao da Capture, e as mesmas duas perguntas: emitiu JUNTO, e nao
//! emitiu quando esta desligado.
//!
//! O que e proprio daqui e o §80 do pedido virando teste de banco: mover uma
//! Task no Kanban num dispositivo e renomea-la no outro sao o caso de merge por
//! campo mais comum que este sistema vai ver.

use mos_core::{
    CaptureRepository, CaptureSource, LifecycleState, NewCapture, NewProject, NewTask, TaskState,
    WorkRepository,
};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::{DeviceRepository, OpBody, OutboxRepository};

fn com_sync() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let arquivo = dir.path().join("mos.db");
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
    let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    (dir, storage)
}

fn campos(op: &mos_sync::Op) -> serde_json::Map<String, serde_json::Value> {
    match &op.body {
        OpBody::Create { fields } | OpBody::Update { fields } => fields.clone(),
        outro => panic!("esperava campos, veio {outro:?}"),
    }
}

#[test]
fn criar_project_e_task_enfileira_as_duas() {
    let (_dir, storage) = com_sync();
    let project = storage
        .create_project(NewProject::create("043 - Rancho Queimado", "", "").unwrap())
        .unwrap();
    let task = storage
        .create_task(NewTask::create("Revisar memorial", "", Some(project.id)).unwrap())
        .unwrap();

    let ops = storage.pendentes(10).unwrap();
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].entity.kind.as_str(), "project");
    assert_eq!(ops[0].entity.id, project.id.as_uuid());
    assert_eq!(ops[1].entity.kind.as_str(), "task");
    assert_eq!(ops[1].entity.id, task.id.as_uuid());
    assert_eq!(
        campos(&ops[1])["projectId"],
        serde_json::json!(project.id.to_string()),
        "a Task precisa viajar sabendo a que Project pertence"
    );
}

#[test]
fn mover_no_kanban_emite_so_o_estado() {
    // O gesto mais repetido do M/OS, e o que mais vai acontecer nos dois
    // dispositivos ao mesmo tempo. Campo proprio: mover no celular e renomear
    // no PC precisam conviver.
    let (_dir, storage) = com_sync();
    let task = storage
        .create_task(NewTask::create("mover", "", None).unwrap())
        .unwrap();
    storage.set_task_state(task.id, TaskState::Doing).unwrap();

    let ops = storage.pendentes(10).unwrap();
    let ultima = campos(ops.last().unwrap());
    assert_eq!(ultima.len(), 1, "so o campo que mudou");
    assert_eq!(ultima["workState"], serde_json::json!("doing"));
}

#[test]
fn task_criada_a_partir_de_capture_tambem_emite() {
    // Os tres caminhos de criacao passam pelo mesmo `insert_task`, e emitir la
    // dentro e o que garante que nenhum caminho novo nasca sem rastro.
    let (_dir, storage) = com_sync();
    let capture = storage
        .create(NewCapture::create("vira task", CaptureSource::Home).unwrap())
        .unwrap();
    let task = storage
        .create_task_from_capture(capture.id, NewTask::create("derivada", "", None).unwrap())
        .unwrap();

    let ops = storage.pendentes(20).unwrap();
    let da_task = ops
        .iter()
        .find(|op| op.entity.id == task.id.as_uuid())
        .expect("a Task derivada precisa ter emitido");
    assert_eq!(
        campos(da_task)["sourceCaptureId"],
        serde_json::json!(capture.id.to_string()),
        "a origem precisa viajar junto"
    );
}

#[test]
fn arquivar_e_campo_nos_dois_tipos() {
    // Se arquivar virasse `Delete`, a regra de "apagar ganha" faria o
    // arquivamento vencer a restauracao para sempre.
    let (_dir, storage) = com_sync();
    let project = storage
        .create_project(NewProject::create("arquiva", "", "").unwrap())
        .unwrap();
    let task = storage
        .create_task(NewTask::create("arquiva", "", None).unwrap())
        .unwrap();
    storage
        .set_project_lifecycle(project.id, LifecycleState::Archived)
        .unwrap();
    storage
        .set_task_lifecycle(task.id, LifecycleState::Archived)
        .unwrap();

    let ops = storage.pendentes(20).unwrap();
    let arquivamentos: Vec<_> = ops
        .iter()
        .filter(|op| matches!(&op.body, OpBody::Update { fields } if fields.contains_key("lifecycleState")))
        .collect();
    assert_eq!(arquivamentos.len(), 2);
    assert!(
        !ops.iter().any(|op| matches!(op.body, OpBody::Delete)),
        "arquivar nunca pode virar Delete"
    );
}

#[test]
fn excluir_de_vez_emite_delete_nos_dois_tipos() {
    let (_dir, storage) = com_sync();
    let project = storage
        .create_project(NewProject::create("some", "", "").unwrap())
        .unwrap();
    let task = storage
        .create_task(NewTask::create("some", "", None).unwrap())
        .unwrap();
    storage
        .set_project_lifecycle(project.id, LifecycleState::Archived)
        .unwrap();
    storage
        .set_task_lifecycle(task.id, LifecycleState::Archived)
        .unwrap();
    storage.delete_task(task.id).unwrap();
    storage.delete_project(project.id).unwrap();

    let ops = storage.pendentes(30).unwrap();
    let deletes: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op.body, OpBody::Delete))
        .collect();
    assert_eq!(deletes.len(), 2);
    assert!(deletes.iter().any(|op| op.entity.kind.as_str() == "task"));
    assert!(deletes
        .iter()
        .any(|op| op.entity.kind.as_str() == "project"));
}

#[test]
fn a_operacao_cai_junto_com_a_mutacao_recusada() {
    // Excluir uma Task ATIVA e recusado: arquivar primeiro e a regra. A
    // operacao nao pode ficar na fila, senao o outro dispositivo apagaria algo
    // que aqui continua existindo.
    let (_dir, storage) = com_sync();
    let task = storage
        .create_task(NewTask::create("ativa", "", None).unwrap())
        .unwrap();
    let antes = storage.pendentes(30).unwrap().len();

    assert!(
        storage.delete_task(task.id).is_err(),
        "o dominio precisa recusar excluir o que esta ativo"
    );

    let depois = storage.pendentes(30).unwrap();
    assert_eq!(depois.len(), antes, "nada pode ter entrado na fila");
    assert!(!depois.iter().any(|op| matches!(op.body, OpBody::Delete)));
    assert!(storage.get_task(task.id).is_ok());
}

#[test]
fn renomear_emite_so_os_campos_do_formulario() {
    let (_dir, storage) = com_sync();
    let task = storage
        .create_task(NewTask::create("antigo", "", None).unwrap())
        .unwrap();
    storage
        .update_task(
            task.id,
            mos_core::EditTask {
                title: "novo titulo".into(),
                description: "com descricao".into(),
                ..mos_core::EditTask::from_task(&task)
            },
        )
        .unwrap();

    let ultima = campos(storage.pendentes(10).unwrap().last().unwrap());
    assert_eq!(ultima["title"], serde_json::json!("novo titulo"));
    assert_eq!(ultima["description"], serde_json::json!("com descricao"));
    assert!(
        !ultima.contains_key("workState"),
        "renomear nao pode mexer na coluna do Kanban"
    );
}

/// **So o campo que mudou viaja.**
///
/// Um campo emitido e um campo DISPUTADO: quem manda `dueAt` numa operacao
/// esta dizendo ao outro aparelho qual e o prazo agora. Emitir os dez campos a
/// cada edicao faria mudar a prioridade no celular carregar junto o `dueAt`
/// que ele leu antes — e, sendo a operacao mais recente, apagaria o prazo que
/// o PC acabou de por. Dois gestos em campos diferentes, e um vence o outro.
///
/// A escrita continua autoritativa; a EMISSAO e que e um diff.
#[test]
fn editar_emite_so_o_que_mudou() {
    let (_dir, storage) = com_sync();
    let task = storage
        .create_task(NewTask::create("Ajustes reuniao", "contexto", None).unwrap())
        .unwrap();

    let prazo = time::OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap();
    storage
        .update_task(
            task.id,
            mos_core::EditTask {
                due_at: Some(prazo),
                ..mos_core::EditTask::from_task(&task)
            },
        )
        .unwrap();

    let ultima = campos(storage.pendentes(50).unwrap().last().unwrap());
    assert_eq!(
        ultima.keys().collect::<Vec<_>>(),
        ["dueAt"],
        "so o prazo mudou, entao so o prazo pode viajar: {ultima:?}"
    );

    // Uma edicao que nao muda nada nao enfileira operacao nenhuma.
    let antes = storage.pendentes(50).unwrap().len();
    let atual = storage.get_task(task.id).unwrap();
    storage
        .update_task(task.id, mos_core::EditTask::from_task(&atual))
        .unwrap();
    assert_eq!(
        storage.pendentes(50).unwrap().len(),
        antes,
        "salvar sem mexer em nada nao e uma mudanca"
    );
}

/// O item de checklist emite como ENTIDADE, e nao como campo da Task.
#[test]
fn o_checklist_emite_por_item() {
    let (_dir, storage) = com_sync();
    let task = storage
        .create_task(
            NewTask::create("Revisar projeto", "", None)
                .unwrap()
                .with_checklist(&["Conferir niveis".into(), "Gerar PDF".into()]),
        )
        .unwrap();

    let fila = storage.pendentes(50).unwrap();
    let itens: Vec<&mos_sync::Op> = fila
        .iter()
        .filter(|op| op.entity.kind.as_str() == "task_checklist_item")
        .collect();
    assert_eq!(itens.len(), 2, "um `Create` por passo");
    assert_eq!(
        campos(itens[0])["label"],
        serde_json::json!("Conferir niveis")
    );
    assert_eq!(
        campos(itens[0])["taskId"],
        serde_json::json!(task.id.to_string())
    );

    // Marcar mexe num campo so, do ITEM. A Task nao emite nada — o progresso
    // dela e derivado, e sincronizar um numero derivado seria uma segunda
    // verdade sobre a mesma coisa.
    let item = storage.task_detail(task.id).unwrap().checklist[0].id;
    let antes = storage.pendentes(50).unwrap().len();
    storage.set_checklist_item_done(item, true).unwrap();
    let depois = storage.pendentes(50).unwrap();
    assert_eq!(depois.len(), antes + 1, "uma operacao, e nao duas");
    let ultima = depois.last().unwrap();
    assert_eq!(ultima.entity.kind.as_str(), "task_checklist_item");
    assert_eq!(
        campos(ultima).keys().collect::<Vec<_>>(),
        ["completedAt"],
        "marcar e um campo so"
    );
}
