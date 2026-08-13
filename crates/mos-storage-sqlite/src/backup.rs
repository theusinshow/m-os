use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use mos_core::{BackupInspection, BackupReceipt, CoreError, DataMaintenance, ErrorCode};
use rusqlite::{Connection, MAIN_DB};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use time::{format_description::well_known::Rfc3339, macros::format_description, OffsetDateTime};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

use crate::{
    app_repository::query_apps_all,
    configure_connection, ensure_search_projection, map_io_error, map_lock_error, map_sql_error,
    migrate, verify_integrity,
    work_repository::{
        query_captures_all, query_projects, query_tasks, PROJECT_COLUMNS, TASK_COLUMNS,
    },
    SqliteStorage, SCHEMA_VERSION,
};

const MANIFEST_NAME: &str = "manifest.json";
const DATABASE_NAME: &str = "mos.db";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    format: String,
    format_version: u32,
    schema_version: u32,
    created_at: String,
    capture_count: u64,
    database_sha256: String,
    database_bytes: u64,
}

impl DataMaintenance for SqliteStorage {
    fn create_backup(&self, destination: &Path) -> Result<BackupReceipt, CoreError> {
        self.backup_to(destination)
    }

    fn inspect_backup(&self, source: &Path) -> Result<BackupInspection, CoreError> {
        let extracted = extract_and_validate(source)?;
        Ok(BackupInspection {
            path: source.display().to_string(),
            schema_version: extracted.manifest.schema_version,
            capture_count: extracted.manifest.capture_count,
            bytes: fs::metadata(source).map_err(map_io_error)?.len(),
            created_at: parse_time(&extracted.manifest.created_at)?,
        })
    }

    fn restore_backup(&self, source: &Path) -> Result<BackupReceipt, CoreError> {
        let extracted = extract_and_validate(source)?;
        let safety_path = self.backup_directory.join(format!(
            "safety-{}.mos-backup",
            OffsetDateTime::now_utc()
                .format(format_description!(
                    "[year][month][day]-[hour][minute][second]"
                ))
                .map_err(format_error)?
        ));
        let safety_receipt = self.backup_to(&safety_path)?;

        let mut connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .restore(
                MAIN_DB,
                &extracted.database_path,
                None::<fn(rusqlite::backup::Progress)>,
            )
            .map_err(map_sql_error)?;
        configure_connection(&connection)?;
        verify_integrity(&connection)?;
        migrate(&connection, &self.backup_directory)?;
        ensure_search_projection(&connection)?;

        Ok(safety_receipt)
    }

    fn ensure_daily_snapshot(&self) -> Result<Option<BackupReceipt>, CoreError> {
        let today = OffsetDateTime::now_utc()
            .date()
            .to_string()
            .replace('-', "");
        let destination = self
            .backup_directory
            .join(format!("daily-{today}.mos-backup"));
        if destination.exists() {
            return Ok(None);
        }

        let receipt = self.backup_to(&destination)?;
        prune_daily_backups(&self.backup_directory, 7)?;
        Ok(Some(receipt))
    }

    fn export_json(&self, destination: &Path) -> Result<BackupReceipt, CoreError> {
        self.export_to_json(destination)
    }
}

impl SqliteStorage {
    fn backup_to(&self, destination: &Path) -> Result<BackupReceipt, CoreError> {
        let _backup_guard = self.backup_lock.lock().map_err(map_lock_error)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(map_io_error)?;
        }
        let temporary = TempDir::new().map_err(map_io_error)?;
        let snapshot = temporary.path().join(DATABASE_NAME);
        let created_at = OffsetDateTime::now_utc();

        let capture_count = {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            connection
                .backup(MAIN_DB, &snapshot, None)
                .map_err(map_sql_error)?;
            connection
                .query_row("SELECT count(*) FROM captures", [], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(map_sql_error)? as u64
        };

        let database_bytes = fs::metadata(&snapshot).map_err(map_io_error)?.len();
        let database_sha256 = sha256_file(&snapshot)?;
        let manifest = BackupManifest {
            format: "m-os-backup".into(),
            format_version: 1,
            schema_version: SCHEMA_VERSION,
            created_at: created_at.format(&Rfc3339).map_err(format_error)?,
            capture_count,
            database_sha256,
            database_bytes,
        };

        let temporary_archive = destination.with_extension("mos-backup.tmp");
        write_archive(&temporary_archive, &snapshot, &manifest)?;
        if destination.exists() {
            fs::remove_file(destination).map_err(map_io_error)?;
        }
        fs::rename(&temporary_archive, destination).map_err(map_io_error)?;

        Ok(BackupReceipt {
            path: destination.display().to_string(),
            bytes: fs::metadata(destination).map_err(map_io_error)?.len(),
            created_at,
        })
    }

