use std::collections::{HashMap, HashSet};

use mos_core::{
    AppId, Capture, CaptureId, CaptureRepository, CoreError, ErrorCode, LifecycleState, NewProject,
    NewTask, NewWorkspace, Project, ProjectId, RegisteredApp, SearchItem, SearchRequest, Task,
    TaskId, TaskState, WorkRepository, Workspace, WorkspaceId,
};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use time::OffsetDateTime;

use crate::{
    app_repository::{query_apps, APP_COLUMNS},
    map_lock_error, map_sql_error,
    repository::{
        ensure_changed, format_time, parse_time, query_capture, to_fts_query, RawCapture,
        CAPTURE_COLUMNS,
    },
    SqliteStorage,
};

pub(crate) const PROJECT_COLUMNS: &str =
    "id, name, description, lifecycle_state, created_at, updated_at, repository";
pub(crate) const WORKSPACE_COLUMNS: &str =
    "id, name, description, lifecycle_state, created_at, updated_at";
pub(crate) const TASK_COLUMNS: &str = "id, title, description, project_id, source_capture_id, work_state, lifecycle_state, created_at, updated_at, completed_at";

struct RawProject {
    id: String,
    name: String,
    description: String,
    lifecycle_state: String,
    created_at: String,
    updated_at: String,
    repository: String,
}

