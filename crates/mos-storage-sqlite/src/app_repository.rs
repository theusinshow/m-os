use mos_core::{
    AppId, AppLaunchKind, AppRepository, CoreError, ErrorCode, LifecycleState, NewRegisteredApp,
    RegisteredApp, SearchRequest,
};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time, to_fts_query},
    SqliteStorage,
};

pub(crate) const APP_COLUMNS: &str = "id, name, description, launch_kind, launch_target, lifecycle_state, created_at, updated_at, last_opened_at";

struct RawApp {
    id: String,
    name: String,
    description: String,
    launch_kind: Option<String>,
    launch_target: Option<String>,
    lifecycle_state: String,
    created_at: String,
    updated_at: String,
    last_opened_at: Option<String>,
}

impl RawApp {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            launch_kind: row.get(3)?,
            launch_target: row.get(4)?,
            lifecycle_state: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
            last_opened_at: row.get(8)?,
        })
    }

    fn into_app(self) -> Result<RegisteredApp, CoreError> {
        Ok(RegisteredApp {
            id: AppId::parse(&self.id)?,
            name: self.name,
            description: self.description,
            launch_kind: self
                .launch_kind
                .as_deref()
                .map(AppLaunchKind::parse)
                .transpose()?,
            launch_target: self.launch_target,
            lifecycle_state: LifecycleState::parse(&self.lifecycle_state)?,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
            last_opened_at: self.last_opened_at.as_deref().map(parse_time).transpose()?,
        })
    }
}

impl AppRepository for SqliteStorage {
    fn create_app(&self, app: NewRegisteredApp) -> Result<RegisteredApp, CoreError> {
        let now = format_time(app.created_at)?;
        let id = app.id;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO apps (
                    id, name, description, launch_kind, launch_target,
                    lifecycle_state, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?6)",
                params![
                    app.id.to_string(),
                    app.name,
                    app.description,
                    app.launch_kind.map(AppLaunchKind::as_str),
                    app.launch_target,
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        insert_app_search(&transaction, transaction.last_insert_rowid())?;
        transaction.commit().map_err(map_sql_error)?;
        query_app(&connection, id)
    }

    fn update_app(
        &self,
        id: AppId,
        name: &str,
        description: &str,
        launch_kind: Option<AppLaunchKind>,
        launch_target: Option<&str>,
    ) -> Result<RegisteredApp, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        delete_app_search(&transaction, id)?;
        let changed = transaction
            .execute(
                "UPDATE apps
                 SET name = ?1, description = ?2, launch_kind = ?3, launch_target = ?4,
                     updated_at = ?5
                 WHERE id = ?6",
                params![
                    name,
                    description,
                    launch_kind.map(AppLaunchKind::as_str),
                    launch_target,
                    now,
                    id.to_string()
                ],
            )
            .map_err(map_sql_error)?;
        ensure_app_changed(changed)?;
        let rowid: i64 = transaction
            .query_row(
                "SELECT rowid FROM apps WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        insert_app_search(&transaction, rowid)?;
        transaction.commit().map_err(map_sql_error)?;
        query_app(&connection, id)
    }

    fn get_app(&self, id: AppId) -> Result<RegisteredApp, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_app(&connection, id)
    }

