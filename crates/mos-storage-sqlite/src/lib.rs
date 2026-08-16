mod app_repository;
mod backup;
mod conversation_repository;
mod cronocad_import;
mod monitoring_repository;
mod repository;
mod resource_repository;
mod tracking_repository;
mod work_repository;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use mos_core::{CoreError, ErrorCode};
use rusqlite::{Connection, MAIN_DB};
use serde::Serialize;

pub use cronocad_import::ImportReport;

const SCHEMA_VERSION: u32 = 14;
const MIGRATION_001: &str = include_str!("../migrations/0001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/0002_work.sql");
const MIGRATION_003: &str = include_str!("../migrations/0003_apps.sql");
const MIGRATION_004: &str = include_str!("../migrations/0004_workspaces.sql");
const MIGRATION_005: &str = include_str!("../migrations/0005_app_sources.sql");
const MIGRATION_006: &str = include_str!("../migrations/0006_resources.sql");
const MIGRATION_007: &str = include_str!("../migrations/0007_v03_design.sql");
const MIGRATION_008: &str = include_str!("../migrations/0008_workspace_widgets.sql");
const MIGRATION_009: &str = include_str!("../migrations/0009_resource_workspaces.sql");
const MIGRATION_010: &str = include_str!("../migrations/0010_conversations.sql");
const MIGRATION_011: &str = include_str!("../migrations/0011_time_tracking.sql");
const MIGRATION_012: &str = include_str!("../migrations/0012_cronocad_import_mark.sql");
const MIGRATION_013: &str = include_str!("../migrations/0013_tracking_surface.sql");
const MIGRATION_014: &str = include_str!("../migrations/0014_project_budget.sql");

pub struct SqliteStorage {
    connection: Mutex<Connection>,
    backup_lock: Mutex<()>,
    database_path: PathBuf,
    backup_directory: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageHealth {
    pub database_path: String,
    pub schema_version: u32,
    pub journal_mode: String,
    pub synchronous: String,
    pub integrity: String,
}

impl SqliteStorage {
    pub fn open(
        database_path: impl AsRef<Path>,
        backup_directory: impl AsRef<Path>,
    ) -> Result<Self, CoreError> {
        let database_path = database_path.as_ref().to_path_buf();
        let backup_directory = backup_directory.as_ref().to_path_buf();
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(map_io_error)?;
        }
        fs::create_dir_all(&backup_directory).map_err(map_io_error)?;

        let connection = Connection::open(&database_path).map_err(map_sql_error)?;
        configure_connection(&connection)?;
        verify_integrity(&connection)?;
        migrate(&connection, &backup_directory)?;
        ensure_search_projection(&connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
            backup_lock: Mutex::new(()),
            database_path,
            backup_directory,
        })
    }

    pub fn health(&self) -> Result<StorageHealth, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(map_sql_error)?;
        let synchronous: i64 = connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(map_sql_error)?;
        let schema_version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(map_sql_error)?;
        let integrity: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(map_sql_error)?;

        Ok(StorageHealth {
            database_path: self.database_path.display().to_string(),
            schema_version,
            journal_mode,
            synchronous: synchronous_name(synchronous).into(),
            integrity,
        })
    }
}

fn configure_connection(connection: &Connection) -> Result<(), CoreError> {
    connection
        .busy_timeout(Duration::from_millis(1_000))
        .map_err(map_sql_error)?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             PRAGMA trusted_schema=OFF;",
        )
        .map_err(map_sql_error)?;

    let journal: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    let synchronous: i64 = connection
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(map_sql_error)?;

    if !journal.eq_ignore_ascii_case("wal") || synchronous != 2 || foreign_keys != 1 {
        return Err(CoreError::new(
            ErrorCode::StorageUnavailable,
            "O banco local nao abriu com as garantias de durabilidade exigidas.",
            false,
        ));
    }
    Ok(())
}

fn verify_integrity(connection: &Connection) -> Result<(), CoreError> {
    let result: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    if result != "ok" {
        return Err(CoreError::new(
            ErrorCode::DataIntegrity,
            format!("O banco local falhou na verificacao de integridade: {result}"),
            false,
        ));
    }
    Ok(())
}