impl RawProject {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            lifecycle_state: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            repository: row.get(6)?,
        })
    }

    fn into_project(self) -> Result<Project, CoreError> {
        Ok(Project {
            id: ProjectId::parse(&self.id)?,
            name: self.name,
            description: self.description,
            repository: self.repository,
            lifecycle_state: LifecycleState::parse(&self.lifecycle_state)?,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

struct RawWorkspace {
    id: String,
    name: String,
    description: String,
    lifecycle_state: String,
    created_at: String,
    updated_at: String,
}

impl RawWorkspace {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            lifecycle_state: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }

    fn into_workspace(self) -> Result<Workspace, CoreError> {
        Ok(Workspace {
            id: WorkspaceId::parse(&self.id)?,
            name: self.name,
            description: self.description,
            lifecycle_state: LifecycleState::parse(&self.lifecycle_state)?,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

struct RawTask {
    id: String,
    title: String,
    description: String,
    project_id: Option<String>,
    source_capture_id: Option<String>,
    state: String,
    lifecycle_state: String,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
}

impl RawTask {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            project_id: row.get(3)?,
            source_capture_id: row.get(4)?,
            state: row.get(5)?,
            lifecycle_state: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            completed_at: row.get(9)?,
        })
    }

    fn into_task(self) -> Result<Task, CoreError> {
        Ok(Task {
            id: TaskId::parse(&self.id)?,
            title: self.title,
            description: self.description,
            project_id: self
                .project_id
                .as_deref()
                .map(ProjectId::parse)
                .transpose()?,
            source_capture_id: self
                .source_capture_id
                .as_deref()
                .map(CaptureId::parse)
                .transpose()?,
            state: TaskState::parse(&self.state)?,
            lifecycle_state: LifecycleState::parse(&self.lifecycle_state)?,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
            completed_at: self.completed_at.as_deref().map(parse_time).transpose()?,
        })
    }
}

impl WorkRepository for SqliteStorage {
    fn create_workspace(&self, workspace: NewWorkspace) -> Result<Workspace, CoreError> {
        let now = format_time(workspace.created_at)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO workspaces (id, name, description, lifecycle_state, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'active', ?4, ?4)",
                params![
                    workspace.id.to_string(),
                    workspace.name,
                    workspace.description,
                    now
                ],
            )
            .map_err(map_sql_error)?;
        let rowid = transaction.last_insert_rowid();
        insert_workspace_search(&transaction, rowid)?;
        transaction.commit().map_err(map_sql_error)?;
        query_workspace(&connection, workspace.id)
    }

    fn update_workspace(
        &self,
        id: WorkspaceId,
        name: &str,
        description: &str,
    ) -> Result<Workspace, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        delete_workspace_search(&transaction, id)?;
        let changed = transaction
            .execute(
                "UPDATE workspaces SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
                params![name, description, now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        let rowid: i64 = transaction
            .query_row(
                "SELECT rowid FROM workspaces WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        insert_workspace_search(&transaction, rowid)?;
        transaction.commit().map_err(map_sql_error)?;
        query_workspace(&connection, id)
    }

    fn get_workspace(&self, id: WorkspaceId) -> Result<Workspace, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_workspace(&connection, id)
    }

    fn workspaces(&self, include_archived: bool) -> Result<Vec<Workspace>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if include_archived {
            "lifecycle_state IN ('active', 'archived')"
        } else {
            "lifecycle_state = 'active'"
        };
        query_workspaces(
            &connection,
            &format!(
                "SELECT {WORKSPACE_COLUMNS} FROM workspaces WHERE {lifecycle} ORDER BY updated_at DESC"
            ),
        )
    }

    fn set_workspace_lifecycle(
        &self,
        id: WorkspaceId,
        lifecycle: LifecycleState,
    ) -> Result<Workspace, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let archived_at = (lifecycle == LifecycleState::Archived).then_some(now.as_str());
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE workspaces SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3
                 WHERE id = ?4",
                params![lifecycle.as_str(), now, archived_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        query_workspace(&connection, id)
    }

    fn workspace_projects(
        &self,
        id: WorkspaceId,
        include_archived: bool,
    ) -> Result<Vec<Project>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if include_archived {
            "p.lifecycle_state IN ('active', 'archived')"
        } else {
            "p.lifecycle_state = 'active'"
        };
        query_projects(
            &connection,
            &format!(
                "SELECT p.{columns}
                 FROM project_workspaces pw
                 JOIN projects p ON p.id = pw.project_id
                 WHERE pw.workspace_id = {workspace_id} AND {lifecycle}
                 ORDER BY p.updated_at DESC",
                columns = PROJECT_COLUMNS.replace(", ", ", p."),
                workspace_id = quote_sql(&id.to_string()),
            ),
        )
    }

    fn workspace_apps(
        &self,
        id: WorkspaceId,
        include_archived: bool,
    ) -> Result<Vec<RegisteredApp>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if include_archived {
            "a.lifecycle_state IN ('active', 'archived')"
        } else {
            "a.lifecycle_state = 'active'"
        };
        query_apps(
            &connection,
            &format!(
                "SELECT a.{columns}
                 FROM app_workspaces aw
                 JOIN apps a ON a.id = aw.app_id
                 WHERE aw.workspace_id = {workspace_id} AND {lifecycle}
                 ORDER BY COALESCE(a.last_opened_at, a.updated_at) DESC",
                columns = APP_COLUMNS.replace(", ", ", a."),
                workspace_id = quote_sql(&id.to_string()),
            ),
        )
    }

    fn project_workspaces(&self, id: ProjectId) -> Result<Vec<Workspace>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_workspaces(
            &connection,
            &format!(
                "SELECT w.{columns}
                 FROM project_workspaces pw
                 JOIN workspaces w ON w.id = pw.workspace_id
                 WHERE pw.project_id = {project_id} AND w.lifecycle_state = 'active'
                 ORDER BY w.updated_at DESC",
                columns = WORKSPACE_COLUMNS.replace(", ", ", w."),
                project_id = quote_sql(&id.to_string()),
            ),
        )
    }

    fn app_workspaces(&self, id: AppId) -> Result<Vec<Workspace>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_workspaces(
            &connection,
            &format!(
                "SELECT w.{columns}
                 FROM app_workspaces aw
                 JOIN workspaces w ON w.id = aw.workspace_id
                 WHERE aw.app_id = {app_id} AND w.lifecycle_state = 'active'
                 ORDER BY w.updated_at DESC",
                columns = WORKSPACE_COLUMNS.replace(", ", ", w."),
                app_id = quote_sql(&id.to_string()),
            ),
        )
    }

    fn set_project_workspace(
        &self,
        project_id: ProjectId,
        workspace_id: WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        if linked {
            let now = format_time(OffsetDateTime::now_utc())?;
            connection
                .execute(
                    "INSERT OR IGNORE INTO project_workspaces (project_id, workspace_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![project_id.to_string(), workspace_id.to_string(), now],
                )
                .map_err(map_sql_error)?;
        } else {
            connection
                .execute(
                    "DELETE FROM project_workspaces WHERE project_id = ?1 AND workspace_id = ?2",
                    params![project_id.to_string(), workspace_id.to_string()],
                )
                .map_err(map_sql_error)?;
        }
        Ok(())
    }

    fn set_app_workspace(
        &self,
        app_id: AppId,
        workspace_id: WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        if linked {
            let now = format_time(OffsetDateTime::now_utc())?;
            connection
                .execute(
                    "INSERT OR IGNORE INTO app_workspaces (app_id, workspace_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![app_id.to_string(), workspace_id.to_string(), now],
                )
                .map_err(map_sql_error)?;
        } else {
            connection
                .execute(
                    "DELETE FROM app_workspaces WHERE app_id = ?1 AND workspace_id = ?2",
                    params![app_id.to_string(), workspace_id.to_string()],
                )
                .map_err(map_sql_error)?;
        }
        Ok(())
    }

    fn create_project(&self, project: NewProject) -> Result<Project, CoreError> {
        let now = format_time(project.created_at)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO projects (id, name, description, lifecycle_state, created_at, updated_at, repository)
                 VALUES (?1, ?2, ?3, 'active', ?4, ?4, ?5)",
                params![
                    project.id.to_string(),
                    project.name,
                    project.description,
                    now,
                    project.repository
                ],
            )
            .map_err(map_sql_error)?;
        let rowid = transaction.last_insert_rowid();
        insert_project_search(&transaction, rowid)?;
        transaction.commit().map_err(map_sql_error)?;
        query_project(&connection, project.id)
    }

    fn update_project(
        &self,
        id: ProjectId,
        name: &str,
        description: &str,
        repository: &str,
    ) -> Result<Project, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        delete_project_search(&transaction, id)?;
        let changed = transaction
            .execute(
                "UPDATE projects
                 SET name = ?1, description = ?2, updated_at = ?3, repository = ?5
                 WHERE id = ?4",
                params![name, description, now, id.to_string(), repository],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        let rowid: i64 = transaction
            .query_row(
                "SELECT rowid FROM projects WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        insert_project_search(&transaction, rowid)?;
        transaction.commit().map_err(map_sql_error)?;
        query_project(&connection, id)
    }

    fn get_project(&self, id: ProjectId) -> Result<Project, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_project(&connection, id)
    }

    fn projects(&self, include_archived: bool) -> Result<Vec<Project>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if include_archived {
            "lifecycle_state IN ('active', 'archived')"
        } else {
            "lifecycle_state = 'active'"
        };
        query_projects(
            &connection,
            &format!(
                "SELECT {PROJECT_COLUMNS} FROM projects WHERE {lifecycle} ORDER BY updated_at DESC"
            ),
        )
    }

    fn set_project_lifecycle(
        &self,
        id: ProjectId,
        lifecycle: LifecycleState,
    ) -> Result<Project, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let archived_at = (lifecycle == LifecycleState::Archived).then_some(now.as_str());
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE projects SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3
                 WHERE id = ?4",
                params![lifecycle.as_str(), now, archived_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        query_project(&connection, id)
    }

    fn create_task(&self, task: NewTask) -> Result<Task, CoreError> {
        let id = task.id;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        insert_task(&transaction, task, None)?;
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, id)
    }

    fn create_task_from_capture(
        &self,
        capture_id: CaptureId,
        task: NewTask,
    ) -> Result<Task, CoreError> {
        let id = task.id;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let already_exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE source_capture_id = ?1)",
                [capture_id.to_string()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        if already_exists {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Esta Capture ja possui uma Task derivada.",
                false,
            ));
        }
        insert_task(&transaction, task, Some(capture_id))?;
        let now = format_time(OffsetDateTime::now_utc())?;
        let changed = transaction
            .execute(
                "UPDATE captures SET processing_state = 'processed', updated_at = ?1
                 WHERE id = ?2 AND processing_state = 'inbox' AND lifecycle_state = 'active'",
                params![now, capture_id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed != 1 {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "A Capture nao esta disponivel para processamento.",
                false,
            ));
        }
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, id)
    }

    fn update_task(
        &self,
        id: TaskId,
        title: &str,
        description: &str,
        project_id: Option<ProjectId>,
    ) -> Result<Task, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        delete_task_search(&transaction, id)?;
        let changed = transaction
            .execute(
                "UPDATE tasks SET title = ?1, description = ?2, project_id = ?3, updated_at = ?4
                 WHERE id = ?5",
                params![
                    title,
                    description,
                    project_id.map(|value| value.to_string()),
                    now,
                    id.to_string()
                ],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        let rowid: i64 = transaction
            .query_row(
                "SELECT rowid FROM tasks WHERE id = ?1",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        insert_task_search(&transaction, rowid)?;
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, id)
    }

    fn get_task(&self, id: TaskId) -> Result<Task, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_task(&connection, id)
    }

    fn tasks(&self, include_archived: bool) -> Result<Vec<Task>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if include_archived {
            "lifecycle_state IN ('active', 'archived')"
        } else {
            "lifecycle_state = 'active'"
        };
        query_tasks(
            &connection,
            &format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE {lifecycle}
                 ORDER BY CASE work_state WHEN 'doing' THEN 0 WHEN 'backlog' THEN 1 ELSE 2 END,
                 updated_at DESC"
            ),
        )
    }

    fn set_task_state(&self, id: TaskId, state: TaskState) -> Result<Task, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let completed_at = (state == TaskState::Done).then_some(now.as_str());
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE tasks SET work_state = ?1, updated_at = ?2, completed_at = ?3 WHERE id = ?4",
                params![state.as_str(), now, completed_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        query_task(&connection, id)
    }

    fn set_task_lifecycle(&self, id: TaskId, lifecycle: LifecycleState) -> Result<Task, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let archived_at = (lifecycle == LifecycleState::Archived).then_some(now.as_str());
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE tasks SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3
                 WHERE id = ?4",
                params![lifecycle.as_str(), now, archived_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        query_task(&connection, id)
    }

    fn search_all(&self, request: SearchRequest) -> Result<Vec<SearchItem>, CoreError> {
        if request.query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let capture_hits = CaptureRepository::search(self, request.clone())?;
        let fts_query = to_fts_query(&request.query);
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if request.include_archived {
            "IN ('active', 'archived')"
        } else {
            "= 'active'"
        };
        let task_hits = query_tasks(
            &connection,
            &format!(
                "SELECT t.{columns} FROM task_search s JOIN tasks t ON t.rowid = s.rowid
                 WHERE task_search MATCH {query} AND t.lifecycle_state {lifecycle}
                 ORDER BY bm25(task_search), t.updated_at DESC LIMIT {limit}",
                columns = TASK_COLUMNS.replace(", ", ", t."),
                query = quote_sql(&fts_query),
                limit = request.limit,
            ),
        )?;
        let project_hits = query_projects(
            &connection,
            &format!(
                "SELECT p.{columns} FROM project_search s JOIN projects p ON p.rowid = s.rowid
                 WHERE project_search MATCH {query} AND p.lifecycle_state {lifecycle}
                 ORDER BY bm25(project_search), p.updated_at DESC LIMIT {limit}",
                columns = PROJECT_COLUMNS.replace(", ", ", p."),
                query = quote_sql(&fts_query),
                limit = request.limit,
            ),
        )?;
        let workspace_hits = query_workspaces(
            &connection,
            &format!(
                "SELECT w.{columns} FROM workspace_search s JOIN workspaces w ON w.rowid = s.rowid
                 WHERE workspace_search MATCH {query} AND w.lifecycle_state {lifecycle}
                 ORDER BY bm25(workspace_search), w.updated_at DESC LIMIT {limit}",
                columns = WORKSPACE_COLUMNS.replace(", ", ", w."),
                query = quote_sql(&fts_query),
                limit = request.limit,
            ),
        )?;

        let mut capture_map = capture_hits
            .into_iter()
            .map(|capture| (capture.id, capture))
            .collect::<HashMap<_, _>>();
        for task in &task_hits {
            if let Some(capture_id) = task.source_capture_id {
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    capture_map.entry(capture_id)
                {
                    entry.insert(query_capture(&connection, capture_id)?);
                }
            }
        }

        let mut grouped_task_ids = HashSet::new();
        let mut items = Vec::new();
        for capture in capture_map.into_values() {
            let derived_task =
                query_task_for_capture(&connection, capture.id, request.include_archived)?;
            let project = derived_task
                .as_ref()
                .and_then(|task| task.project_id)
                .map(|id| query_project(&connection, id))
                .transpose()?;
            if let Some(task) = &derived_task {
                grouped_task_ids.insert(task.id);
            }
            items.push(SearchItem::Capture {
                capture,
                derived_task,
                project,
            });
        }
        for task in task_hits {
            if grouped_task_ids.contains(&task.id) || task.source_capture_id.is_some() {
                continue;
            }
            let project = task
                .project_id
                .map(|id| query_project(&connection, id))
                .transpose()?;
            items.push(SearchItem::Task { task, project });
        }
        items.extend(
            project_hits
                .into_iter()
                .map(|project| SearchItem::Project { project }),
        );
        items.extend(
            workspace_hits
                .into_iter()
                .map(|workspace| SearchItem::Workspace { workspace }),
        );
        items.truncate(request.limit);
        Ok(items)
    }

    fn rebuild_all_search(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        for table in [
            "capture_search",
            "project_search",
            "task_search",
            "workspace_search",
        ] {
            transaction
                .execute(
                    &format!("INSERT INTO {table}({table}) VALUES('rebuild')"),
                    [],
                )
                .map_err(map_sql_error)?;
        }
        let count = [
            "capture_search",
            "project_search",
            "task_search",
            "workspace_search",
        ]
        .into_iter()
        .try_fold(0_usize, |total, table| {
            transaction
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
                .map(|count| total + count as usize)
                .map_err(map_sql_error)
        })?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(count)
    }
}