    fn export_to_json(&self, destination: &Path) -> Result<BackupReceipt, CoreError> {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(map_io_error)?;
        }
        let created_at = OffsetDateTime::now_utc();
        let dataset = {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            ExportDataset {
                format: "m-os-export".into(),
                format_version: 1,
                schema_version: SCHEMA_VERSION,
                exported_at: created_at.format(&Rfc3339).map_err(format_error)?,
                captures: query_captures_all(&connection)?,
                projects: query_projects(
                    &connection,
                    &format!("SELECT {PROJECT_COLUMNS} FROM projects ORDER BY created_at ASC"),
                )?,
                tasks: query_tasks(
                    &connection,
                    &format!("SELECT {TASK_COLUMNS} FROM tasks ORDER BY created_at ASC"),
                )?,
                apps: query_apps_all(&connection)?,
            }
        };
        let bytes = serde_json::to_vec_pretty(&dataset)
            .map_err(|error| backup_invalid(error.to_string()))?;
        let temporary = destination.with_extension("json.tmp");
        let mut file = File::create(&temporary).map_err(map_io_error)?;
        file.write_all(&bytes).map_err(map_io_error)?;
        file.sync_all().map_err(map_io_error)?;
        if destination.exists() {
            fs::remove_file(destination).map_err(map_io_error)?;
        }
        fs::rename(&temporary, destination).map_err(map_io_error)?;
        Ok(BackupReceipt {
            path: destination.display().to_string(),
            bytes: bytes.len() as u64,
            created_at,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportDataset {
    format: String,
    format_version: u32,
    schema_version: u32,
    exported_at: String,
    captures: Vec<mos_core::Capture>,
    projects: Vec<mos_core::Project>,
    tasks: Vec<mos_core::Task>,
    apps: Vec<mos_core::RegisteredApp>,
}

struct ExtractedBackup {
    _directory: TempDir,
    database_path: PathBuf,
    manifest: BackupManifest,
}

fn extract_and_validate(source: &Path) -> Result<ExtractedBackup, CoreError> {
    let file = File::open(source).map_err(map_io_error)?;
    let mut archive = ZipArchive::new(file).map_err(backup_zip_error)?;
    let manifest: BackupManifest = {
        let mut entry = archive.by_name(MANIFEST_NAME).map_err(backup_zip_error)?;
        let mut json = String::new();
        entry.read_to_string(&mut json).map_err(backup_read_error)?;
        serde_json::from_str(&json).map_err(|error| backup_invalid(error.to_string()))?
    };

    if manifest.format != "m-os-backup" || manifest.format_version != 1 {
        return Err(backup_invalid("Formato de backup desconhecido."));
    }
    if manifest.schema_version > SCHEMA_VERSION {
        return Err(CoreError::new(
            ErrorCode::UnsupportedBackup,
            "O backup pertence a uma versao mais nova do M/OS.",
            false,
        ));
    }

    let directory = TempDir::new().map_err(map_io_error)?;
    let database_path = directory.path().join(DATABASE_NAME);
    {
        let mut entry = archive.by_name(DATABASE_NAME).map_err(backup_zip_error)?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(backup_read_error)?;
        let mut output = File::create(&database_path).map_err(map_io_error)?;
        output.write_all(&bytes).map_err(map_io_error)?;
        output.sync_all().map_err(map_io_error)?;
    }

    if fs::metadata(&database_path).map_err(map_io_error)?.len() != manifest.database_bytes
        || sha256_file(&database_path)? != manifest.database_sha256
    {
        return Err(backup_invalid("O checksum do backup nao confere."));
    }

    let validation = Connection::open(&database_path).map_err(map_sql_error)?;
    verify_integrity(&validation)?;
    let schema: u32 = validation
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    let count = validation
        .query_row("SELECT count(*) FROM captures", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(map_sql_error)? as u64;
    if schema != manifest.schema_version || count != manifest.capture_count {
        return Err(backup_invalid(
            "O manifest nao corresponde ao banco do backup.",
        ));
    }

    Ok(ExtractedBackup {
        _directory: directory,
        database_path,
        manifest,
    })
}

fn write_archive(
    destination: &Path,
    snapshot: &Path,
    manifest: &BackupManifest,
) -> Result<(), CoreError> {
    let file = File::create(destination).map_err(map_io_error)?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o600);

    archive
        .start_file(MANIFEST_NAME, options)
        .map_err(backup_zip_error)?;
    archive
        .write_all(
            serde_json::to_string_pretty(manifest)
                .map_err(|error| backup_invalid(error.to_string()))?
                .as_bytes(),
        )
        .map_err(map_io_error)?;
    archive
        .start_file(DATABASE_NAME, options)
        .map_err(backup_zip_error)?;
    let mut database = File::open(snapshot).map_err(map_io_error)?;
    std::io::copy(&mut database, &mut archive).map_err(map_io_error)?;
    archive
        .finish()
        .map_err(backup_zip_error)?
        .sync_all()
        .map_err(map_io_error)
}

fn sha256_file(path: &Path) -> Result<String, CoreError> {
    let mut file = File::open(path).map_err(map_io_error)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(map_io_error)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn prune_daily_backups(directory: &Path, retain: usize) -> Result<(), CoreError> {
    let mut backups = fs::read_dir(directory)
        .map_err(map_io_error)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("daily-"))
        .collect::<Vec<_>>();
    backups.sort_by_key(|entry| entry.file_name());
    let remove_count = backups.len().saturating_sub(retain);
    for entry in backups.into_iter().take(remove_count) {
        fs::remove_file(entry.path()).map_err(map_io_error)?;
    }
    Ok(())
}

fn parse_time(value: &str) -> Result<OffsetDateTime, CoreError> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|error| backup_invalid(format!("Data do manifest invalida: {error}")))
}

fn format_error(error: time::error::Format) -> CoreError {
    CoreError::new(
        ErrorCode::Io,
        format!("Falha ao formatar data: {error}"),
        false,
    )
}

fn backup_zip_error(error: zip::result::ZipError) -> CoreError {
    backup_invalid(error.to_string())
}

fn backup_read_error(error: std::io::Error) -> CoreError {
    backup_invalid(error.to_string())
}

fn backup_invalid(message: impl Into<String>) -> CoreError {
    CoreError::new(
        ErrorCode::BackupInvalid,
        format!("Backup invalido: {}", message.into()),
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{AppRepository, CaptureRepository, CaptureSource, NewCapture, NewRegisteredApp};

    #[test]
    fn backup_restore_round_trip_and_safety_backup() {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("internal"),
        )
        .unwrap();
        let original = storage
            .create(NewCapture::create("Original", CaptureSource::Home).unwrap())
            .unwrap();
        let backup_path = directory.path().join("manual.mos-backup");
        storage.create_backup(&backup_path).unwrap();
        storage
            .create(NewCapture::create("Posterior", CaptureSource::Home).unwrap())
            .unwrap();

        let inspection = storage.inspect_backup(&backup_path).unwrap();
        assert_eq!(inspection.capture_count, 1);
        let safety = storage.restore_backup(&backup_path).unwrap();
        assert!(Path::new(&safety.path).exists());
        assert_eq!(storage.recent(10).unwrap()[0].id, original.id);
        assert_eq!(storage.recent(10).unwrap().len(), 1);
    }

    #[test]
    fn rejects_tampered_backup_before_restore() {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("internal"),
        )
        .unwrap();
        let backup_path = directory.path().join("manual.mos-backup");
        storage.create_backup(&backup_path).unwrap();
        let mut bytes = fs::read(&backup_path).unwrap();
        let middle = bytes.len() / 2;
        bytes[middle] ^= 0xff;
        fs::write(&backup_path, bytes).unwrap();

        assert_eq!(
            storage.inspect_backup(&backup_path).unwrap_err().code,
            ErrorCode::BackupInvalid
        );
    }

    #[test]
    fn json_export_is_versioned_and_readable() {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("internal"),
        )
        .unwrap();
        storage
            .create(NewCapture::create("Exportada", CaptureSource::Home).unwrap())
            .unwrap();
        storage
            .create_app(NewRegisteredApp::create("M-Finance", "Cockpit", None, None).unwrap())
            .unwrap();
        let destination = directory.path().join("m-os-export.json");
        storage.export_json(&destination).unwrap();
        let export: serde_json::Value =
            serde_json::from_slice(&fs::read(destination).unwrap()).unwrap();

        assert_eq!(export["format"], "m-os-export");
        assert_eq!(export["formatVersion"], 1);
        assert_eq!(export["captures"].as_array().unwrap().len(), 1);
        assert_eq!(export["apps"].as_array().unwrap().len(), 1);
    }
}