fn migrate(connection: &Connection, backup_directory: &Path) -> Result<(), CoreError> {
    let current: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    if current > SCHEMA_VERSION {
        return Err(CoreError::new(
            ErrorCode::StorageUnavailable,
            "O banco local foi criado por uma versao mais nova do M/OS.",
            false,
        ));
    }
    if current > 0 && current < SCHEMA_VERSION {
        create_pre_migration_snapshot(connection, backup_directory, current)?;
    }
    if current == 0 {
        connection
            .execute_batch(MIGRATION_001)
            .map_err(map_sql_error)?;
    }
    if current <= 1 {
        connection
            .execute_batch(MIGRATION_002)
            .map_err(map_sql_error)?;
    }
    if current <= 2 {
        connection
            .execute_batch(MIGRATION_003)
            .map_err(map_sql_error)?;
    }
    if current <= 3 {
        connection
            .execute_batch(MIGRATION_004)
            .map_err(map_sql_error)?;
    }
    if current <= 4 {
        connection
            .execute_batch(MIGRATION_005)
            .map_err(map_sql_error)?;
    }
    if current <= 5 {
        connection
            .execute_batch(MIGRATION_006)
            .map_err(map_sql_error)?;
    }
    if current <= 6 {
        connection
            .execute_batch(MIGRATION_007)
            .map_err(map_sql_error)?;
    }
    if current <= 7 {
        connection
            .execute_batch(MIGRATION_008)
            .map_err(map_sql_error)?;
    }
    if current <= 8 {
        connection
            .execute_batch(MIGRATION_009)
            .map_err(map_sql_error)?;
    }
    if current <= 9 {
        connection
            .execute_batch(MIGRATION_010)
            .map_err(map_sql_error)?;
    }
    if current <= 10 {
        connection
            .execute_batch(MIGRATION_011)
            .map_err(map_sql_error)?;
    }
    if current <= 11 {
        connection
            .execute_batch(MIGRATION_012)
            .map_err(map_sql_error)?;
    }
    if current <= 12 {
        connection
            .execute_batch(MIGRATION_013)
            .map_err(map_sql_error)?;
    }
    if current <= 13 {
        connection
            .execute_batch(MIGRATION_014)
            .map_err(map_sql_error)?;
    }
    Ok(())
}

fn create_pre_migration_snapshot(
    connection: &Connection,
    backup_directory: &Path,
    version: u32,
) -> Result<(), CoreError> {
    let destination = backup_directory.join(format!(
        "pre-migration-v{version}-{}.db",
        time::OffsetDateTime::now_utc().unix_timestamp()
    ));
    connection
        .backup(MAIN_DB, &destination, None)
        .map_err(map_sql_error)?;
    let snapshot = Connection::open(&destination).map_err(map_sql_error)?;
    verify_integrity(&snapshot)
}

fn ensure_search_projection(connection: &Connection) -> Result<(), CoreError> {
    for (source, projection) in [
        ("captures", "capture_search"),
        ("projects", "project_search"),
        ("tasks", "task_search"),
        ("apps", "app_search"),
        ("workspaces", "workspace_search"),
        ("resources", "resource_search"),
        ("message_parts", "message_search"),
    ] {
        let source_count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {source}"), [], |row| {
                row.get(0)
            })
            .map_err(map_sql_error)?;
        let search_count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {projection}"), [], |row| {
                row.get(0)
            })
            .map_err(map_sql_error)?;
        if source_count != search_count {
            connection
                .execute(
                    &format!("INSERT INTO {projection}({projection}) VALUES('rebuild')"),
                    [],
                )
                .map_err(map_sql_error)?;
        }
    }
    Ok(())
}

fn rebuild_search_projection(connection: &Connection) -> Result<usize, CoreError> {
    let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
    transaction
        .execute(
            "INSERT INTO capture_search(capture_search) VALUES('rebuild')",
            [],
        )
        .map_err(map_sql_error)?;
    let count = transaction
        .query_row("SELECT count(*) FROM capture_search", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(map_sql_error)? as usize;
    transaction.commit().map_err(map_sql_error)?;
    Ok(count)
}

fn synchronous_name(value: i64) -> &'static str {
    match value {
        0 => "OFF",
        1 => "NORMAL",
        2 => "FULL",
        3 => "EXTRA",
        _ => "UNKNOWN",
    }
}

