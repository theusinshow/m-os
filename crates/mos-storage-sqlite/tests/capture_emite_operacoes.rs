//! A Capture emitindo operacoes junto com as proprias mutacoes.
//!
//! É a primeira entidade a ser ligada, e o padrao que as outras onze vao
//! seguir. O que precisa ser provado nao e "emitiu": e **emitiu junto**, e
//! **nao emitiu quando esta desligado**.

use mos_core::{CaptureRepository, CaptureSource, LifecycleState, NewCapture, ProcessingState};
use mos_sync::{DeviceRepository, Op, OpBody, OutboxRepository};
use mos_storage_sqlite::SqliteStorage;

fn banco() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let arquivo = dir.path().join("mos.db");
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
    (dir, storage)
}

fn com_sync() -> (tempfile::TempDir, SqliteStorage) {
    let (dir, storage) = banco();
    let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    (dir, storage)
}

fn nova(texto: &str) -> NewCapture {
    NewCapture::create(texto, CaptureSource::Home).unwrap()
}

#[test]
fn sem_habilitar_o_sync_nada_e_emitido() {
    // O M/OS funciona inteiro sem sincronizacao, e e isso que permite ligar a
    // emissao por entidade, uma de cada vez, sem parar o desktop. Uma Capture
    // criada aqui nao pode falhar nem deixar lixo na fila.
    let (_dir, storage) = banco();
    assert!(!storage.sync_ligado());

    storage.create(nova("sem sync")).unwrap();
    assert_eq!(storage.quantidade_pendente().unwrap(), 0);
}

#[test]
fn criar_uma_capture_enfileira_a_operacao() {
    let (_dir, storage) = com_sync();
    let capture = storage.create(nova("primeira ideia")).unwrap();

    let pendentes = storage.pendentes(10).unwrap();
    assert_eq!(pendentes.len(), 1);
    let op: &Op = &pendentes[0];
    assert_eq!(op.entity.kind.as_str(), "capture");
    assert_eq!(op.entity.id, capture.id.as_uuid());
    match &op.body {
        OpBody::Create { fields } => {
            assert_eq!(fields["content"], serde_json::json!("primeira ideia"));
        }
        outro => panic!("criar precisa emitir Create, veio {outro:?}"),
    }
}

#[test]
fn mudar_o_estado_emite_so_o_campo_que_mudou() {
    // O que viaja e a MUDANCA DE CAMPO. Mandar a Capture inteira faria a
    // reconciliacao ter que escolher uma versao e perder a outra.
    let (_dir, storage) = com_sync();
    let capture = storage.create(nova("vira task")).unwrap();
    storage
        .set_processing_state(capture.id, ProcessingState::Processed)
        .unwrap();

    let pendentes = storage.pendentes(10).unwrap();
    assert_eq!(pendentes.len(), 2, "criacao e mudanca sao duas operacoes");
    match &pendentes[1].body {
        OpBody::Update { fields } => {
            assert_eq!(fields.len(), 1, "so o campo que mudou");
            assert_eq!(fields["processingState"], serde_json::json!("processed"));
        }
        outro => panic!("mudar estado precisa emitir Update, veio {outro:?}"),
    }
}

#[test]
fn arquivar_e_campo_e_nao_apagamento() {
    // Arquivar num dispositivo e restaurar no outro precisa ser decidido pelo
    // INSTANTE, como qualquer campo. Se arquivar virasse `OpBody::Delete`, a
    // regra de "apagar ganha" faria o arquivamento vencer a restauracao para
    // sempre — e a Capture nunca mais voltaria.
    let (_dir, storage) = com_sync();
    let capture = storage.create(nova("arquiva")).unwrap();
    storage
        .set_lifecycle_state(capture.id, LifecycleState::Archived)
        .unwrap();

    let pendentes = storage.pendentes(10).unwrap();
    match &pendentes[1].body {
        OpBody::Update { fields } => {
            assert_eq!(fields["lifecycleState"], serde_json::json!("archived"));
        }
        outro => panic!("arquivar e mudanca de campo, veio {outro:?}"),
    }
}

#[test]
fn excluir_de_vez_emite_delete() {
    // Aqui sim: a exclusao definitiva, a que o M/OS so aceita depois de
    // arquivar. O outro dispositivo precisa SABER que sumiu.
    let (_dir, storage) = com_sync();
    let capture = storage.create(nova("some de vez")).unwrap();
    storage
        .set_lifecycle_state(capture.id, LifecycleState::Archived)
        .unwrap();
    storage.delete_capture(capture.id).unwrap();

    let pendentes = storage.pendentes(10).unwrap();
    assert!(
        matches!(pendentes.last().unwrap().body, OpBody::Delete),
        "a ultima operacao precisa ser Delete"
    );
}

#[test]
fn a_operacao_cai_junto_com_a_mutacao_recusada() {
    // A garantia central deste arquivo. Excluir uma Capture que virou Task e
    // recusado pelo dominio — e a operacao NAO pode ficar na fila, senao o
    // outro dispositivo apagaria algo que aqui continua existindo.
    use mos_core::{NewTask, WorkRepository};

    let (_dir, storage) = com_sync();
    let capture = storage.create(nova("vira task")).unwrap();
    storage
        .create_task_from_capture(capture.id, NewTask::create("derivada", "", None).unwrap())
        .unwrap();
    let antes = storage.pendentes(50).unwrap().len();

    let recusa = storage.delete_capture(capture.id);
    assert!(recusa.is_err(), "o dominio precisa recusar");

    let depois = storage.pendentes(50).unwrap();
    assert!(
        !depois.iter().any(|op| matches!(op.body, OpBody::Delete)),
        "uma exclusao recusada nao pode deixar Delete na fila"
    );
    assert_eq!(
        depois.len(),
        antes,
        "nada pode ter entrado na fila por causa da tentativa"
    );
    // E a Capture continua la, que e o outro lado da mesma moeda.
    assert!(storage.get(capture.id).is_ok());
}

#[test]
fn o_relogio_avanca_e_sobrevive_ao_reinicio() {
    // O instante emitido precisa sobreviver junto com a operacao que o usou.
    // Se a operacao commitar e o relogio nao, reabrir reemitiria aquele
    // instante para outra operacao — e duas operacoes com o mesmo instante e o
    // mesmo dispositivo quebram a ordem total.
    let dir = tempfile::tempdir().unwrap();
    let arquivo = dir.path().join("mos.db");
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();

    let ultimo = {
        let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
        let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
        storage.habilitar_sync(device.id).unwrap();
        storage.create(nova("antes")).unwrap();
        storage.create(nova("depois")).unwrap();
        let ops = storage.pendentes(10).unwrap();
        assert!(ops[1].at > ops[0].at, "o relogio precisa avancar");
        ops[1].at
    };

    let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
    let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    storage.create(nova("depois de reabrir")).unwrap();

    let ops = storage.pendentes(10).unwrap();
    let nova_op = ops.last().unwrap();
    assert!(
        nova_op.at > ultimo,
        "o instante depois de reabrir precisa vir depois do ultimo guardado"
    );
}

#[test]
fn todas_as_operacoes_saem_do_mesmo_dispositivo() {
    let (_dir, storage) = com_sync();
    let este = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    storage.create(nova("a")).unwrap();
    storage.create(nova("b")).unwrap();

    for op in storage.pendentes(10).unwrap() {
        assert_eq!(op.at.device, este.id);
    }
}
