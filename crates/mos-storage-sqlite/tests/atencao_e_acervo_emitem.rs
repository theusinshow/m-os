//! Reminders e Resources emitindo operacoes.
//!
//! O que e proprio destas duas entidades, e nao aparece em Task nem Capture:
//!
//! - **Lembrete tem intencao e tem entrega**, e so a intencao viaja.
//! - **Resource tem metadado e tem arquivo**, e por enquanto so o metadado
//!   existe — o binario e uma camada separada que ainda nao foi construida.

use mos_core::{
    AttentionRepository, Clock, LifecycleState, NewReminder, NewResource, ResourceKind,
    ResourceRepository, SystemClock,
};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::{DeviceRepository, OpBody, OutboxRepository};
use time::OffsetDateTime;

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

fn lembrete() -> NewReminder {
    let relogio = SystemClock;
    NewReminder::at(
        "Revisar memorial",
        "",
        relogio.now() + time::Duration::hours(2),
        &relogio,
    )
    .unwrap()
}

#[test]
fn criar_lembrete_emite_a_intencao() {
    let (_dir, storage) = com_sync();
    let criado = storage.create_reminder(lembrete()).unwrap();

    let ops = storage.pendentes(10).unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].entity.kind.as_str(), "reminder");
    assert_eq!(ops[0].entity.id, criado.id.as_uuid());
    let f = campos(&ops[0]);
    assert_eq!(f["title"], serde_json::json!("Revisar memorial"));
    assert!(f.contains_key("trigger"), "quando disparar precisa viajar");
}

#[test]
fn a_entrega_nao_viaja_junto_com_a_intencao() {
    // A distincao central deste arquivo. `deliveredCount` conta quantas vezes
    // ESTE dispositivo mostrou o aviso — o iPhone tocar nao significa que o PC
    // tocou. Sincronizar esse numero faria dois aparelhos disputarem um
    // contador que nem descreve a mesma coisa.
    let (_dir, storage) = com_sync();
    let mut criado = storage.create_reminder(lembrete()).unwrap();
    criado.delivered_count += 1;
    criado.snooze_count += 1;
    criado.updated_at = OffsetDateTime::now_utc();
    storage.save_reminder(&criado).unwrap();

    let ultima = campos(storage.pendentes(10).unwrap().last().unwrap());
    assert!(
        !ultima.contains_key("deliveredCount"),
        "a entrega e local e nao pode viajar"
    );
    assert_eq!(
        ultima["snoozeCount"],
        serde_json::json!(1),
        "adiar e acao da pessoa, e viaja"
    );
}

#[test]
fn arquivar_lembrete_e_campo() {
    let (_dir, storage) = com_sync();
    let criado = storage.create_reminder(lembrete()).unwrap();
    storage
        .set_reminder_lifecycle(criado.id, LifecycleState::Archived)
        .unwrap();

    let ops = storage.pendentes(10).unwrap();
    assert!(!ops.iter().any(|op| matches!(op.body, OpBody::Delete)));
    assert_eq!(
        campos(ops.last().unwrap())["lifecycleState"],
        serde_json::json!("archived")
    );
}

#[test]
fn criar_resource_emite_o_metadado() {
    // O METADADO viaja. O arquivo em si e uma camada separada, com upload,
    // download, cache e checksum proprios — e ela ainda nao existe. Ver
    // `docs/SYNC.md`.
    let (_dir, storage) = com_sync();
    let recurso = storage
        .create_resource(
            NewResource::create(
                ResourceKind::Site,
                "Allplan",
                "https://allplan.com",
                "",
                None,
            )
            .unwrap(),
        )
        .unwrap();

    let ops = storage.pendentes(10).unwrap();
    assert_eq!(ops[0].entity.kind.as_str(), "resource");
    assert_eq!(ops[0].entity.id, recurso.id.as_uuid());
    let f = campos(&ops[0]);
    assert_eq!(f["title"], serde_json::json!("Allplan"));
    assert_eq!(f["url"], serde_json::json!("https://allplan.com"));
}

#[test]
fn resource_derivado_de_capture_emite_as_duas_mudancas() {
    // Sao DUAS entidades: o Resource nasce e a Capture muda de estado. Do outro
    // lado elas se reconciliam separadas, e por isso viajam separadas.
    use mos_core::{CaptureRepository, CaptureSource, NewCapture};

    let (_dir, storage) = com_sync();
    let capture = storage
        .create(NewCapture::create("https://allplan.com", CaptureSource::Home).unwrap())
        .unwrap();
    let nova = NewResource::create(
        ResourceKind::Site,
        "Allplan",
        "https://allplan.com",
        "",
        Some(capture.id),
    )
    .unwrap();
    storage.create_resource(nova).unwrap();

    let ops = storage.pendentes(20).unwrap();
    assert!(
        ops.iter().any(|op| op.entity.kind.as_str() == "resource"),
        "o Resource precisa emitir"
    );
    let da_capture: Vec<_> = ops
        .iter()
        .filter(|op| op.entity.id == capture.id.as_uuid())
        .collect();
    assert!(
        da_capture.iter().any(|op| matches!(
            &op.body,
            OpBody::Update { fields } if fields.get("processingState") == Some(&serde_json::json!("processed"))
        )),
        "a Capture precisa emitir a propria mudanca de estado"
    );
}

#[test]
fn excluir_resource_de_vez_emite_delete() {
    let (_dir, storage) = com_sync();
    let recurso = storage
        .create_resource(
            NewResource::create(ResourceKind::Site, "some", "https://x.com", "", None).unwrap(),
        )
        .unwrap();
    storage
        .set_resource_lifecycle(recurso.id, LifecycleState::Archived)
        .unwrap();
    storage.delete_resource(recurso.id).unwrap();

    let ops = storage.pendentes(20).unwrap();
    assert!(matches!(ops.last().unwrap().body, OpBody::Delete));
}
