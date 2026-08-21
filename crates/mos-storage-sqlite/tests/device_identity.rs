//! A identidade do dispositivo, contra um banco de verdade.
//!
//! Teste de integracao e nao unitario porque o que pode dar errado aqui e o
//! SQL: o indice parcial que garante um unico "este dispositivo", a
//! idempotencia da abertura e a sobrevivencia do id entre execucoes. Nada disso
//! aparece num teste que troca o banco por um mapa em memoria.

use mos_sync::{DeviceRepository, Platform};
use mos_storage_sqlite::SqliteStorage;

fn banco() -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let arquivo = dir.path().join("mos.db");
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
    (dir, storage)
}

#[test]
fn a_primeira_abertura_cria_o_dispositivo() {
    let (_dir, storage) = banco();
    let device = storage
        .este_dispositivo("PC Principal", "windows", "0.3.0")
        .unwrap();

    assert_eq!(device.name, "PC Principal");
    assert_eq!(device.platform, Platform::Windows);
    assert!(device.is_this_device);
    assert_eq!(device.last_sync_at, "", "nunca sincronizou ainda");
}

#[test]
fn abrir_de_novo_nao_cria_outro_dispositivo() {
    // O app abre todo dia. Se cada abertura criasse um dispositivo, a lista do
    // usuario encheria de fantasmas — e, pior, o desempate do relogio logico
    // mudaria de resposta entre execucoes e a ordem total deixaria de ser
    // estavel.
    let (_dir, storage) = banco();
    let primeiro = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    let segundo = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();

    assert_eq!(primeiro.id, segundo.id);
    assert_eq!(storage.listar().unwrap().len(), 1);
}

#[test]
fn renomear_a_maquina_preserva_o_id() {
    // O nome e da pessoa e a versao do app muda a cada release. O id nao pode
    // mudar junto: ele e o que amarra tudo que este dispositivo ja escreveu.
    let (_dir, storage) = banco();
    let antes = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    let depois = storage
        .este_dispositivo("Estacao do Escritorio", "windows", "0.4.0")
        .unwrap();

    assert_eq!(antes.id, depois.id);
    assert_eq!(depois.name, "Estacao do Escritorio");
    assert_eq!(depois.app_version, "0.4.0");
}

#[test]
fn marcar_sync_guarda_o_instante() {
    let (_dir, storage) = banco();
    let device = storage.este_dispositivo("PC", "windows", "0.3.0").unwrap();
    storage
        .marcar_sync(device.id, "2026-08-21T03:00:00Z")
        .unwrap();

    let lido = &storage.listar().unwrap()[0];
    assert_eq!(lido.last_sync_at, "2026-08-21T03:00:00Z");
}

#[test]
fn plataforma_desconhecida_sobrevive_a_ida_e_volta() {
    // Um cliente futuro precisa aparecer na lista com o proprio nome. Se o
    // banco normalizasse para "outra", a tela de dispositivos passaria a mentir
    // sobre o que esta conectado.
    let (_dir, storage) = banco();
    let device = storage
        .este_dispositivo("Oculos", "visionos", "0.1.0")
        .unwrap();
    assert_eq!(device.platform.as_str(), "visionos");
    assert_eq!(storage.listar().unwrap()[0].platform.as_str(), "visionos");
}