fn insert_task(
    transaction: &Transaction<'_>,
    task: NewTask,
    source_capture_id: Option<CaptureId>,
) -> Result<(), CoreError> {
    let now = format_time(task.created_at)?;
    transaction
        .execute(
            "INSERT INTO tasks (
                id, title, description, project_id, source_capture_id, work_state,
                lifecycle_state, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'backlog', 'active', ?6, ?6)",
            params![
                task.id.to_string(),
                task.title,
                task.description,
                task.project_id.map(|value| value.to_string()),
                source_capture_id.map(|value| value.to_string()),
                now,
            ],
        )
        .map_err(map_sql_error)?;
    insert_task_search(transaction, transaction.last_insert_rowid())
}

fn insert_project_search(transaction: &Transaction<'_>, rowid: i64) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO project_search (rowid, name, description)
             SELECT rowid, name, description FROM projects WHERE rowid = ?1",
            [rowid],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn insert_workspace_search(transaction: &Transaction<'_>, rowid: i64) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO workspace_search (rowid, name, description)
             SELECT rowid, name, description FROM workspaces WHERE rowid = ?1",
            [rowid],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn insert_task_search(transaction: &Transaction<'_>, rowid: i64) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO task_search (rowid, title, description)
             SELECT rowid, title, description FROM tasks WHERE rowid = ?1",
            [rowid],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn delete_project_search(transaction: &Transaction<'_>, id: ProjectId) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO project_search(project_search, rowid, name, description)
             SELECT 'delete', rowid, name, description FROM projects WHERE id = ?1",
            [id.to_string()],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn delete_task_search(transaction: &Transaction<'_>, id: TaskId) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO task_search(task_search, rowid, title, description)
             SELECT 'delete', rowid, title, description FROM tasks WHERE id = ?1",
            [id.to_string()],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn delete_workspace_search(
    transaction: &Transaction<'_>,
    id: WorkspaceId,
) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO workspace_search(workspace_search, rowid, name, description)
             SELECT 'delete', rowid, name, description FROM workspaces WHERE id = ?1",
            [id.to_string()],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

pub(crate) fn query_project(
    connection: &rusqlite::Connection,
    id: ProjectId,
) -> Result<Project, CoreError> {
    connection
        .query_row(
            &format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE id = ?1"),
            [id.to_string()],
            RawProject::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Project nao encontrado.", false))?
        .into_project()
}

pub(crate) fn query_workspace(
    connection: &rusqlite::Connection,
    id: WorkspaceId,
) -> Result<Workspace, CoreError> {
    connection
        .query_row(
            &format!("SELECT {WORKSPACE_COLUMNS} FROM workspaces WHERE id = ?1"),
            [id.to_string()],
            RawWorkspace::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Workspace nao encontrado.", false))?
        .into_workspace()
}

pub(crate) fn query_task(connection: &rusqlite::Connection, id: TaskId) -> Result<Task, CoreError> {
    connection
        .query_row(
            &format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?1"),
            [id.to_string()],
            RawTask::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Task nao encontrada.", false))?
        .into_task()
}

fn query_task_for_capture(
    connection: &rusqlite::Connection,
    capture_id: CaptureId,
    include_archived: bool,
) -> Result<Option<Task>, CoreError> {
    let lifecycle = if include_archived {
        "lifecycle_state IN ('active', 'archived')"
    } else {
        "lifecycle_state = 'active'"
    };
    connection
        .query_row(
            &format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE source_capture_id = ?1 AND {lifecycle}"
            ),
            [capture_id.to_string()],
            RawTask::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .map(RawTask::into_task)
        .transpose()
}

pub(crate) fn query_workspaces(
    connection: &rusqlite::Connection,
    sql: &str,
) -> Result<Vec<Workspace>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let workspaces = statement
        .query_map([], RawWorkspace::from_row)
        .map_err(map_sql_error)?
        .map(|row| row.map_err(map_sql_error)?.into_workspace())
        .collect();
    workspaces
}

pub(crate) fn query_workspaces_all(
    connection: &rusqlite::Connection,
) -> Result<Vec<Workspace>, CoreError> {
    query_workspaces(
        connection,
        &format!("SELECT {WORKSPACE_COLUMNS} FROM workspaces ORDER BY created_at ASC"),
    )
}

pub(crate) fn query_project_workspace_links(
    connection: &rusqlite::Connection,
) -> Result<Vec<(ProjectId, WorkspaceId)>, CoreError> {
    let mut statement = connection
        .prepare("SELECT project_id, workspace_id FROM project_workspaces ORDER BY created_at ASC")
        .map_err(map_sql_error)?;
    let links = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(map_sql_error)?
        .map(|row| {
            let (project_id, workspace_id) = row.map_err(map_sql_error)?;
            Ok((
                ProjectId::parse(&project_id)?,
                WorkspaceId::parse(&workspace_id)?,
            ))
        })
        .collect();
    links
}

pub(crate) fn query_app_workspace_links(
    connection: &rusqlite::Connection,
) -> Result<Vec<(AppId, WorkspaceId)>, CoreError> {
    let mut statement = connection
        .prepare("SELECT app_id, workspace_id FROM app_workspaces ORDER BY created_at ASC")
        .map_err(map_sql_error)?;
    let links = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(map_sql_error)?
        .map(|row| {
            let (app_id, workspace_id) = row.map_err(map_sql_error)?;
            Ok((AppId::parse(&app_id)?, WorkspaceId::parse(&workspace_id)?))
        })
        .collect();
    links
}

pub(crate) fn query_projects(
    connection: &rusqlite::Connection,
    sql: &str,
) -> Result<Vec<Project>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let projects = statement
        .query_map([], RawProject::from_row)
        .map_err(map_sql_error)?
        .map(|row| row.map_err(map_sql_error)?.into_project())
        .collect();
    projects
}

pub(crate) fn query_tasks(
    connection: &rusqlite::Connection,
    sql: &str,
) -> Result<Vec<Task>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let tasks = statement
        .query_map([], RawTask::from_row)
        .map_err(map_sql_error)?
        .map(|row| row.map_err(map_sql_error)?.into_task())
        .collect();
    tasks
}

pub(crate) fn query_captures_all(
    connection: &rusqlite::Connection,
) -> Result<Vec<Capture>, CoreError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {CAPTURE_COLUMNS} FROM captures ORDER BY captured_at ASC"
        ))
        .map_err(map_sql_error)?;
    let captures = statement
        .query_map([], RawCapture::from_row)
        .map_err(map_sql_error)?
        .map(|row| row.map_err(map_sql_error)?.into_capture())
        .collect();
    captures
}

fn quote_sql(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{AppLaunchKind, AppRepository, CaptureSource, NewCapture, NewRegisteredApp};

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
    fn capture_to_task_is_atomic_and_preserves_origin() {
        let (_directory, storage) = storage();
        let capture = storage
            .create(NewCapture::create("Refatorar navbar", CaptureSource::Home).unwrap())
            .unwrap();
        let project = storage
            .create_project(NewProject::create("Minarum", "", "").unwrap())
            .unwrap();
        let task = storage
            .create_task_from_capture(
                capture.id,
                NewTask::create("Refatorar navbar", "", Some(project.id)).unwrap(),
            )
            .unwrap();

        assert_eq!(task.source_capture_id, Some(capture.id));
        assert_eq!(
            storage.get(capture.id).unwrap().processing_state,
            mos_core::ProcessingState::Processed
        );
        assert_eq!(storage.get(capture.id).unwrap().content, "Refatorar navbar");
        assert!(storage
            .create_task_from_capture(capture.id, NewTask::create("Duplicada", "", None).unwrap(),)
            .is_err());
    }

    #[test]
    fn search_groups_a_derived_task_with_its_capture() {
        let (_directory, storage) = storage();
        let capture = storage
            .create(NewCapture::create("Contexto exclusivo", CaptureSource::Home).unwrap())
            .unwrap();
        storage
            .create_task_from_capture(
                capture.id,
                NewTask::create("Executar trabalho", "", None).unwrap(),
            )
            .unwrap();

        let results = storage
            .search_all(SearchRequest {
                query: "exclusivo".into(),
                include_archived: false,
                limit: 20,
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(matches!(
            &results[0],
            SearchItem::Capture {
                derived_task: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn done_and_reopen_manage_completed_at() {
        let (_directory, storage) = storage();
        let task = storage
            .create_task(NewTask::create("Concluir", "", None).unwrap())
            .unwrap();
        let done = storage.set_task_state(task.id, TaskState::Done).unwrap();
        assert!(done.completed_at.is_some());
        let reopened = storage.set_task_state(task.id, TaskState::Backlog).unwrap();
        assert!(reopened.completed_at.is_none());
    }

    #[test]
    fn workspace_links_projects_and_apps_without_hiding_global_lists() {
        let (_directory, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engineering", "").unwrap())
            .unwrap();
        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        let app = storage
            .create_app(
                NewRegisteredApp::create(
                    "GitHub",
                    "",
                    Some(AppLaunchKind::Url),
                    Some("https://github.com"),
                )
                .unwrap(),
            )
            .unwrap();

        storage
            .set_project_workspace(project.id, workspace.id, true)
            .unwrap();
        storage
            .set_app_workspace(app.id, workspace.id, true)
            .unwrap();

        assert_eq!(
            storage
                .workspace_projects(workspace.id, false)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            storage.workspace_apps(workspace.id, false).unwrap().len(),
            1
        );
        assert_eq!(storage.projects(false).unwrap().len(), 1);
        assert_eq!(storage.apps(false).unwrap().len(), 1);

        storage
            .set_project_workspace(project.id, workspace.id, false)
            .unwrap();
        assert!(storage
            .workspace_projects(workspace.id, false)
            .unwrap()
            .is_empty());
    }
}