fn map_sql_error(error: rusqlite::Error) -> CoreError {
    use rusqlite::{Error, ErrorCode as SqlErrorCode};

    let (code, retryable, message) = match &error {
        Error::SqliteFailure(details, _)
            if matches!(
                details.code,
                SqlErrorCode::DatabaseBusy | SqlErrorCode::DatabaseLocked
            ) =>
        {
            (
                ErrorCode::StorageBusy,
                true,
                "O banco local esta ocupado. Nada foi salvo; tente novamente.".to_owned(),
            )
        }
        Error::SqliteFailure(details, _) if details.code == SqlErrorCode::DiskFull => (
            ErrorCode::StorageUnavailable,
            false,
            "O disco esta sem espaco. Nada foi salvo.".to_owned(),
        ),
        Error::SqliteFailure(details, _) if details.code == SqlErrorCode::DatabaseCorrupt => (
            ErrorCode::DataIntegrity,
            false,
            "O banco local parece corrompido. Escritas foram bloqueadas.".to_owned(),
        ),
        _ => (
            ErrorCode::StorageUnavailable,
            false,
            format!("Falha no armazenamento local: {error}"),
        ),
    };
    CoreError::new(code, message, retryable)
}

fn map_io_error(error: std::io::Error) -> CoreError {
    CoreError::new(
        ErrorCode::Io,
        format!("Falha de arquivo local: {error}"),
        false,
    )
}

