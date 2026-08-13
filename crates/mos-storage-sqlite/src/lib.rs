mod backup;
mod repository;
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

const SCHEMA_VERSION: u32 = 2;
const MIGRATION_001: &str = include_str!("../migrations/0001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/0002_work.sql");

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
    use mos_core::CaptureRepository;

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
        assert_eq!(health.schema_version, 2);
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
        assert_eq!(storage.health().unwrap().schema_version, 2);
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
}
