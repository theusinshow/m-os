use std::{path::Path, sync::Mutex, time::Instant};

use rusqlite::{params, Connection};
use serde::Serialize;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS captures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL CHECK (length(trim(content)) > 0),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE VIRTUAL TABLE IF NOT EXISTS capture_fts USING fts5(
    content,
    content='captures',
    content_rowid='id'
);
"#;

pub struct Storage {
    connection: Mutex<Connection>,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReceipt {
    pub id: i64,
    pub committed_in_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRow {
    pub id: i64,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatus {
    pub database_path: String,
    pub journal_mode: String,
    pub synchronous: String,
    pub sqlite_version: String,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(error_text)?;
        Self::configure(&connection)?;
        connection.execute_batch(SCHEMA).map_err(error_text)?;

        Ok(Self {
            connection: Mutex::new(connection),
            path: path.display().to_string(),
        })
    }

    fn configure(connection: &Connection) -> Result<(), String> {
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(error_text)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;\
                 PRAGMA synchronous=FULL;\
                 PRAGMA foreign_keys=ON;",
            )
            .map_err(error_text)
    }

    pub fn save_capture(&self, content: &str) -> Result<CaptureReceipt, String> {
        let content = content.trim();
        if content.is_empty() {
            return Err("A captura nao pode estar vazia.".into());
        }

        let started = Instant::now();
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection.transaction().map_err(error_text)?;
        transaction
            .execute("INSERT INTO captures (content) VALUES (?1)", [content])
            .map_err(error_text)?;
        let id = transaction.last_insert_rowid();
        transaction
            .execute(
                "INSERT INTO capture_fts (rowid, content) VALUES (?1, ?2)",
                params![id, content],
            )
            .map_err(error_text)?;
        transaction.commit().map_err(error_text)?;

        Ok(CaptureReceipt {
            id,
            committed_in_ms: started.elapsed().as_millis(),
        })
    }

    pub fn list_captures(&self, query: Option<&str>) -> Result<Vec<CaptureRow>, String> {
        let connection = self.connection.lock().map_err(lock_error)?;
        let query = query.map(str::trim).filter(|value| !value.is_empty());

        let (sql, parameter) = if let Some(query) = query {
            let escaped = query.replace('"', "\"\"");
            (
                "SELECT c.id, c.content, c.created_at
                 FROM capture_fts f
                 JOIN captures c ON c.id = f.rowid
                 WHERE capture_fts MATCH ?1
                 ORDER BY c.id DESC
                 LIMIT 50",
                Some(format!("\"{escaped}\"*")),
            )
        } else {
            (
                "SELECT id, content, created_at FROM captures ORDER BY id DESC LIMIT 50",
                None,
            )
        };

        let mut statement = connection.prepare(sql).map_err(error_text)?;
        let rows = match parameter {
            Some(parameter) => statement
                .query_map([parameter], map_capture)
                .map_err(error_text)?,
            None => statement.query_map([], map_capture).map_err(error_text)?,
        };

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(error_text)
    }

    pub fn status(&self) -> Result<StorageStatus, String> {
        let connection = self.connection.lock().map_err(lock_error)?;
        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(error_text)?;
        let synchronous_value: i64 = connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(error_text)?;
        let sqlite_version = connection
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .map_err(error_text)?;

        Ok(StorageStatus {
            database_path: self.path.clone(),
            journal_mode,
            synchronous: match synchronous_value {
                0 => "OFF",
                1 => "NORMAL",
                2 => "FULL",
                3 => "EXTRA",
                _ => "UNKNOWN",
            }
            .into(),
            sqlite_version,
        })
    }
}

fn map_capture(row: &rusqlite::Row<'_>) -> rusqlite::Result<CaptureRow> {
    Ok(CaptureRow {
        id: row.get(0)?,
        content: row.get(1)?,
        created_at: row.get(2)?,
    })
}

fn error_text(error: rusqlite::Error) -> String {
    error.to_string()
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_storage() -> (tempfile::TempDir, Storage) {
        let directory = tempfile::tempdir().expect("create temp directory");
        let storage = Storage::open(&directory.path().join("spike.db")).expect("open storage");
        (directory, storage)
    }

    #[test]
    fn configures_durable_local_storage() {
        let (_directory, storage) = test_storage();
        let status = storage.status().expect("read status");

        assert_eq!(status.journal_mode.to_lowercase(), "wal");
        assert_eq!(status.synchronous, "FULL");
    }

    #[test]
    fn capture_and_fts_index_commit_together() {
        let (_directory, storage) = test_storage();
        let receipt = storage
            .save_capture("Revisar arquitetura de captura")
            .expect("save capture");
        let results = storage
            .list_captures(Some("arquitetura"))
            .expect("search captures");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, receipt.id);
    }

    #[test]
    fn preserves_unicode_content_and_searches_diacritics() {
        let (_directory, storage) = test_storage();
        storage
            .save_capture("Revisar ação no iOS")
            .expect("save unicode capture");
        let results = storage
            .list_captures(Some("ação"))
            .expect("search unicode capture");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Revisar ação no iOS");
    }

    #[test]
    fn rolls_back_capture_when_index_write_fails() {
        let (_directory, storage) = test_storage();
        {
            let connection = storage.connection.lock().expect("lock connection");
            connection
                .execute("DROP TABLE capture_fts", [])
                .expect("drop test index");
        }

        assert!(storage.save_capture("Nao deve persistir").is_err());
        let connection = storage.connection.lock().expect("lock connection");
        let count: i64 = connection
            .query_row("SELECT count(*) FROM captures", [], |row| row.get(0))
            .expect("count captures");
        assert_eq!(count, 0);
    }
}