fn map_lock_error<T>(error: std::sync::PoisonError<T>) -> CoreError {
    CoreError::new(
        ErrorCode::StorageUnavailable,
        format!("Acesso ao banco local foi interrompido: {error}"),
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{
        AppRepository, CaptureRepository, NewResource, ResourceRepository, SearchRequest,
    };

    #[test]
    fn opens_with_required_pragmas_and_schema() {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        let health = storage.health().unwrap();

        assert_eq!(health.journal_mode.to_lowercase(), "wal");
        assert_eq!(health.synchronous, "FULL");
        assert_eq!(health.schema_version, SCHEMA_VERSION);
        assert_eq!(health.integrity, "ok");
    }

    #[test]
    fn upgrades_v1_after_creating_a_consistent_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();
        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(MIGRATION_001).unwrap();
        connection
            .execute(
                "INSERT INTO captures (
                    id, content, source_kind, processing_state, lifecycle_state,
                    captured_at, created_at, updated_at
                 ) VALUES (
                    '0198a7d5-a64e-7000-8000-000000000001', 'Preservada', 'home',
                    'inbox', 'active', '2026-08-13T00:00:00Z',
                    '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        drop(connection);

        let storage = SqliteStorage::open(&database, &backups).unwrap();
        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        assert_eq!(CaptureRepository::recent(&storage, 10).unwrap().len(), 1);
        assert_eq!(
            fs::read_dir(&backups)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("pre-migration-v1-"))
                .count(),
            1
        );
    }

    /// Sobe um v6 povoado ate v7 e confere as tres mudancas do design v0.3.
    ///
    /// O ponto delicado e a busca: tasks e resources sao recriadas, e as tabelas
    /// FTS de conteudo externo indexam por rowid. Depois de um swap as contagens
    /// continuam batendo enquanto os rowids mudam, entao ensure_search_projection()
    /// nao detectaria a divergencia. Se a migration esquecer o rebuild explicito,
    /// a busca passa a devolver resultado errado em silencio — e este teste falha.
    #[test]
    fn upgrades_populated_v6_to_the_v03_design_schema() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();
        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            MIGRATION_001,
            MIGRATION_002,
            MIGRATION_003,
            MIGRATION_004,
            MIGRATION_005,
            MIGRATION_006,
        ] {
            connection.execute_batch(migration).unwrap();
        }

        connection
            .execute(
                "INSERT INTO projects (id, name, description, lifecycle_state, created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000010', 'Minarum', 'Escadas',
                         'active', '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO tasks (id, title, description, work_state, lifecycle_state,
                                    created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000011', 'Refatorar navbar',
                         'com urgencia', 'doing', 'active',
                         '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO resources (id, kind, title, url, note, lifecycle_state,
                                        created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000012', 'link', 'Motion',
                         'https://motion.dev', 'animacao declarativa', 'active',
                         '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO apps (id, name, description, launch_kind, launch_target,
                                   lifecycle_state, created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000013', 'ChronoCAD', 'Cronometro',
                         'url', 'https://chronocad.local', 'active',
                         '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .unwrap();
        drop(connection);

        let storage = SqliteStorage::open(&database, &backups).unwrap();
        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        assert_eq!(storage.health().unwrap().integrity, "ok");

        let connection = Connection::open(&database).unwrap();

        // 1. Estado existente preservado, e os tres estados novos sao aceitos.
        let state: String = connection
            .query_row(
                "SELECT work_state FROM tasks WHERE id = '0198a7d5-a64e-7000-8000-000000000011'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "doing", "estado existente nao pode ser reescrito");

        for new_state in ["inbox", "planned", "review"] {
            connection
                .execute(
                    "UPDATE tasks SET work_state = ?1
                     WHERE id = '0198a7d5-a64e-7000-8000-000000000011'",
                    [new_state],
                )
                .unwrap_or_else(|error| panic!("estado {new_state} deveria ser aceito: {error}"));
        }

        // 2. link virou site, e a busca continua encontrando o recurso.
        let kind: String = connection
            .query_row("SELECT kind FROM resources", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kind, "site");

        let found = storage
            .search_resources(SearchRequest {
                query: "animacao".into(),
                include_archived: false,
                limit: 10,
            })
            .unwrap();
        assert_eq!(found.len(), 1, "o indice FTS de resources ficou obsoleto");
        assert_eq!(found[0].title, "Motion");

        // 3. Capacidades e repositorio.
        let can_open: i64 = connection
            .query_row("SELECT can_open FROM apps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            can_open, 1,
            "app com launch_target ja abre, e deve declarar isso"
        );

        let can_write: i64 = connection
            .query_row("SELECT can_write FROM apps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(can_write, 0);

        let repository: String = connection
            .query_row("SELECT repository FROM projects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(repository, "");
    }

    /// A CHECK de url no banco espelha a validacao do dominio: uma Note nao tem
    /// url, e um Site sem http(s) e recusado nos dois lugares.
    #[test]
    fn resource_url_rules_follow_the_kind() {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        let connection = Connection::open(directory.path().join("mos.db")).unwrap();

        connection
            .execute(
                "INSERT INTO resources (id, kind, title, url, note, lifecycle_state,
                                        created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000020', 'note', 'Ideia solta', '',
                         'o motivo', 'active', '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .expect("note sem url deve ser aceita");

        connection
            .execute(
                "INSERT INTO resources (id, kind, title, url, note, lifecycle_state,
                                        created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000021', 'site', 'Sem esquema',
                         'motion.dev', '', 'active',
                         '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .expect_err("site sem http(s) deve ser recusado pelo banco");

        drop(storage);
    }

    #[test]
    fn upgrades_populated_v4_through_app_sources_and_resources() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();
        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(MIGRATION_001).unwrap();
        connection.execute_batch(MIGRATION_002).unwrap();
        connection.execute_batch(MIGRATION_003).unwrap();
        connection.execute_batch(MIGRATION_004).unwrap();
        connection
            .execute(
                "INSERT INTO apps (
                    id, name, description, launch_kind, launch_target,
                    lifecycle_state, created_at, updated_at
                 ) VALUES (
                    '0198a7d5-a64e-7000-8000-000000000004', 'Motion',
                    'Biblioteca de animacao', 'url', 'https://motion.dev',
                    'active', '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        drop(connection);

        let storage = SqliteStorage::open(&database, &backups).unwrap();

        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        let apps = storage.apps(false).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Motion");
        assert_eq!(apps[0].source_url, None);
        assert_eq!(
            storage
                .search_apps(SearchRequest {
                    query: "animacao".into(),
                    include_archived: false,
                    limit: 10,
                })
                .unwrap()
                .len(),
            1
        );

        let resource = storage
            .create_resource(
                NewResource::create_link(
                    "SQLite FTS5",
                    "https://sqlite.org/fts5.html",
                    "Busca local depois da migration",
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            storage
                .search_resources(SearchRequest {
                    query: "migration".into(),
                    include_archived: false,
                    limit: 10,
                })
                .unwrap()[0]
                .id,
            resource.id
        );
        assert_eq!(
            fs::read_dir(&backups)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("pre-migration-v4-"))
                .count(),
            1
        );
    }
}
