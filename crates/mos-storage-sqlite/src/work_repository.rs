use std::collections::{HashMap, HashSet};

use mos_core::{
    validate_widget_id, AppId, Capture, CaptureId, CaptureRepository, CoreError, ErrorCode,
    HiddenWidget, LifecycleState, NewProject, NewTask, NewWorkspace, Project, ProjectId,
    RegisteredApp, SearchItem, SearchRequest, Task, TaskId, TaskState, WorkRepository, Workspace,
    WorkspaceId,
};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use time::OffsetDateTime;

use crate::{
    app_repository::{query_apps, APP_COLUMNS},
    map_lock_error, map_sql_error,
    repository::{
        ensure_changed, format_time, guard_deletable, parse_time, query_capture, to_fts_query,
        RawCapture, CAPTURE_COLUMNS,
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

    fn delete_task(&self, id: TaskId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        guard_deletable(&transaction, "tasks", &id.to_string(), "Task")?;
        delete_task_search(&transaction, id)?;
        transaction
            .execute("DELETE FROM tasks WHERE id = ?1", [id.to_string()])
            .map_err(map_sql_error)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    /// As Tasks do Project sobrevivem: `tasks.project_id` e ON DELETE SET NULL
    /// (0007_v03_design.sql:30). Apagar um Project nao pode levar trabalho junto
    /// — ele deixa de ter contexto, o que ja e perda suficiente.
    fn delete_project(&self, id: ProjectId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        guard_deletable(&transaction, "projects", &id.to_string(), "Project")?;
        delete_project_search(&transaction, id)?;
        transaction
            .execute("DELETE FROM projects WHERE id = ?1", [id.to_string()])
            .map_err(map_sql_error)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    /// Os vinculos caem por cascata declarada nas migrations: project_workspaces,
    /// app_workspaces, resource_workspaces e workspace_hidden_widgets. Nenhum
    /// Project, App ou Resource e apagado — some so a lente.
    fn delete_workspace(&self, id: WorkspaceId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        guard_deletable(&transaction, "workspaces", &id.to_string(), "Workspace")?;
        delete_workspace_search(&transaction, id)?;
        transaction
            .execute("DELETE FROM workspaces WHERE id = ?1", [id.to_string()])
            .map_err(map_sql_error)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn set_widget_hidden(
        &self,
        workspace_id: Option<WorkspaceId>,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError> {
        let widget_id = validate_widget_id(widget_id)?;
        let escopo = workspace_id.map(|id| id.to_string());
        let connection = self.connection.lock().map_err(map_lock_error)?;
        if hidden {
            let now = format_time(OffsetDateTime::now_utc())?;
            // `INSERT OR IGNORE` continua contando com a unicidade, que agora
            // vem do indice sobre `COALESCE(workspace_id, '')` e nao da PRIMARY
            // KEY (migration 0019). Sem esse indice, esconder o mesmo widget
            // duas vezes em "Todos" empilharia linhas em silencio: no SQLite,
            // NULL nunca colide com NULL.
            connection
                .execute(
                    "INSERT OR IGNORE INTO workspace_hidden_widgets (workspace_id, widget_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![escopo, widget_id, now],
                )
                .map_err(map_sql_error)?;
        } else {
            connection
                .execute(
                    "DELETE FROM workspace_hidden_widgets
                      WHERE COALESCE(workspace_id, '') = COALESCE(?1, '')
                        AND widget_id = ?2",
                    params![escopo, widget_id],
                )
                .map_err(map_sql_error)?;
        }
        Ok(())
    }

    /// Devolve todos os pares de uma vez. No teto sao sete linhas por Workspace,
    /// e uma chamada so deixa a troca de contexto na Home filtrar em memoria em
    /// vez de ir ao core a cada clique.
    fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT workspace_id, widget_id FROM workspace_hidden_widgets
                 ORDER BY COALESCE(workspace_id, ''), widget_id",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(map_sql_error)?;
        let mut hidden = Vec::new();
        for row in rows {
            let (workspace_id, widget_id) = row.map_err(map_sql_error)?;
            hidden.push(HiddenWidget {
                // Nulo e a visao "Todos", e nao um dado faltando (migration 0019).
                workspace_id: workspace_id.as_deref().map(WorkspaceId::parse).transpose()?,
                widget_id,
            });
        }
        Ok(hidden)
    }

    /// Devolve todas as posicoes de uma vez, pelo mesmo motivo de
    /// `hidden_widgets`: sao poucas linhas, e uma chamada so deixa a troca de
    /// contexto na Home filtrar em memoria em vez de ir ao core a cada clique.
    fn widget_placements(&self) -> Result<Vec<mos_core::WidgetPlacement>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT workspace_id, widget_id, position, section, span
                 FROM workspace_widget_layout
                 ORDER BY COALESCE(workspace_id, ''), position",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut found = Vec::new();
        for row in rows {
            let (workspace_id, widget_id, position, section, span) = row.map_err(map_sql_error)?;
            found.push(mos_core::WidgetPlacement {
                // Nulo e a visao "Todos", e nao um dado faltando (migration 0018).
                workspace_id: workspace_id.as_deref().map(WorkspaceId::parse).transpose()?,
                widget_id,
                position,
                section,
                span,
            });
        }
        Ok(found)
    }

    fn set_widget_layout(
        &self,
        workspace: Option<WorkspaceId>,
        placements: &[mos_core::WidgetPlacementInput],
    ) -> Result<Vec<mos_core::WidgetPlacement>, CoreError> {
        // Valida a lista INTEIRA antes de abrir a transacao: um id fora de
        // formato ou um span fora da grade no meio dela deixaria metade da
        // faixa gravada e metade nao.
        let entries: Vec<(String, i64, String, Option<i64>)> = placements
            .iter()
            .map(|entry| {
                Ok((
                    mos_core::validate_widget_id(&entry.widget_id)?,
                    entry.position,
                    mos_core::validate_section_id(&entry.section)?,
                    entry.span.map(mos_core::validate_span).transpose()?,
                ))
            })
            .collect::<Result<_, CoreError>>()?;

        let now = format_time(OffsetDateTime::now_utc())?;
        let escopo = workspace.map(|id| id.to_string());
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        for (id, position, section, span) in &entries {
            // Apaga e insere, em vez de `ON CONFLICT`. A unicidade agora vive
            // num indice sobre `COALESCE(workspace_id, '')` (migration 0018), e
            // um upsert teria de repetir essa expressao como alvo de conflito —
            // uma segunda copia da regra, num lugar onde escrever `workspace_id`
            // cru compilaria e silenciosamente pararia de casar as linhas de
            // "Todos". Dentro da transacao os dois passos sao um so.
            transaction
                .execute(
                    "DELETE FROM workspace_widget_layout
                      WHERE COALESCE(workspace_id, '') = COALESCE(?1, '')
                        AND widget_id = ?2",
                    params![escopo, id],
                )
                .map_err(map_sql_error)?;
            // Escrita autoritativa: o que chega e o que fica. Sem COALESCE nos
            // campos de proposito — com ele, `span: NULL` passaria a significar
            // "nao mexi" e nao haveria como desfazer um redimensionamento. Quem
            // monta a lista e responsavel por repassar o `span` ja guardado
            // quando esta so reordenando.
            transaction
                .execute(
                    "INSERT INTO workspace_widget_layout
                     (workspace_id, widget_id, position, section, span, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![escopo, id, position, section, span, now],
                )
                .map_err(map_sql_error)?;
        }

        transaction.commit().map_err(map_sql_error)?;
        drop(connection);
        self.widget_placements()
    }

    fn reset_widget_layout(
        &self,
        workspace: Option<WorkspaceId>,
    ) -> Result<Vec<mos_core::WidgetPlacement>, CoreError> {
        {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            connection
                .execute(
                    "DELETE FROM workspace_widget_layout
                      WHERE COALESCE(workspace_id, '') = COALESCE(?1, '')",
                    params![workspace.map(|id| id.to_string())],
                )
                .map_err(map_sql_error)?;
        }
        self.widget_placements()
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

    /// A regra de exclusao: nada ativo e apagado, e o vinculo cai junto sem
    /// levar o vinculado.
    #[test]
    fn delete_refuses_active_and_only_removes_the_lens() {
        let (_directory, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engineering", "").unwrap())
            .unwrap();
        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        storage
            .set_project_workspace(project.id, workspace.id, true)
            .unwrap();

        // Ativo recusa.
        assert!(storage.delete_workspace(workspace.id).is_err());
        assert_eq!(storage.workspaces(false).unwrap().len(), 1);

        // Arquivado aceita.
        storage
            .set_workspace_lifecycle(workspace.id, LifecycleState::Archived)
            .unwrap();
        storage.delete_workspace(workspace.id).unwrap();
        assert!(storage.workspaces(true).unwrap().is_empty());

        // O Project sobreviveu: sumiu a lente, nao o trabalho.
        assert_eq!(storage.projects(false).unwrap().len(), 1);
    }

    /// Apagar um Project nao pode levar Task junto — a FK e SET NULL, e a Task
    /// so perde o contexto.
    #[test]
    fn deleting_a_project_keeps_its_tasks() {
        let (_directory, storage) = storage();
        let project = storage
            .create_project(NewProject::create("Minarum", "", "").unwrap())
            .unwrap();
        let task = storage
            .create_task(NewTask::create("Refatorar navbar", "", Some(project.id)).unwrap())
            .unwrap();

        storage
            .set_project_lifecycle(project.id, LifecycleState::Archived)
            .unwrap();
        storage.delete_project(project.id).unwrap();

        let tasks = storage.tasks(false).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, task.id);
        assert_eq!(tasks[0].project_id, None);
    }

    #[test]
    fn hidden_widget_is_per_workspace_and_repeating_the_call_is_idempotent() {
        let (_directory, storage) = storage();
        let engineering = storage
            .create_workspace(NewWorkspace::create("Engineering", "").unwrap())
            .unwrap();
        let finance = storage
            .create_workspace(NewWorkspace::create("Finance", "").unwrap())
            .unwrap();

        storage
            .set_widget_hidden(Some(engineering.id), "inbox_pulse", true)
            .unwrap();
        storage
            .set_widget_hidden(Some(engineering.id), "inbox_pulse", true)
            .unwrap();

        let hidden = storage.hidden_widgets().unwrap();
        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].workspace_id, Some(engineering.id));
        assert_eq!(hidden[0].widget_id, "inbox_pulse");

        storage
            .set_widget_hidden(Some(engineering.id), "inbox_pulse", false)
            .unwrap();
        storage
            .set_widget_hidden(Some(engineering.id), "inbox_pulse", false)
            .unwrap();
        assert!(storage.hidden_widgets().unwrap().is_empty());

        storage
            .set_widget_hidden(Some(finance.id), "system_health", true)
            .unwrap();
        let hidden = storage.hidden_widgets().unwrap();
        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].workspace_id, Some(finance.id));
    }

    /// A Home sem Workspace tambem esconde widget. Antes da 0019 nao havia onde
    /// gravar, e quem nunca criou Workspace nenhum ficava sem a feature.
    #[test]
    fn the_home_without_a_workspace_hides_its_own_widgets() {
        let (_directory, storage) = storage();

        storage.set_widget_hidden(None, "inbox_pulse", true).unwrap();
        let hidden = storage.hidden_widgets().unwrap();

        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].workspace_id, None, "nao pertence a Workspace nenhum");
        assert_eq!(hidden[0].widget_id, "inbox_pulse");

        storage.set_widget_hidden(None, "inbox_pulse", false).unwrap();
        assert!(storage.hidden_widgets().unwrap().is_empty(), "e volta a aparecer");
    }

    /// A armadilha que o indice unico da 0019 fecha: no SQLite, coluna de
    /// PRIMARY KEY aceita NULL e NULL nunca colide com NULL. Sem o indice, o
    /// `INSERT OR IGNORE` nao teria com o que conflitar e cada clique em
    /// "ocultar" empilharia mais uma linha para "Todos".
    #[test]
    fn hiding_twice_without_a_workspace_does_not_pile_up_rows() {
        let (_directory, storage) = storage();

        for _ in 0..3 {
            storage.set_widget_hidden(None, "system_health", true).unwrap();
        }

        assert_eq!(storage.hidden_widgets().unwrap().len(), 1, "uma linha, nao tres");
    }

    /// "Todos" e um escopo como outro qualquer: esconder la nao esconde no
    /// Workspace, nem o contrario.
    #[test]
    fn hiding_in_one_scope_does_not_hide_in_the_other() {
        let (_directory, storage) = storage();
        let estudio = storage
            .create_workspace(NewWorkspace::create("Estudio", "").unwrap())
            .unwrap();

        storage.set_widget_hidden(None, "timer", true).unwrap();
        storage
            .set_widget_hidden(Some(estudio.id), "system_health", true)
            .unwrap();

        let hidden = storage.hidden_widgets().unwrap();
        let de = |escopo: Option<WorkspaceId>| {
            hidden
                .iter()
                .filter(|h| h.workspace_id == escopo)
                .map(|h| h.widget_id.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(de(None), ["timer"]);
        assert_eq!(de(Some(estudio.id)), ["system_health"]);
    }

    /// Apagar um Workspace leva as escolhas DELE. As de "Todos" nao pertencem a
    /// Workspace nenhum, e por isso nao morrem com nenhum.
    #[test]
    fn deleting_a_workspace_leaves_the_workspaceless_choices_alone() {
        let (_directory, storage) = storage();
        let efemero = storage
            .create_workspace(NewWorkspace::create("Efemero", "").unwrap())
            .unwrap();

        storage.set_widget_hidden(None, "timer", true).unwrap();
        storage
            .set_widget_hidden(Some(efemero.id), "timer", true)
            .unwrap();

        storage
            .set_workspace_lifecycle(efemero.id, LifecycleState::Archived)
            .unwrap();
        storage.delete_workspace(efemero.id).unwrap();

        let restante = storage.hidden_widgets().unwrap();
        assert_eq!(restante.len(), 1);
        assert_eq!(restante[0].workspace_id, None);
    }

    #[test]
    fn widget_id_outside_the_allowed_shape_is_refused() {
        let (_directory, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engineering", "").unwrap())
            .unwrap();

        for invalid in ["", "  ", "Inbox Pulse", "inbox-pulse", "1inbox"] {
            assert!(
                storage
                    .set_widget_hidden(Some(workspace.id), invalid, true)
                    .is_err(),
                "aceitou o id invalido {invalid:?}"
            );
        }
        assert!(storage.hidden_widgets().unwrap().is_empty());
    }

    /// Nao existe delete de Workspace no produto — arquivar e o caminho. O DELETE
    /// cru aqui prova que a FK esta ativa: se `foreign_keys=ON` se perder em
    /// `configure_connection` (lib.rs:103), a linha sobrevive e este teste falha.
    #[test]
    fn deleting_the_workspace_takes_its_hidden_widgets() {
        let (_directory, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engineering", "").unwrap())
            .unwrap();
        storage
            .set_widget_hidden(Some(workspace.id), "system_health", true)
            .unwrap();

        storage
            .connection
            .lock()
            .unwrap()
            .execute(
                "DELETE FROM workspaces WHERE id = ?1",
                params![workspace.id.to_string()],
            )
            .unwrap();

        assert!(storage.hidden_widgets().unwrap().is_empty());
    }

    // ----------------------------------------------------- arranjo dos widgets

    /// A escrita mais comum: uma faixa inteira, na ordem nova, com a largura
    /// de todo mundo no que o desenho escolheu.
    fn ordem(ids: &[&str]) -> Vec<mos_core::WidgetPlacementInput> {
        faixa("agora", ids)
    }

    fn faixa(section: &str, ids: &[&str]) -> Vec<mos_core::WidgetPlacementInput> {
        ids.iter()
            .enumerate()
            .map(|(position, id)| mos_core::WidgetPlacementInput {
                widget_id: (*id).to_owned(),
                position: position as i64,
                section: section.to_owned(),
                span: None,
            })
            .collect()
    }

    /// Sem nenhuma escrita a tabela fica vazia — e a inversao herdada da 0008:
    /// quem nunca arrastou nada nao gera linha nenhuma.
    #[test]
    fn a_home_never_arranged_stores_nothing() {
        let (_guard, storage) = storage();
        assert!(storage.widget_placements().unwrap().is_empty());
    }

    #[test]
    fn the_saved_order_comes_back_in_order() {
        let (_guard, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Web Design", "").unwrap())
            .unwrap();

        let saved = storage
            .set_widget_layout(Some(workspace.id), &ordem(&["inbox_pulse", "timer", "now"]))
            .unwrap();

        assert_eq!(saved.len(), 3);
        assert_eq!(
            saved.iter().map(|p| p.widget_id.as_str()).collect::<Vec<_>>(),
            ["inbox_pulse", "timer", "now"]
        );
        assert_eq!(saved[0].position, 0);
        assert_eq!(saved[2].position, 2);
    }

    /// Arrastar de novo reescreve, e nao acumula: sem o `ON CONFLICT` cada
    /// arrasto deixaria uma linha morta com a posicao antiga.
    #[test]
    fn arranging_twice_replaces_instead_of_piling_up() {
        let (_guard, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Engenharia", "").unwrap())
            .unwrap();

        storage
            .set_widget_layout(Some(workspace.id), &ordem(&["timer", "now"]))
            .unwrap();
        let saved = storage
            .set_widget_layout(Some(workspace.id), &ordem(&["now", "timer"]))
            .unwrap();

        assert_eq!(saved.len(), 2, "duas linhas, nao quatro");
        assert_eq!(saved[0].widget_id, "now");
        assert_eq!(saved[1].widget_id, "timer");
    }

    /// Cada Workspace arruma a propria Home. Sem isso, arrumar em um bagunçaria
    /// o outro — que e o oposto do que Workspace significa.
    #[test]
    fn each_workspace_arranges_its_own_home() {
        let (_guard, storage) = storage();
        let design = storage
            .create_workspace(NewWorkspace::create("Design", "").unwrap())
            .unwrap();
        let financas = storage
            .create_workspace(NewWorkspace::create("Financas", "").unwrap())
            .unwrap();

        storage
            .set_widget_layout(Some(design.id), &ordem(&["timer", "now"]))
            .unwrap();
        storage
            .set_widget_layout(Some(financas.id), &ordem(&["now", "timer"]))
            .unwrap();

        let all = storage.widget_placements().unwrap();
        let of = |id: WorkspaceId| {
            all.iter()
                .filter(|p| p.workspace_id == Some(id))
                .map(|p| p.widget_id.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(of(design.id), ["timer", "now"]);
        assert_eq!(of(financas.id), ["now", "timer"]);
    }

    /// Id fora de formato e recusado ANTES de abrir a transacao. Recusar no
    /// meio deixaria metade da secao gravada e metade nao.
    #[test]
    fn a_malformed_id_is_refused_without_writing_anything() {
        let (_guard, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Testes", "").unwrap())
            .unwrap();

        let error = storage
            .set_widget_layout(Some(workspace.id), &ordem(&["timer", "NAO PODE", "now"]))
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(
            storage.widget_placements().unwrap().is_empty(),
            "nem o `timer`, que vinha antes do invalido, foi gravado"
        );
    }

    /// Apagar o Workspace leva a arrumacao junto: ela so significa alguma coisa
    /// dentro dele. E o mesmo `ON DELETE CASCADE` da tabela de ocultos.
    #[test]
    fn deleting_the_workspace_takes_its_arrangement() {
        let (_guard, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Efemero", "").unwrap())
            .unwrap();
        storage
            .set_widget_layout(Some(workspace.id), &ordem(&["timer"]))
            .unwrap();
        assert_eq!(storage.widget_placements().unwrap().len(), 1);

        // Arquivar antes: o dominio recusa excluir Workspace ativo, e essa
        // regra e do produto, nao um detalhe do teste.
        storage
            .set_workspace_lifecycle(workspace.id, LifecycleState::Archived)
            .unwrap();
        storage.delete_workspace(workspace.id).unwrap();
        assert!(storage.widget_placements().unwrap().is_empty());
    }

    /// O que o domínio faz com o que o banco devolve: a ponta a ponta da
    /// feature, sem interface.
    #[test]
    fn the_stored_order_drives_the_catalog() {
        let (_guard, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Web", "").unwrap())
            .unwrap();

        let catalog: Vec<String> = ["timer", "now", "today_hours"]
            .iter()
            .map(|id| (*id).to_owned())
            .collect();

        assert_eq!(
            mos_core::order_widgets(&catalog, &storage.widget_placements().unwrap()),
            catalog,
            "sem arrumacao, a ordem e a do catalogo"
        );

        storage
            .set_widget_layout(Some(workspace.id), &ordem(&["today_hours", "timer"]))
            .unwrap();

        assert_eq!(
            mos_core::order_widgets(&catalog, &storage.widget_placements().unwrap()),
            ["today_hours", "timer", "now"],
            "o nao arrumado cai para o fim"
        );
    }

    /// A escrita e autoritativa: campo por campo, o que chega e o que fica.
    /// `span: None` e como se desfaz um redimensionamento — se ele significasse
    /// "nao mexi", voltar a largura do desenho seria impossivel sem apagar o
    /// arranjo inteiro.
    #[test]
    fn writing_is_authoritative_field_by_field() {
        let (_guard, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Estudio", "").unwrap())
            .unwrap();

        storage
            .set_widget_layout(
                Some(workspace.id),
                &[mos_core::WidgetPlacementInput {
                    widget_id: "timer".to_owned(),
                    position: 0,
                    section: "overview".to_owned(),
                    span: Some(12),
                }],
            )
            .unwrap();

        let mudou_de_faixa = storage
            .set_widget_layout(Some(workspace.id), &faixa("agora", &["now", "timer"]))
            .unwrap();
        let timer = mudou_de_faixa.iter().find(|p| p.widget_id == "timer").unwrap();
        assert_eq!(timer.position, 1);
        assert_eq!(timer.section.as_deref(), Some("agora"), "voltou de faixa");
        assert_eq!(timer.span, None, "e a largura voltou ao desenho");
    }

    /// O contrato que o front tem de honrar, escrito como teste para ficar
    /// visivel: quem so reordena PRECISA repassar o `span` ja guardado. E o
    /// preco de nao ter COALESCE, e o unico jeito de errar aqui.
    #[test]
    fn reordering_must_carry_the_stored_width_along() {
        let (_guard, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Estudio", "").unwrap())
            .unwrap();

        storage
            .set_widget_layout(
                Some(workspace.id),
                &[mos_core::WidgetPlacementInput {
                    widget_id: "timer".to_owned(),
                    position: 0,
                    section: "agora".to_owned(),
                    span: Some(12),
                }],
            )
            .unwrap();

        // A reordenacao repassa o que ja estava guardado, em vez de mandar None.
        let guardado = storage.widget_placements().unwrap();
        let span_de = |id: &str| {
            guardado
                .iter()
                .find(|p| p.widget_id == id)
                .and_then(|p| p.span)
        };
        let saved = storage
            .set_widget_layout(
                Some(workspace.id),
                &["now", "timer"]
                    .iter()
                    .enumerate()
                    .map(|(position, id)| mos_core::WidgetPlacementInput {
                        widget_id: (*id).to_owned(),
                        position: position as i64,
                        section: "agora".to_owned(),
                        span: span_de(id),
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap();

        let timer = saved.iter().find(|p| p.widget_id == "timer").unwrap();
        assert_eq!(timer.position, 1, "a ordem nova valeu");
        assert_eq!(timer.span, Some(12), "e a largura escolhida sobreviveu");
    }

    /// Largura fora da grade e recusada antes da transacao, igual ao id.
    #[test]
    fn a_span_outside_the_grid_is_refused_without_writing_anything() {
        let (_guard, storage) = storage();
        let workspace = storage
            .create_workspace(NewWorkspace::create("Testes", "").unwrap())
            .unwrap();

        let error = storage
            .set_widget_layout(
                Some(workspace.id),
                &[
                    mos_core::WidgetPlacementInput {
                        widget_id: "timer".to_owned(),
                        position: 0,
                        section: "agora".to_owned(),
                        span: None,
                    },
                    mos_core::WidgetPlacementInput {
                        widget_id: "now".to_owned(),
                        position: 1,
                        section: "agora".to_owned(),
                        span: Some(99),
                    },
                ],
            )
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(
            storage.widget_placements().unwrap().is_empty(),
            "nem o `timer`, que vinha antes do invalido, foi gravado"
        );
    }

    /// Voltar ao desenho APAGA as linhas, e nao grava o catalogo por cima.
    /// Gravar petrificaria o desenho de hoje — o oposto do que a inversao faz.
    #[test]
    fn restoring_the_design_deletes_the_rows_instead_of_writing_them() {
        let (_guard, storage) = storage();
        let meu = storage
            .create_workspace(NewWorkspace::create("Meu", "").unwrap())
            .unwrap();
        let outro = storage
            .create_workspace(NewWorkspace::create("Outro", "").unwrap())
            .unwrap();

        storage
            .set_widget_layout(Some(meu.id), &ordem(&["timer", "now"]))
            .unwrap();
        storage
            .set_widget_layout(Some(outro.id), &ordem(&["now", "timer"]))
            .unwrap();

        let restante = storage.reset_widget_layout(Some(meu.id)).unwrap();

        assert!(
            !restante.iter().any(|p| p.workspace_id == Some(meu.id)),
            "o arranjo do Workspace some por inteiro"
        );
        assert_eq!(
            restante.iter().filter(|p| p.workspace_id == Some(outro.id)).count(),
            2,
            "e o do vizinho fica intacto"
        );
    }

    // ------------------------------------------------- a Home sem Workspace

    /// A visao "Todos" arruma a propria Home. Antes da 0018 ela nao tinha onde
    /// gravar, e quem nunca criou Workspace nenhum — o estado de quem instala e
    /// comeca a usar — ficava sem a feature inteira.
    #[test]
    fn the_home_without_a_workspace_arranges_itself() {
        let (_guard, storage) = storage();

        let saved = storage
            .set_widget_layout(None, &ordem(&["today_hours", "timer"]))
            .unwrap();

        assert_eq!(saved.len(), 2);
        assert!(
            saved.iter().all(|p| p.workspace_id.is_none()),
            "o arranjo de \"Todos\" nao pertence a Workspace nenhum"
        );
        assert_eq!(saved[0].widget_id, "today_hours");
    }

    /// O teste que a migration 0018 existe para poder passar, e o que mais
    /// facilmente passaria despercebido: no SQLite, coluna de PRIMARY KEY
    /// aceita NULL, e NULL nunca colide com NULL. Sem o indice unico sobre
    /// `COALESCE(workspace_id, '')`, arrumar "Todos" duas vezes empilharia
    /// linhas em vez de substitui-las, e o arranjo viraria lixo silencioso.
    #[test]
    fn arranging_the_workspaceless_home_twice_replaces_instead_of_piling_up() {
        let (_guard, storage) = storage();

        storage
            .set_widget_layout(None, &ordem(&["timer", "now"]))
            .unwrap();
        let saved = storage
            .set_widget_layout(None, &ordem(&["now", "timer"]))
            .unwrap();

        assert_eq!(saved.len(), 2, "duas linhas, nao quatro");
        assert_eq!(saved[0].widget_id, "now");
        assert_eq!(saved[1].widget_id, "timer");
    }

    /// "Todos" e um escopo como outro qualquer: o que se arruma la nao vaza
    /// para um Workspace, nem o contrario.
    #[test]
    fn the_workspaceless_home_and_a_workspace_do_not_mix() {
        let (_guard, storage) = storage();
        let estudio = storage
            .create_workspace(NewWorkspace::create("Estudio", "").unwrap())
            .unwrap();

        storage
            .set_widget_layout(None, &ordem(&["timer", "now"]))
            .unwrap();
        let todos = storage
            .set_widget_layout(Some(estudio.id), &ordem(&["now", "timer"]))
            .unwrap();

        let de = |escopo: Option<WorkspaceId>| {
            todos
                .iter()
                .filter(|p| p.workspace_id == escopo)
                .map(|p| p.widget_id.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(de(None), ["timer", "now"]);
        assert_eq!(de(Some(estudio.id)), ["now", "timer"]);
    }

    /// Apagar um Workspace leva o arranjo DELE. O de "Todos" nao pertence a
    /// Workspace nenhum, e por isso nao morre com nenhum — e o que o `NULL`
    /// numa chave estrangeira significa, e nao um efeito colateral.
    #[test]
    fn deleting_a_workspace_leaves_the_workspaceless_home_alone() {
        let (_guard, storage) = storage();
        let efemero = storage
            .create_workspace(NewWorkspace::create("Efemero", "").unwrap())
            .unwrap();

        storage
            .set_widget_layout(None, &ordem(&["timer", "now"]))
            .unwrap();
        storage
            .set_widget_layout(Some(efemero.id), &ordem(&["now"]))
            .unwrap();

        storage
            .set_workspace_lifecycle(efemero.id, LifecycleState::Archived)
            .unwrap();
        storage.delete_workspace(efemero.id).unwrap();

        let restante = storage.widget_placements().unwrap();
        assert_eq!(restante.len(), 2, "so o arranjo do Workspace foi embora");
        assert!(restante.iter().all(|p| p.workspace_id.is_none()));
    }

    /// E restaurar o desenho de um escopo nao mexe no outro.
    #[test]
    fn restoring_one_scope_leaves_the_other_untouched() {
        let (_guard, storage) = storage();
        let estudio = storage
            .create_workspace(NewWorkspace::create("Estudio", "").unwrap())
            .unwrap();

        storage
            .set_widget_layout(None, &ordem(&["timer", "now"]))
            .unwrap();
        storage
            .set_widget_layout(Some(estudio.id), &ordem(&["now", "timer"]))
            .unwrap();

        let apos_todos = storage.reset_widget_layout(None).unwrap();
        assert!(apos_todos.iter().all(|p| p.workspace_id == Some(estudio.id)));
        assert_eq!(apos_todos.len(), 2);

        let apos_estudio = storage.reset_widget_layout(Some(estudio.id)).unwrap();
        assert!(apos_estudio.is_empty());
    }

    /// A carga EXATA que a Home manda ao alargar um widget em "Todos": a faixa
    /// inteira, com a largura escolhida em um e `None` nos vizinhos.
    #[test]
    fn the_payload_the_home_sends_when_resizing_goes_through() {
        let (_guard, storage) = storage();
        let carga = [("now", Some(8)), ("timer", None), ("today_hours", None)]
            .iter()
            .enumerate()
            .map(|(position, (id, span))| mos_core::WidgetPlacementInput {
                widget_id: (*id).to_owned(),
                position: position as i64,
                section: "now".to_owned(),
                span: *span,
            })
            .collect::<Vec<_>>();

        let saved = storage.set_widget_layout(None, &carga).unwrap();
        assert_eq!(saved.len(), 3);
        assert_eq!(
            saved.iter().find(|p| p.widget_id == "now").and_then(|p| p.span),
            Some(8)
        );
    }
}