    fn apps(&self, include_archived: bool) -> Result<Vec<RegisteredApp>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if include_archived {
            "lifecycle_state IN ('active', 'archived')"
        } else {
            "lifecycle_state = 'active'"
        };
        query_apps(
            &connection,
            &format!(
                "SELECT {APP_COLUMNS} FROM apps WHERE {lifecycle}
                 ORDER BY COALESCE(last_opened_at, updated_at) DESC"
            ),
        )
    }

    fn set_app_lifecycle(
        &self,
        id: AppId,
        lifecycle: LifecycleState,
    ) -> Result<RegisteredApp, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let (archived_at, deleted_at) = match lifecycle {
            LifecycleState::Active => (None, None),
            LifecycleState::Archived => (Some(now.as_str()), None),
            LifecycleState::Trashed => (None, Some(now.as_str())),
        };
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE apps
                 SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3, deleted_at = ?4
                 WHERE id = ?5",
                params![
                    lifecycle.as_str(),
                    now,
                    archived_at,
                    deleted_at,
                    id.to_string()
                ],
            )
            .map_err(map_sql_error)?;
        ensure_app_changed(changed)?;
        query_app(&connection, id)
    }

    fn mark_app_opened(&self, id: AppId) -> Result<RegisteredApp, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE apps SET last_opened_at = ?1 WHERE id = ?2",
                params![now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_app_changed(changed)?;
        query_app(&connection, id)
    }

    fn search_apps(&self, request: SearchRequest) -> Result<Vec<RegisteredApp>, CoreError> {
        if request.query.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = to_fts_query(&request.query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let lifecycle = if request.include_archived {
            "a.lifecycle_state IN ('active', 'archived')"
        } else {
            "a.lifecycle_state = 'active'"
        };
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT a.{columns}
                 FROM app_search s
                 JOIN apps a ON a.rowid = s.rowid
                 WHERE app_search MATCH ?1 AND {lifecycle}
                 ORDER BY bm25(app_search), COALESCE(a.last_opened_at, a.updated_at) DESC
                 LIMIT ?2",
                columns = APP_COLUMNS.replace(", ", ", a.")
            ))
            .map_err(map_sql_error)?;
        let apps = statement
            .query_map(params![fts_query, request.limit as i64], RawApp::from_row)
            .map_err(map_sql_error)?
            .map(|row| row.map_err(map_sql_error)?.into_app())
            .collect();
        apps
    }

    fn rebuild_app_search(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute("INSERT INTO app_search(app_search) VALUES('rebuild')", [])
            .map_err(map_sql_error)?;
        let count = transaction
            .query_row("SELECT count(*) FROM app_search", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(map_sql_error)? as usize;
        transaction.commit().map_err(map_sql_error)?;
        Ok(count)
    }
}

fn insert_app_search(transaction: &Transaction<'_>, rowid: i64) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO app_search (rowid, name, description, launch_target)
             SELECT rowid, name, description, launch_target FROM apps WHERE rowid = ?1",
            [rowid],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn delete_app_search(transaction: &Transaction<'_>, id: AppId) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO app_search(app_search, rowid, name, description, launch_target)
             SELECT 'delete', rowid, name, description, launch_target FROM apps WHERE id = ?1",
            [id.to_string()],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

pub(crate) fn query_app(
    connection: &rusqlite::Connection,
    id: AppId,
) -> Result<RegisteredApp, CoreError> {
    connection
        .query_row(
            &format!("SELECT {APP_COLUMNS} FROM apps WHERE id = ?1"),
            [id.to_string()],
            RawApp::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "App nao encontrado.", false))?
        .into_app()
}

pub(crate) fn query_apps(
    connection: &rusqlite::Connection,
    sql: &str,
) -> Result<Vec<RegisteredApp>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let apps = statement
        .query_map([], RawApp::from_row)
        .map_err(map_sql_error)?
        .map(|row| row.map_err(map_sql_error)?.into_app())
        .collect();
    apps
}

pub(crate) fn query_apps_all(
    connection: &rusqlite::Connection,
) -> Result<Vec<RegisteredApp>, CoreError> {
    query_apps(
        connection,
        &format!("SELECT {APP_COLUMNS} FROM apps ORDER BY created_at ASC"),
    )
}

fn ensure_app_changed(changed: usize) -> Result<(), CoreError> {
    if changed == 0 {
        Err(CoreError::new(
            ErrorCode::NotFound,
            "App nao encontrado.",
            false,
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage() -> (tempfile::TempDir, SqliteStorage) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (directory, storage)
    }

    #[test]
    fn create_update_and_search_app() {
        let (_directory, storage) = storage();
        let app = storage
            .create_app(
                NewRegisteredApp::create(
                    "M-Finance",
                    "Cockpit mensal",
                    Some(AppLaunchKind::Url),
                    Some("https://m-finance.local"),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(app.name, "M-Finance");

        let updated = storage
            .update_app(
                app.id,
                "M Finance",
                "Contas e faturas",
                Some(AppLaunchKind::Path),
                Some("C:\\Apps\\m-finance.exe"),
            )
            .unwrap();
        assert_eq!(updated.launch_kind, Some(AppLaunchKind::Path));

        let results = storage
            .search_apps(SearchRequest {
                query: "faturas".into(),
                include_archived: false,
                limit: 20,
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, app.id);
    }

    #[test]
    fn archived_app_is_hidden_by_default() {
        let (_directory, storage) = storage();
        let app = storage
            .create_app(NewRegisteredApp::create("NexoDoc", "", None, None).unwrap())
            .unwrap();
        storage
            .set_app_lifecycle(app.id, LifecycleState::Archived)
            .unwrap();

        assert!(storage.apps(false).unwrap().is_empty());
        assert_eq!(storage.apps(true).unwrap().len(), 1);
    }
}
