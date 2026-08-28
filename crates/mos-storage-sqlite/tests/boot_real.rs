//! Reproduz as leituras do boot contra uma CÓPIA do banco real.
//!
//! Existe para responder uma pergunta que a tela não responde: quando o app diz
//! "não abriu os dados locais com segurança", QUAL das dezesseis leituras
//! paralelas falhou. O `Promise.all` do renderer perde essa informação, e o
//! `<details>` da tela não é legível de fora do app.
//!
//! Ignorado por padrão, porque depende do banco desta máquina. Rode com:
//! `cargo test -p mos-storage-sqlite --test boot_real -- --ignored --nocapture`
use mos_core::{
    AppRepository, CaptureRepository, IngestionRepository, ResourceRepository, WorkRepository,
};
use mos_storage_sqlite::SqliteStorage;

#[test]
#[ignore = "depende do banco real desta maquina"]
fn qual_leitura_do_boot_falha() {
    let appdata = std::env::var("APPDATA").expect("APPDATA");
    let origem = std::path::PathBuf::from(&appdata).join("com.codedbym.mos");
    let dir = tempfile::tempdir().unwrap();

    // Cópia, e nunca o original: uma leitura que abre em modo de escrita pode
    // rodar migração, e este teste não pode ser o motivo de um banco mudar.
    for sufixo in ["", "-wal", "-shm"] {
        let de = origem.join(format!("m-os.db{sufixo}"));
        if de.exists() {
            std::fs::copy(&de, dir.path().join(format!("m-os.db{sufixo}"))).unwrap();
        }
    }

    let storage = match SqliteStorage::open(dir.path().join("m-os.db"), dir.path().join("backups"))
    {
        Ok(storage) => {
            println!("ABRIR                        ok");
            storage
        }
        Err(erro) => {
            println!("ABRIR                        FALHOU -> {erro:?}");
            panic!("o banco nem abriu");
        }
    };

    macro_rules! tenta {
        ($nome:expr, $expr:expr) => {
            match $expr {
                Ok(itens) => println!("{:28} ok ({} itens)", $nome, itens.len()),
                Err(erro) => println!("{:28} FALHOU -> {:?}", $nome, erro),
            }
        };
    }

    tenta!("recent", storage.recent(50));
    tenta!("inbox", storage.inbox(200));
    tenta!("projects", storage.projects(true));
    tenta!("tasks", storage.tasks(true));
    tenta!("workspaces", storage.workspaces(true));
    tenta!("apps", storage.apps(true));
    tenta!("resources", storage.resources(true));
    tenta!("trashed_resources", storage.trashed_resources());
    tenta!("file_ingestions", storage.file_ingestions());
    tenta!(
        "archived",
        storage.by_lifecycle(mos_core::LifecycleState::Archived, 200)
    );
    tenta!(
        "trashed",
        storage.by_lifecycle(mos_core::LifecycleState::Trashed, 200)
    );
    tenta!("hidden_widgets", storage.hidden_widgets());
    tenta!("widget_placements", storage.widget_placements());
    tenta!("radial_pins", storage.radial_pins());
    tenta!("resource_workspaces", storage.resource_workspaces());

    // O que o `get_app_status` faz alem do que ja foi testado acima.
    tenta!("projects(false)", storage.projects(false));
    tenta!("tasks(false)", storage.tasks(false));
    tenta!("apps(false)", storage.apps(false));
    tenta!("resources(false)", storage.resources(false));
    tenta!("workspaces(false)", storage.workspaces(false));
    match storage.health() {
        Ok(saude) => println!("{:28} ok ({:?})", "health", saude),
        Err(erro) => println!("{:28} FALHOU -> {:?}", "health", erro),
    }
}
