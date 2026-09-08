use std::collections::{HashMap, HashSet};

use mos_core::{
    validate_widget_id, AppId, AttentionRepository, Capture, CaptureId, CaptureRepository,
    ChecklistItem, ChecklistItemId, CoreError, EditTask, ErrorCode, HiddenWidget, LifecycleState,
    NewChecklistItem, NewProject, NewReminder, NewTask, NewWorkspace, Priority, Project, ProjectId,
    RegisteredApp, Reminder, Resource, ResourceId, SearchItem, SearchRequest, Task, TaskDetail,
    TaskId, TaskState, WorkRepository, Workspace, WorkspaceId,
};
use rusqlite::{params, OptionalExtension, Row, Transaction};
use time::OffsetDateTime;

use crate::{
    app_repository::{query_apps, APP_COLUMNS},
    attention_repository::insert_reminder,
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
pub(crate) const TASK_COLUMNS: &str = "id, title, description, project_id, source_capture_id, work_state, lifecycle_state, due_at, priority, estimate_minutes, parent_task_id, blocked_by_task_id, waiting_for, follow_up_at, created_at, updated_at, completed_at";

pub(crate) const CHECKLIST_COLUMNS: &str =
    "id, task_id, label, position, completed_at, created_at, updated_at";

/// A lista de colunas da Task, com apelido, MAIS as duas contagens do checklist.
///
/// # Por que subconsulta, e nao uma segunda ida ao banco
///
/// O card do Kanban mostra `4/7` e uma barra. Buscar isso por Task seria um
/// `SELECT` por cartao — o N+1 classico, e num quadro de sessenta Tasks sao
/// sessenta consultas para desenhar uma tela. As duas correlacionadas aqui
/// resolvem tudo numa consulta so, e as duas caem no indice
/// `task_checklist_order`, que comeca por `task_id`.
///
/// # Por que aqui, e nao um JOIN com GROUP BY
///
/// Um `LEFT JOIN ... GROUP BY` obrigaria toda consulta de Task a agrupar por
/// dezessete colunas, e a primeira que esquecesse uma delas devolveria linha
/// duplicada em silencio. A subconsulta e local: quem escreve SQL de Task nao
/// precisa saber que checklist existe.
///
/// Item `trashed` nao conta. Um item apagado no celular chega aqui como
/// `lifecycle_state = 'trashed'` (o apagamento do sync e logico), e conta-lo
/// faria a barra do card encolher sozinha no outro PC.
pub(crate) fn task_select(alias: &str) -> String {
    let colunas = TASK_COLUMNS
        .split(", ")
        .map(|coluna| format!("{alias}.{coluna}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{colunas}, \
         (SELECT COUNT(*) FROM task_checklist_items ci \
           WHERE ci.task_id = {alias}.id AND ci.lifecycle_state = 'active'), \
         (SELECT COUNT(*) FROM task_checklist_items ci \
           WHERE ci.task_id = {alias}.id AND ci.lifecycle_state = 'active' \
             AND ci.completed_at IS NOT NULL)"
    )
}

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
    due_at: Option<String>,
    priority: String,
    estimate_minutes: Option<i64>,
    parent_task_id: Option<String>,
    blocked_by_task_id: Option<String>,
    waiting_for: String,
    follow_up_at: Option<String>,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
    checklist_total: i64,
    checklist_done: i64,
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
            due_at: row.get(7)?,
            priority: row.get(8)?,
            estimate_minutes: row.get(9)?,
            parent_task_id: row.get(10)?,
            blocked_by_task_id: row.get(11)?,
            waiting_for: row.get(12)?,
            follow_up_at: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
            completed_at: row.get(16)?,
            checklist_total: row.get(17)?,
            checklist_done: row.get(18)?,
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
            due_at: self.due_at.as_deref().map(parse_time).transpose()?,
            priority: Priority::parse(&self.priority)?,
            estimate_minutes: self.estimate_minutes,
            parent_task_id: self
                .parent_task_id
                .as_deref()
                .map(TaskId::parse)
                .transpose()?,
            blocked_by_task_id: self
                .blocked_by_task_id
                .as_deref()
                .map(TaskId::parse)
                .transpose()?,
            waiting_for: self.waiting_for,
            follow_up_at: self.follow_up_at.as_deref().map(parse_time).transpose()?,
            checklist_total: self.checklist_total.max(0) as usize,
            checklist_done: self.checklist_done.max(0) as usize,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
            completed_at: self.completed_at.as_deref().map(parse_time).transpose()?,
        })
    }
}

struct RawChecklistItem {
    id: String,
    task_id: String,
    label: String,
    position: i64,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl RawChecklistItem {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            task_id: row.get(1)?,
            label: row.get(2)?,
            position: row.get(3)?,
            completed_at: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }

    fn into_item(self) -> Result<ChecklistItem, CoreError> {
        Ok(ChecklistItem {
            id: ChecklistItemId::parse(&self.id)?,
            task_id: TaskId::parse(&self.task_id)?,
            label: self.label,
            position: self.position,
            completed_at: self.completed_at.as_deref().map(parse_time).transpose()?,
            created_at: parse_time(&self.created_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

impl WorkRepository for SqliteStorage {
    fn create_workspace(&self, workspace: NewWorkspace) -> Result<Workspace, CoreError> {
        let now = format_time(workspace.created_at)?;
        let connection = self.escrita()?;
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
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("workspace", workspace.id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    ("name".to_owned(), serde_json::json!(workspace.name)),
                    (
                        "description".to_owned(),
                        serde_json::json!(workspace.description),
                    ),
                    ("createdAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;
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
        let connection = self.escrita()?;
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
        self.emitir_update(
            &transaction,
            "workspace",
            id.as_uuid(),
            &[
                ("name", serde_json::json!(name)),
                ("description", serde_json::json!(description)),
            ],
        )?;
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
        let connection = self.escrita()?;
        // Transacao, e nao `execute` solto como era antes: a operacao de
        // sincronizacao precisa entrar JUNTO com a mudanca. Arquivar e falhar ao
        // enfileirar deixaria um Workspace arquivado que nunca sai deste
        // aparelho, sem ninguem ficar sabendo.
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE workspaces SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3
                 WHERE id = ?4",
                params![lifecycle.as_str(), now, archived_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        self.emitir_update(
            &transaction,
            "workspace",
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(lifecycle.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
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
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        if linked {
            let now = format_time(OffsetDateTime::now_utc())?;
            transaction
                .execute(
                    "INSERT OR IGNORE INTO project_workspaces (project_id, workspace_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![project_id.to_string(), workspace_id.to_string(), now],
                )
                .map_err(map_sql_error)?;
        } else {
            transaction
                .execute(
                    "DELETE FROM project_workspaces WHERE project_id = ?1 AND workspace_id = ?2",
                    params![project_id.to_string(), workspace_id.to_string()],
                )
                .map_err(map_sql_error)?;
        }
        self.emitir_relacao(
            &transaction,
            "projectWorkspace",
            project_id.as_uuid(),
            workspace_id.as_uuid(),
            linked,
        )?;
        transaction.commit().map_err(map_sql_error)?;
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
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        guard_deletable(&transaction, "tasks", &id.to_string(), "Task")?;
        delete_task_search(&transaction, id)?;
        // O checklist cai por CASCADE (migration 0039), e o indice dele NAO —
        // um FTS nao tem chave estrangeira. Sem esta linha ficariam linhas de
        // indice apontando para rowids que nao existem mais, e a busca acharia
        // passos de uma Task apagada.
        {
            let mut consulta = transaction
                .prepare("SELECT rowid FROM task_checklist_items WHERE task_id = ?1")
                .map_err(map_sql_error)?;
            let rowids: Vec<i64> = consulta
                .query_map([id.to_string()], |linha| linha.get(0))
                .map_err(map_sql_error)?
                .collect::<Result<_, _>>()
                .map_err(map_sql_error)?;
            for rowid in rowids {
                crate::repository::tirar_do_indice(
                    &transaction,
                    "task_checklist_search",
                    "task_checklist_items",
                    &["label"],
                    rowid,
                )?;
            }
        }
        transaction
            .execute("DELETE FROM tasks WHERE id = ?1", [id.to_string()])
            .map_err(map_sql_error)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("task", id.as_uuid()),
            mos_sync::OpBody::Delete,
        )?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    /// As Tasks do Project sobrevivem: `tasks.project_id` e ON DELETE SET NULL
    /// (0007_v03_design.sql:30). Apagar um Project nao pode levar trabalho junto
    /// — ele deixa de ter contexto, o que ja e perda suficiente.
    fn delete_project(&self, id: ProjectId) -> Result<(), CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        guard_deletable(&transaction, "projects", &id.to_string(), "Project")?;
        delete_project_search(&transaction, id)?;
        transaction
            .execute("DELETE FROM projects WHERE id = ?1", [id.to_string()])
            .map_err(map_sql_error)?;
        // As Tasks do Project sobrevivem aqui e sobrevivem no outro dispositivo:
        // o `project_id` delas vira NULL pela FK, e a operacao que viaja e a
        // exclusao do Project — nunca a das Tasks.
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("project", id.as_uuid()),
            mos_sync::OpBody::Delete,
        )?;
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
                workspace_id: workspace_id
                    .as_deref()
                    .map(WorkspaceId::parse)
                    .transpose()?,
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
                workspace_id: workspace_id
                    .as_deref()
                    .map(WorkspaceId::parse)
                    .transpose()?,
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

    /// Todas as petalas de uma vez, pelo mesmo motivo de `widget_placements`:
    /// sao poucas linhas, e uma chamada so deixa a troca de Workspace filtrar
    /// em memoria em vez de ir ao core a cada clique.
    fn radial_pins(&self) -> Result<Vec<mos_core::RadialPin>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT workspace_id, slot, kind, target
                 FROM radial_pins
                 ORDER BY COALESCE(workspace_id, ''), slot",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut found = Vec::new();
        for row in rows {
            let (workspace_id, slot, kind, target) = row.map_err(map_sql_error)?;
            found.push(mos_core::RadialPin {
                // Nulo e a visao "Todos", e nao um dado faltando (migration 0021).
                workspace_id: workspace_id
                    .as_deref()
                    .map(WorkspaceId::parse)
                    .transpose()?,
                slot,
                kind,
                target,
            });
        }
        Ok(found)
    }

    fn set_radial_pin(
        &self,
        workspace: Option<WorkspaceId>,
        pin: mos_core::RadialPinInput,
    ) -> Result<Vec<mos_core::RadialPin>, CoreError> {
        // Valida ANTES de tocar no banco, como `set_widget_layout` faz.
        let kind = mos_core::validate_pin_kind(&pin.kind)?;
        let target = pin.target.trim().to_owned();
        if target.is_empty() {
            return Err(CoreError::new(
                mos_core::ErrorCode::InvalidInput,
                "Petala sem alvo.",
                false,
            ));
        }

        {
            let now = format_time(OffsetDateTime::now_utc())?;
            let escopo = workspace.map(|id| id.to_string());
            let connection = self.connection.lock().map_err(map_lock_error)?;
            let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
            // Apaga e insere, e nao `ON CONFLICT`, pela mesma razao do
            // `set_widget_layout`: a unicidade vive num indice sobre
            // `COALESCE(workspace_id, '')`, e um upsert teria de repetir essa
            // expressao — uma segunda copia da regra, num lugar onde escrever
            // `workspace_id` cru compila e silenciosamente para de casar as
            // linhas de "Todos".
            transaction
                .execute(
                    "DELETE FROM radial_pins
                      WHERE COALESCE(workspace_id, '') = COALESCE(?1, '') AND slot = ?2",
                    params![escopo, pin.slot],
                )
                .map_err(map_sql_error)?;
            transaction
                .execute(
                    "INSERT INTO radial_pins (workspace_id, slot, kind, target, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![escopo, pin.slot, kind, target, now],
                )
                .map_err(map_sql_error)?;
            transaction.commit().map_err(map_sql_error)?;
        }
        self.radial_pins()
    }

    fn clear_radial_pin(
        &self,
        workspace: Option<WorkspaceId>,
        slot: i64,
    ) -> Result<Vec<mos_core::RadialPin>, CoreError> {
        {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            // APAGA a linha em vez de gravar um alvo vazio: e a inversao da
            // 0021, e e o que faz o slot voltar a seguir o padrao em vez de
            // congelar no padrao de hoje.
            connection
                .execute(
                    "DELETE FROM radial_pins
                      WHERE COALESCE(workspace_id, '') = COALESCE(?1, '') AND slot = ?2",
                    params![workspace.map(|id| id.to_string()), slot],
                )
                .map_err(map_sql_error)?;
        }
        self.radial_pins()
    }

    fn create_project(&self, project: NewProject) -> Result<Project, CoreError> {
        let now = format_time(project.created_at)?;
        let connection = self.escrita()?;
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
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("project", project.id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    ("name".to_owned(), serde_json::json!(project.name)),
                    (
                        "description".to_owned(),
                        serde_json::json!(project.description),
                    ),
                    (
                        "repository".to_owned(),
                        serde_json::json!(project.repository),
                    ),
                    ("createdAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;
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
        let connection = self.escrita()?;
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
        self.emitir_update(
            &transaction,
            "project",
            id.as_uuid(),
            &[
                ("name", serde_json::json!(name)),
                ("description", serde_json::json!(description)),
                ("repository", serde_json::json!(repository)),
            ],
        )?;
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
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE projects SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3
                 WHERE id = ?4",
                params![lifecycle.as_str(), now, archived_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        self.emitir_update(
            &transaction,
            "project",
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(lifecycle.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        query_project(&connection, id)
    }

    fn create_task(&self, task: NewTask) -> Result<Task, CoreError> {
        let id = task.id;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        insert_task(self, &transaction, task, None)?;
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, id)
    }

    fn create_task_from_capture(
        &self,
        capture_id: CaptureId,
        task: NewTask,
    ) -> Result<Task, CoreError> {
        let id = task.id;
        let connection = self.escrita()?;
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
        insert_task(self, &transaction, task, Some(capture_id))?;
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

    fn create_task_from_capture_with_reminder(
        &self,
        capture_id: CaptureId,
        task: NewTask,
        reminder: Option<NewReminder>,
    ) -> Result<(Task, Option<Reminder>), CoreError> {
        let task_id = task.id;
        let reminder_id = reminder.as_ref().map(|draft| draft.id);
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        // A mesma guarda de `create_task_from_capture`: uma Capture tem no
        // maximo uma Task derivada, e o UNIQUE da coluna ja a imporia — mas o
        // erro dele fala de constraint, e este fala de produto.
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

        insert_task(self, &transaction, task, Some(capture_id))?;

        // O Reminder aponta para a TASK, e nao para a Capture. Quando ele tocar
        // amanha as nove, "o que eu tenho de fazer?" precisa de resposta sem
        // passar pela Inbox.
        if let Some(draft) = reminder {
            let draft = draft.with_target(mos_core::ReminderTarget::Task(task_id));
            insert_reminder(&transaction, &draft)?;
        }

        let changed = transaction
            .execute(
                "UPDATE captures SET processing_state = 'processed', updated_at = ?1
                 WHERE id = ?2 AND processing_state = 'inbox' AND lifecycle_state = 'active'",
                params![
                    format_time(OffsetDateTime::now_utc())?,
                    capture_id.to_string()
                ],
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
        let task = query_task(&connection, task_id)?;
        drop(connection);
        let reminder = reminder_id.map(|id| self.reminder(id)).transpose()?;
        Ok((task, reminder))
    }

    fn update_task(&self, id: TaskId, edit: EditTask) -> Result<Task, CoreError> {
        let edit = edit.validate(id)?;
        let now = format_time(OffsetDateTime::now_utc())?;
        let prazo = edit.due_at.map(format_time).transpose()?;
        let cobranca = edit.follow_up_at.map(format_time).transpose()?;
        let projeto = edit.project_id.map(|valor| valor.to_string());
        let pai = edit.parent_task_id.map(|valor| valor.to_string());
        let travada = edit.blocked_by_task_id.map(|valor| valor.to_string());
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        // Lido ANTES da escrita: e a unica hora em que o valor antigo existe, e
        // sem ele nao ha diff para emitir.
        let antes = query_task(&transaction, id)?;
        delete_task_search(&transaction, id)?;
        let changed = transaction
            .execute(
                "UPDATE tasks SET title = ?1, description = ?2, project_id = ?3, due_at = ?4,
                        priority = ?5, estimate_minutes = ?6, parent_task_id = ?7,
                        blocked_by_task_id = ?8, waiting_for = ?9, follow_up_at = ?10,
                        updated_at = ?11
                 WHERE id = ?12",
                params![
                    edit.title,
                    edit.description,
                    projeto,
                    prazo,
                    edit.priority.as_str(),
                    edit.estimate_minutes,
                    pai,
                    travada,
                    edit.waiting_for,
                    cobranca,
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
        // **So o que MUDOU viaja.**
        //
        // A escrita e autoritativa (a Task inteira chega pronta), mas a emissao
        // nao pode ser: o merge do M/OS e por campo, e um campo emitido e um
        // campo DISPUTADO. Emitindo os dez sempre, mudar a prioridade no
        // celular mandaria junto o `dueAt: null` que ele leu antes — e como a
        // operacao do celular e mais recente, ela apagaria o prazo que o PC
        // acabou de pôr. Os dois gestos tocaram campos diferentes e mesmo assim
        // um venceria o outro, que e exatamente o que o `SYNC.md` §4 promete
        // que nao acontece.
        //
        // Com quatro campos isso era teoria; com dez e o caminho normal de
        // perder trabalho. O diff contra a linha ANTERIOR resolve, e de quebra
        // encolhe a fila: renomear uma Task emite um campo, e nao dez.
        let mut mudancas: Vec<(&str, serde_json::Value)> = Vec::new();
        let mut mudou = |nome: &'static str, antes: serde_json::Value, agora: serde_json::Value| {
            if antes != agora {
                mudancas.push((nome, agora));
            }
        };
        mudou(
            "title",
            serde_json::json!(antes.title),
            serde_json::json!(edit.title),
        );
        mudou(
            "description",
            serde_json::json!(antes.description),
            serde_json::json!(edit.description),
        );
        mudou(
            "projectId",
            serde_json::json!(antes.project_id.map(|valor| valor.to_string())),
            serde_json::json!(projeto),
        );
        mudou(
            "dueAt",
            serde_json::json!(antes.due_at.map(format_time).transpose()?),
            serde_json::json!(prazo),
        );
        mudou(
            "priority",
            serde_json::json!(antes.priority.as_str()),
            serde_json::json!(edit.priority.as_str()),
        );
        mudou(
            "estimateMinutes",
            serde_json::json!(antes.estimate_minutes),
            serde_json::json!(edit.estimate_minutes),
        );
        mudou(
            "parentTaskId",
            serde_json::json!(antes.parent_task_id.map(|valor| valor.to_string())),
            serde_json::json!(pai),
        );
        mudou(
            "blockedByTaskId",
            serde_json::json!(antes.blocked_by_task_id.map(|valor| valor.to_string())),
            serde_json::json!(travada),
        );
        mudou(
            "waitingFor",
            serde_json::json!(antes.waiting_for),
            serde_json::json!(edit.waiting_for),
        );
        mudou(
            "followUpAt",
            serde_json::json!(antes.follow_up_at.map(format_time).transpose()?),
            serde_json::json!(cobranca),
        );
        if !mudancas.is_empty() {
            self.emitir_update(&transaction, "task", id.as_uuid(), &mudancas)?;
        }
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, id)
    }

    /// A folha inteira da Task, numa conexao so.
    ///
    /// Os lembretes vem de `AttentionRepository::reminders_for` — o Attention
    /// System, e nao um agendador proprio. As referencias vem de `resources`
    /// pela juncao. Nada aqui e uma segunda implementacao de nada.
    fn task_detail(&self, id: TaskId) -> Result<TaskDetail, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let task = query_task(&connection, id)?;
        let checklist = query_checklist(&connection, id)?;
        let subtasks = query_tasks(
            &connection,
            &format!(
                "SELECT {colunas} FROM tasks t
                  WHERE t.parent_task_id = '{pai}' AND t.lifecycle_state = 'active'
                  ORDER BY t.created_at ASC",
                colunas = task_select("t"),
                pai = id.to_string().replace('\'', "''"),
            ),
        )?;
        let blocked_by = match task.blocked_by_task_id {
            // `ok()` e nao `?`: a Task que bloqueava pode ter sido apagada, e a
            // coluna vira NULL pela FK — mas um banco vindo de outro aparelho
            // pode ter o id sem a linha ainda. Uma gaveta que se recusa a abrir
            // por causa disso e pior que uma gaveta sem o titulo do bloqueio.
            Some(travada) => query_task(&connection, travada).ok(),
            None => None,
        };
        let references = query_task_references(&connection, id)?;
        let reminders = crate::attention_repository::query_reminders_for_target(
            &connection,
            "task",
            &id.to_string(),
        )?;
        Ok(TaskDetail {
            task,
            checklist,
            subtasks,
            blocked_by,
            references,
            reminders,
        })
    }

    fn add_checklist_item(&self, item: NewChecklistItem) -> Result<Task, CoreError> {
        let task_id = item.task_id;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let position = next_checklist_position(&transaction, task_id)?;
        insert_checklist_item(self, &transaction, &item, position)?;
        touch_task(self, &transaction, task_id)?;
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, task_id)
    }

    fn add_checklist_items(
        &self,
        task_id: TaskId,
        labels: &[String],
    ) -> Result<Vec<ChecklistItem>, CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        // A partir do fim do que ja existe: colar cinco linhas numa Task que ja
        // tem tres passos poe os cinco DEPOIS deles, e nao por cima.
        let primeira = next_checklist_position(&transaction, task_id)?;
        for (position, label) in (primeira..).zip(labels.iter()) {
            let item = NewChecklistItem::create(task_id, label)?;
            insert_checklist_item(self, &transaction, &item, position)?;
        }
        touch_task(self, &transaction, task_id)?;
        transaction.commit().map_err(map_sql_error)?;
        query_checklist(&connection, task_id)
    }

    fn rename_checklist_item(
        &self,
        id: ChecklistItemId,
        label: &str,
    ) -> Result<ChecklistItem, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        // Uma unica passagem pelo portao. Pedir `escrita()` duas vezes na mesma
        // funcao e o abraco mortal que o portao existe para impedir — ele nao e
        // reentrante, e a segunda chamada esperaria a primeira soltar.
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let task_id = task_of_item(&transaction, id)?;
        let label = NewChecklistItem::create(task_id, label)?.label;
        delete_checklist_search(&transaction, id)?;
        let changed = transaction
            .execute(
                "UPDATE task_checklist_items SET label = ?1, updated_at = ?2 WHERE id = ?3",
                params![label, now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        let rowid: i64 = transaction
            .query_row(
                "SELECT rowid FROM task_checklist_items WHERE id = ?1",
                [id.to_string()],
                |linha| linha.get(0),
            )
            .map_err(map_sql_error)?;
        insert_checklist_search(&transaction, rowid)?;
        self.emitir_update(
            &transaction,
            "task_checklist_item",
            id.as_uuid(),
            &[("label", serde_json::json!(label))],
        )?;
        touch_task(self, &transaction, task_id)?;
        transaction.commit().map_err(map_sql_error)?;
        query_checklist_item(&connection, id)
    }

    /// Marcar e desmarcar sao a MESMA operacao com valores opostos.
    ///
    /// `completedAt` e um campo so, entao marcar no PC e desmarcar no celular e
    /// uma escrita concorrente sobre o mesmo campo: o instante decide, e o lado
    /// perdedor vai para `sync_conflicts` em vez de sumir. Um par de campos
    /// (`done` + `completedAt`) daria dois campos que podem discordar.
    fn set_checklist_item_done(&self, id: ChecklistItemId, done: bool) -> Result<Task, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let completed_at = done.then(|| now.clone());
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let task_id = task_of_item(&transaction, id)?;
        let changed = transaction
            .execute(
                "UPDATE task_checklist_items SET completed_at = ?1, updated_at = ?2 WHERE id = ?3",
                params![completed_at, now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        self.emitir_update(
            &transaction,
            "task_checklist_item",
            id.as_uuid(),
            &[("completedAt", serde_json::json!(completed_at))],
        )?;
        touch_task(self, &transaction, task_id)?;
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, task_id)
    }

    /// Apagar e LOGICO, como todo apagamento que atravessa.
    ///
    /// `lifecycle_state = 'trashed'` e nao `DELETE`: e o que a projecao escreve
    /// quando um `OpBody::Delete` chega de fora, e divergir aqui faria a mesma
    /// acao deixar dois estados diferentes nos dois PCs.
    fn delete_checklist_item(&self, id: ChecklistItemId) -> Result<Task, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let task_id = task_of_item(&transaction, id)?;
        // A linha do indice FICA. Ela espelha a tabela uma-para-uma, e
        // `ensure_search_projection` compara as duas contagens na abertura:
        // tirar do indice o que continua na tabela faria o app reconstruir o
        // indice inteiro toda vez que alguem apagasse um item. Quem filtra
        // `trashed` e a consulta da busca, que ja o faz.
        let changed = transaction
            .execute(
                "UPDATE task_checklist_items SET lifecycle_state = 'trashed', updated_at = ?1
                  WHERE id = ?2",
                params![now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("task_checklist_item", id.as_uuid()),
            mos_sync::OpBody::Delete,
        )?;
        touch_task(self, &transaction, task_id)?;
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, task_id)
    }

    fn reorder_checklist(
        &self,
        task_id: TaskId,
        ids: &[ChecklistItemId],
    ) -> Result<Vec<ChecklistItem>, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        for (posicao, id) in ids.iter().enumerate() {
            let posicao = posicao as i64;
            // `AND task_id = ?` de proposito: um id de outra Task chegando aqui
            // reordenaria o checklist alheio em silencio. Zero linhas alteradas
            // e a resposta certa, e nao um erro — a lista pode ter mudado
            // debaixo de quem arrastou.
            transaction
                .execute(
                    "UPDATE task_checklist_items SET position = ?1, updated_at = ?2
                      WHERE id = ?3 AND task_id = ?4",
                    params![posicao, now, id.to_string(), task_id.to_string()],
                )
                .map_err(map_sql_error)?;
            self.emitir_update(
                &transaction,
                "task_checklist_item",
                id.as_uuid(),
                &[("position", serde_json::json!(posicao))],
            )?;
        }
        touch_task(self, &transaction, task_id)?;
        transaction.commit().map_err(map_sql_error)?;
        query_checklist(&connection, task_id)
    }

    fn set_task_reference(
        &self,
        task_id: TaskId,
        resource_id: ResourceId,
        linked: bool,
    ) -> Result<(), CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        link_resource_task(self, &transaction, resource_id, task_id, linked)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
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
                "SELECT {colunas} FROM tasks t WHERE t.{lifecycle}
                 ORDER BY CASE t.work_state WHEN 'doing' THEN 0 WHEN 'backlog' THEN 1 ELSE 2 END,
                 t.updated_at DESC",
                colunas = task_select("t")
            ),
        )
    }

    fn set_task_state(&self, id: TaskId, state: TaskState) -> Result<Task, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let completed_at = (state == TaskState::Done).then_some(now.as_str());
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE tasks SET work_state = ?1, updated_at = ?2, completed_at = ?3 WHERE id = ?4",
                params![state.as_str(), now, completed_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        // Mover no Kanban e o gesto mais repetido do M/OS, e o que mais vai
        // acontecer nos dois dispositivos ao mesmo tempo. Campo proprio: mover
        // no celular e renomear no PC precisam conviver.
        self.emitir_update(
            &transaction,
            "task",
            id.as_uuid(),
            &[("workState", serde_json::json!(state.as_str()))],
        )?;
        // A Daily Session acompanha, NA MESMA TRANSACAO.
        //
        // O §11 do pedido pede que concluir a Task vinculada conclua o objetivo
        // do dia — e pede tambem que nao existam estados divergentes. As duas
        // coisas sao a mesma coisa: se isto rodasse depois do commit, uma queda
        // no meio deixaria a Task em `done` e o objetivo pendente, e nada
        // reconciliaria os dois.
        //
        // QUEM DECIDE nao e este arquivo. A regra ("so o objetivo que E aquela
        // Task fecha junto") vive em `mos_core::completes_with_task`, com teste;
        // o filtro `link_kind = 'task'` la dentro e a traducao dela para SQL.
        self.sync_objectives_with_task(&transaction, id.as_uuid(), state == TaskState::Done, &now)?;
        transaction.commit().map_err(map_sql_error)?;
        query_task(&connection, id)
    }

    fn set_task_lifecycle(&self, id: TaskId, lifecycle: LifecycleState) -> Result<Task, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let archived_at = (lifecycle == LifecycleState::Archived).then_some(now.as_str());
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE tasks SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3
                 WHERE id = ?4",
                params![lifecycle.as_str(), now, archived_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        self.emitir_update(
            &transaction,
            "task",
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(lifecycle.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
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
        // A Task chega pelo TITULO ou pelo texto de um PASSO dela.
        //
        // Dois indices e um `UNION`, e nao uma coluna a mais em `task_search`:
        // aquela tabela e de conteudo externo sobre `tasks`, e so pode ter
        // colunas que `tasks` tem. O item de checklist promove a Task pelo
        // mesmo desenho que o segmento de transcricao promove a Meeting
        // (`MEETING-AGENT.md` §15) — procurar "armadura Caixa 01" acha o
        // trabalho, e nao um checkbox solto.
        //
        // `MIN(rank)` porque uma Task pode casar pelos dois lados; sem ele, ela
        // apareceria duas vezes na mesma lista.
        let task_hits = query_tasks(
            &connection,
            &format!(
                "SELECT {columns} FROM tasks t
                   JOIN (SELECT rowid AS alvo, bm25(task_search) AS rank
                           FROM task_search WHERE task_search MATCH {query}
                         UNION ALL
                         SELECT ci.rowid AS alvo, bm25(task_checklist_search) AS rank
                           FROM task_checklist_search s
                           JOIN task_checklist_items c ON c.rowid = s.rowid
                           JOIN tasks ci ON ci.id = c.task_id
                          WHERE task_checklist_search MATCH {query}
                            AND c.lifecycle_state = 'active') acerto
                     ON acerto.alvo = t.rowid
                  WHERE t.lifecycle_state {lifecycle}
                  GROUP BY t.rowid
                  ORDER BY MIN(acerto.rank), t.updated_at DESC LIMIT {limit}",
                columns = task_select("t"),
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
            "task_checklist_search",
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
            "task_checklist_search",
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

/// Insere uma Task dentro de uma transacao ja aberta.
///
/// `pub(crate)` porque a aceitacao de um item de reuniao precisa criar Task,
/// Reminder e o vinculo na MESMA transacao. Duplicar o INSERT la seria criar um
/// segundo lugar que precisa lembrar do FTS.
/// Insere a Task e emite a operacao, na MESMA transacao.
///
/// Os tres caminhos de criacao — direto, a partir de Capture e a partir de
/// Capture com lembrete — passam por aqui. Emitir aqui dentro, e nao em cada
/// um deles, e o que garante que nenhum caminho novo nasca sem rastro: quem
/// esquecer de emitir tera esquecido tambem de inserir.
pub(crate) fn insert_task(
    storage: &SqliteStorage,
    transaction: &Transaction<'_>,
    task: NewTask,
    source_capture_id: Option<CaptureId>,
) -> Result<(), CoreError> {
    let now = format_time(task.created_at)?;
    let id = task.id;
    let titulo = task.title.clone();
    let descricao = task.description.clone();
    let projeto = task.project_id;
    let prazo = task.due_at.map(format_time).transpose()?;
    let prioridade = task.priority.as_str();
    let pai = task.parent_task_id.map(|value| value.to_string());
    transaction
        .execute(
            "INSERT INTO tasks (
                id, title, description, project_id, source_capture_id, work_state,
                lifecycle_state, due_at, priority, estimate_minutes, parent_task_id,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'backlog', 'active', ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                task.id.to_string(),
                task.title,
                task.description,
                task.project_id.map(|value| value.to_string()),
                source_capture_id.map(|value| value.to_string()),
                prazo,
                prioridade,
                task.estimate_minutes,
                pai,
                now,
            ],
        )
        .map_err(map_sql_error)?;
    insert_task_search(transaction, transaction.last_insert_rowid())?;
    storage.emitir(
        transaction,
        mos_sync::EntityRef::new("task", id.as_uuid()),
        mos_sync::OpBody::Create {
            fields: [
                ("title".to_owned(), serde_json::json!(titulo)),
                ("description".to_owned(), serde_json::json!(descricao)),
                (
                    "projectId".to_owned(),
                    serde_json::json!(projeto.map(|value| value.to_string())),
                ),
                (
                    "sourceCaptureId".to_owned(),
                    serde_json::json!(source_capture_id.map(|value| value.to_string())),
                ),
                ("workState".to_owned(), serde_json::json!("backlog")),
                ("dueAt".to_owned(), serde_json::json!(prazo)),
                ("priority".to_owned(), serde_json::json!(prioridade)),
                (
                    "estimateMinutes".to_owned(),
                    serde_json::json!(task.estimate_minutes),
                ),
                ("parentTaskId".to_owned(), serde_json::json!(pai)),
                ("createdAt".to_owned(), serde_json::json!(now)),
            ]
            .into_iter()
            .collect(),
        },
    )?;
    // Os passos entram NA MESMA TRANSACAO da Task.
    //
    // Em duas transacoes existiria um instante em que a Task aparece vazia no
    // quadro, e um `0/0` piscando onde a pessoa acabou de escrever cinco
    // linhas nao e "quase certo" — e uma tela que mente.
    for (indice, label) in task.checklist.iter().enumerate() {
        let item = NewChecklistItem::create(id, label)?;
        insert_checklist_item(storage, transaction, &item, indice as i64)?;
    }
    Ok(())
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

/// Tira a Task do indice, e nao faz nada quando ela nunca entrou nele.
///
/// A tolerancia NAO e defensiva: uma Task que CHEGOU pelo sync e materializada
/// direto na tabela, e ate 2026-09-08 sem passar pelo indice. Editar essa Task
/// aqui pedia ao fts5 para apagar uma linha que ele nao tinha, e o erro dele
/// para isso e `SQLITE_CORRUPT` — o app acusava o banco de estar corrompido
/// quando o que faltava era uma linha de indice. Ver `repository::tirar_do_indice`.
fn delete_task_search(transaction: &Transaction<'_>, id: TaskId) -> Result<(), CoreError> {
    let Some(rowid) = crate::repository::rowid_de(transaction, "tasks", "id", &id.to_string())?
    else {
        return Ok(());
    };
    crate::repository::tirar_do_indice(
        transaction,
        "task_search",
        "tasks",
        &["title", "description"],
        rowid,
    )
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
            &format!("SELECT {} FROM tasks t WHERE t.id = ?1", task_select("t")),
            [id.to_string()],
            RawTask::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Task nao encontrada.", false))?
        .into_task()
}

/// Os itens ATIVOS de uma Task, na ordem escolhida.
///
/// `position` primeiro e `created_at` como desempate: dois itens colados na
/// mesma transacao nascem com posicoes distintas, mas um banco vindo de um
/// aparelho que reordenou pode ter empate — e ordem instavel faz a lista trocar
/// de arranjo entre dois desenhos da mesma tela.
pub(crate) fn query_checklist(
    connection: &rusqlite::Connection,
    task_id: TaskId,
) -> Result<Vec<ChecklistItem>, CoreError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {CHECKLIST_COLUMNS} FROM task_checklist_items
              WHERE task_id = ?1 AND lifecycle_state = 'active'
              ORDER BY position, created_at"
        ))
        .map_err(map_sql_error)?;
    let itens: Vec<ChecklistItem> = statement
        .query_map([task_id.to_string()], RawChecklistItem::from_row)
        .map_err(map_sql_error)?
        .map(|linha| linha.map_err(map_sql_error)?.into_item())
        .collect::<Result<_, _>>()?;
    Ok(itens)
}

fn query_checklist_item(
    connection: &rusqlite::Connection,
    id: ChecklistItemId,
) -> Result<ChecklistItem, CoreError> {
    connection
        .query_row(
            &format!("SELECT {CHECKLIST_COLUMNS} FROM task_checklist_items WHERE id = ?1"),
            [id.to_string()],
            RawChecklistItem::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| {
            CoreError::new(
                ErrorCode::NotFound,
                "Item de checklist nao encontrado.",
                false,
            )
        })?
        .into_item()
}

/// De que Task e este item. Toda escrita de item precisa saber, porque o que
/// volta para a tela e a Task com o progresso novo.
fn task_of_item(
    connection: &rusqlite::Connection,
    id: ChecklistItemId,
) -> Result<TaskId, CoreError> {
    let bruto: Option<String> = connection
        .query_row(
            "SELECT task_id FROM task_checklist_items WHERE id = ?1",
            [id.to_string()],
            |linha| linha.get(0),
        )
        .optional()
        .map_err(map_sql_error)?;
    TaskId::parse(&bruto.ok_or_else(|| {
        CoreError::new(
            ErrorCode::NotFound,
            "Item de checklist nao encontrado.",
            false,
        )
    })?)
}

/// A proxima posicao livre no checklist de uma Task.
///
/// Item novo vai para o FIM, e nao para onde a tabela sortear. E a mesma regra
/// que `WidgetPlacement` segue na Home, e pela mesma razao: quem arrumou a lista
/// escolheu aquela ordem, e um item novo no meio dela seria o sistema
/// desarrumando o que a pessoa arrumou.
fn next_checklist_position(
    connection: &rusqlite::Connection,
    task_id: TaskId,
) -> Result<i64, CoreError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM task_checklist_items WHERE task_id = ?1",
            [task_id.to_string()],
            |linha| linha.get(0),
        )
        .map_err(map_sql_error)
}

fn insert_checklist_search(transaction: &Transaction<'_>, rowid: i64) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO task_checklist_search (rowid, label)
             SELECT rowid, label FROM task_checklist_items WHERE rowid = ?1",
            [rowid],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn delete_checklist_search(
    transaction: &Transaction<'_>,
    id: ChecklistItemId,
) -> Result<(), CoreError> {
    let Some(rowid) =
        crate::repository::rowid_de(transaction, "task_checklist_items", "id", &id.to_string())?
    else {
        return Ok(());
    };
    crate::repository::tirar_do_indice(
        transaction,
        "task_checklist_search",
        "task_checklist_items",
        &["label"],
        rowid,
    )
}

/// Grava UM item e emite a operacao, dentro de uma transacao ja aberta.
///
/// Existe fora do trait porque a criacao de Task com checklist precisa dos
/// itens na MESMA transacao da Task: uma Task que nasce e um checklist que
/// chega depois sao dois estados observaveis, e o de dentro e uma Task vazia
/// piscando no quadro.
fn insert_checklist_item(
    storage: &SqliteStorage,
    transaction: &Transaction<'_>,
    item: &NewChecklistItem,
    position: i64,
) -> Result<(), CoreError> {
    let now = format_time(item.created_at)?;
    transaction
        .execute(
            "INSERT INTO task_checklist_items
                (id, task_id, label, position, lifecycle_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)",
            params![
                item.id.to_string(),
                item.task_id.to_string(),
                item.label,
                position,
                now
            ],
        )
        .map_err(map_sql_error)?;
    insert_checklist_search(transaction, transaction.last_insert_rowid())?;
    storage.emitir(
        transaction,
        mos_sync::EntityRef::new("task_checklist_item", item.id.as_uuid()),
        mos_sync::OpBody::Create {
            fields: [
                (
                    "taskId".to_owned(),
                    serde_json::json!(item.task_id.to_string()),
                ),
                ("label".to_owned(), serde_json::json!(item.label)),
                ("position".to_owned(), serde_json::json!(position)),
                ("completedAt".to_owned(), serde_json::Value::Null),
                ("createdAt".to_owned(), serde_json::json!(now)),
            ]
            .into_iter()
            .collect(),
        },
    )
}

/// Os Resources ligados a uma Task, do mais recente para o mais antigo.
fn query_task_references(
    connection: &rusqlite::Connection,
    task_id: TaskId,
) -> Result<Vec<Resource>, CoreError> {
    crate::resource_repository::query_resources(
        connection,
        &format!(
            "SELECT r.{colunas} FROM resources r
               JOIN resource_tasks rt ON rt.resource_id = r.id
              WHERE rt.task_id = '{task}' AND r.lifecycle_state = 'active'
              ORDER BY rt.created_at DESC",
            colunas = crate::resource_repository::RESOURCE_COLUMNS.replace(", ", ", r."),
            task = task_id.to_string().replace('\'', "''"),
        ),
    )
}

/// Marca a Task como mexida agora.
///
/// Mexer no checklist E mexer na Task: o quadro ordena por `updated_at`, a
/// deteccao de parada (`stale.rs`) conta dias desde ela, e uma Task cujo
/// checklist andou a manha inteira apareceria como abandonada sem isto.
///
/// **Nao emite operacao de `updatedAt`.** O carimbo e de quem APLICOU a
/// mudanca, e nao um campo do dominio — quem recebe a operacao do item ja
/// carimba a Task dele pelo mesmo caminho. Sincronizar o carimbo faria os dois
/// aparelhos disputarem um numero que nem descreve a mesma coisa, que e
/// exatamente o argumento que manteve `deliveredCount` fora do sync.
fn touch_task(
    _storage: &SqliteStorage,
    transaction: &Transaction<'_>,
    task_id: TaskId,
) -> Result<(), CoreError> {
    let now = format_time(OffsetDateTime::now_utc())?;
    transaction
        .execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2",
            params![now, task_id.to_string()],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

/// Liga ou desliga um Resource a uma Task. Gemeo de `link_resource_project`.
pub(crate) fn link_resource_task(
    storage: &SqliteStorage,
    connection: &rusqlite::Connection,
    resource_id: ResourceId,
    task_id: TaskId,
    linked: bool,
) -> Result<(), CoreError> {
    if linked {
        let now = format_time(OffsetDateTime::now_utc())?;
        connection
            .execute(
                "INSERT OR IGNORE INTO resource_tasks (resource_id, task_id, created_at)
                 VALUES (?1, ?2, ?3)",
                params![resource_id.to_string(), task_id.to_string(), now],
            )
            .map_err(map_sql_error)?;
    } else {
        connection
            .execute(
                "DELETE FROM resource_tasks WHERE resource_id = ?1 AND task_id = ?2",
                params![resource_id.to_string(), task_id.to_string()],
            )
            .map_err(map_sql_error)?;
    }
    storage.emitir_relacao(
        connection,
        "resourceTask",
        resource_id.as_uuid(),
        task_id.as_uuid(),
        linked,
    )
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
                "SELECT {colunas} FROM tasks t WHERE t.source_capture_id = ?1 AND t.{lifecycle}",
                colunas = task_select("t")
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

    // ---------------------------------------------------------- o checklist

    fn task_de(storage: &SqliteStorage, titulo: &str) -> Task {
        storage
            .create_task(NewTask::create(titulo, "", None).unwrap())
            .unwrap()
    }

    #[test]
    fn o_ciclo_de_um_item_de_checklist() {
        let (_guarda, storage) = storage();
        let task = task_de(&storage, "Revisar projeto estrutural");
        assert_eq!((task.checklist_total, task.checklist_done), (0, 0));

        let task = storage
            .add_checklist_item(NewChecklistItem::create(task.id, "Conferir niveis").unwrap())
            .unwrap();
        assert_eq!((task.checklist_total, task.checklist_done), (1, 0));

        let itens = storage.task_detail(task.id).unwrap().checklist;
        let item = itens[0].id;

        let task = storage.set_checklist_item_done(item, true).unwrap();
        assert_eq!((task.checklist_total, task.checklist_done), (1, 1));
        assert_eq!(task.checklist_progress(), Some(1.0));
        assert!(
            task.checklist_is_complete(),
            "oferece concluir, sem concluir"
        );
        assert_eq!(
            storage.get_task(task.id).unwrap().state,
            TaskState::Backlog,
            "marcar todo o checklist NAO conclui a Task sozinha"
        );

        let task = storage.set_checklist_item_done(item, false).unwrap();
        assert_eq!((task.checklist_total, task.checklist_done), (1, 0));

        let renomeado = storage
            .rename_checklist_item(item, "  - Conferir os niveis ")
            .unwrap();
        assert_eq!(renomeado.label, "Conferir os niveis");

        let task = storage.delete_checklist_item(item).unwrap();
        assert_eq!((task.checklist_total, task.checklist_done), (0, 0));
        assert!(storage.task_detail(task.id).unwrap().checklist.is_empty());
    }

    /// Concluir a Task PRESERVA o checklist, e reabrir a devolve como estava.
    #[test]
    fn concluir_a_task_nao_apaga_o_que_foi_feito() {
        let (_guarda, storage) = storage();
        let task = task_de(&storage, "Ajustes reuniao");
        storage
            .add_checklist_items(task.id, &["Corrigir nivel".into(), "Gerar PDF".into()])
            .unwrap();
        let itens = storage.task_detail(task.id).unwrap().checklist;
        storage.set_checklist_item_done(itens[0].id, true).unwrap();

        let concluida = storage.set_task_state(task.id, TaskState::Done).unwrap();
        assert_eq!(
            (concluida.checklist_total, concluida.checklist_done),
            (2, 1)
        );

        let reaberta = storage.set_task_state(task.id, TaskState::Doing).unwrap();
        assert_eq!((reaberta.checklist_total, reaberta.checklist_done), (2, 1));
        let depois = storage.task_detail(task.id).unwrap().checklist;
        assert!(depois[0].completed(), "o item feito continua feito");
        assert!(!depois[1].completed());
    }

    #[test]
    fn item_novo_vai_para_o_fim_e_a_ordem_e_do_usuario() {
        let (_guarda, storage) = storage();
        let task = task_de(&storage, "Finalizar 167-25");
        storage
            .add_checklist_items(task.id, &["Um".into(), "Dois".into(), "Tres".into()])
            .unwrap();
        let itens = storage.task_detail(task.id).unwrap().checklist;
        assert_eq!(
            itens
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["Um", "Dois", "Tres"]
        );

        let invertido: Vec<_> = itens.iter().rev().map(|item| item.id).collect();
        let depois = storage.reorder_checklist(task.id, &invertido).unwrap();
        assert_eq!(
            depois
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["Tres", "Dois", "Um"]
        );

        storage
            .add_checklist_item(NewChecklistItem::create(task.id, "Quatro").unwrap())
            .unwrap();
        let final_ = storage.task_detail(task.id).unwrap().checklist;
        assert_eq!(
            final_.last().unwrap().label,
            "Quatro",
            "item novo nasce no fim, e nao no meio do que a pessoa arrumou"
        );
    }

    /// Reordenar com o id de OUTRA Task nao mexe no checklist alheio.
    #[test]
    fn reordenar_nao_alcanca_o_checklist_de_outra_task() {
        let (_guarda, storage) = storage();
        let uma = task_de(&storage, "Uma");
        let outra = task_de(&storage, "Outra");
        storage
            .add_checklist_items(uma.id, &["A".into(), "B".into()])
            .unwrap();
        storage
            .add_checklist_items(outra.id, &["X".into()])
            .unwrap();
        let alheio = storage.task_detail(outra.id).unwrap().checklist[0].id;
        let posicao_antes = storage.task_detail(outra.id).unwrap().checklist[0].position;

        storage.reorder_checklist(uma.id, &[alheio]).unwrap();

        assert_eq!(
            storage.task_detail(outra.id).unwrap().checklist[0].position,
            posicao_antes
        );
    }

    /// A busca acha a Task pelo texto do PASSO, e nao so pelo titulo.
    #[test]
    fn buscar_o_texto_de_um_item_acha_a_task() {
        let (_guarda, storage) = storage();
        let task = task_de(&storage, "Ajustes reuniao");
        storage
            .add_checklist_items(task.id, &["Corrigir armadura Caixa 01".into()])
            .unwrap();

        let achados = storage
            .search_all(SearchRequest {
                query: "armadura Caixa 01".into(),
                limit: 10,
                include_archived: false,
            })
            .unwrap();
        assert!(
            achados.iter().any(|item| matches!(
                item,
                SearchItem::Task { task: achada, .. } if achada.id == task.id
            )),
            "o passo tem de promover a Task: {achados:?}"
        );

        // E uma vez so, mesmo quando o titulo tambem casa.
        let por_titulo = storage
            .search_all(SearchRequest {
                query: "Ajustes".into(),
                limit: 10,
                include_archived: false,
            })
            .unwrap();
        let vezes = por_titulo
            .iter()
            .filter(|item| matches!(item, SearchItem::Task { task: achada, .. } if achada.id == task.id))
            .count();
        assert_eq!(vezes, 1, "a mesma Task nao pode aparecer duas vezes");
    }

    /// Item apagado sai da busca — e sem derrubar a contagem do indice, que e o
    /// que faria o app reconstruir o FTS a cada abertura.
    #[test]
    fn item_apagado_some_da_busca_sem_desalinhar_o_indice() {
        let (_guarda, storage) = storage();
        let task = task_de(&storage, "Ajustes");
        storage
            .add_checklist_items(task.id, &["Revisar cobrimento".into()])
            .unwrap();
        let item = storage.task_detail(task.id).unwrap().checklist[0].id;
        storage.delete_checklist_item(item).unwrap();

        let achados = storage
            .search_all(SearchRequest {
                query: "cobrimento".into(),
                limit: 10,
                include_archived: false,
            })
            .unwrap();
        assert!(achados.is_empty(), "{achados:?}");

        let connection = storage.connection.lock().unwrap();
        let na_tabela: i64 = connection
            .query_row("SELECT count(*) FROM task_checklist_items", [], |linha| {
                linha.get(0)
            })
            .unwrap();
        let no_indice: i64 = connection
            .query_row("SELECT count(*) FROM task_checklist_search", [], |linha| {
                linha.get(0)
            })
            .unwrap();
        assert_eq!(na_tabela, no_indice);
    }

    /// A Task antiga — sem nenhuma coluna da 0039 preenchida — continua valida.
    #[test]
    fn a_task_de_antes_da_migration_continua_inteira() {
        let (_guarda, storage) = storage();
        let task = task_de(&storage, "Task de antes");
        assert!(task.due_at.is_none());
        assert_eq!(task.priority, Priority::Normal);
        assert!(task.estimate_minutes.is_none());
        assert!(task.parent_task_id.is_none());
        assert!(task.blocked_by_task_id.is_none());
        assert!(task.waiting_for.is_empty());
        assert_eq!((task.checklist_total, task.checklist_done), (0, 0));

        let detalhe = storage.task_detail(task.id).unwrap();
        assert!(detalhe.checklist.is_empty());
        assert!(detalhe.subtasks.is_empty());
        assert!(detalhe.references.is_empty());
        assert!(detalhe.reminders.is_empty());
    }

    /// Prazo, prioridade, subtask e bloqueio sobrevivem a uma edicao.
    #[test]
    fn a_gaveta_grava_prazo_prioridade_e_hierarquia() {
        let (_guarda, storage) = storage();
        let pai = task_de(&storage, "Finalizar 167-25");
        let trava = task_de(&storage, "Finalizar revisao");
        let filha = task_de(&storage, "Gerar PDF");

        let prazo = OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap();
        let editada = storage
            .update_task(
                filha.id,
                EditTask {
                    priority: Priority::High,
                    due_at: Some(prazo),
                    estimate_minutes: Some(30),
                    parent_task_id: Some(pai.id),
                    blocked_by_task_id: Some(trava.id),
                    waiting_for: "Victor".into(),
                    follow_up_at: Some(prazo),
                    ..EditTask::from_task(&filha)
                },
            )
            .unwrap();
        assert_eq!(editada.priority, Priority::High);
        assert_eq!(editada.due_at, Some(prazo));
        assert_eq!(editada.estimate_minutes, Some(30));
        assert_eq!(editada.waiting_for, "Victor");

        let detalhe = storage.task_detail(pai.id).unwrap();
        assert_eq!(detalhe.subtasks.len(), 1);
        assert_eq!(detalhe.subtasks[0].id, filha.id);
        assert_eq!(
            storage
                .task_detail(filha.id)
                .unwrap()
                .blocked_by
                .unwrap()
                .title,
            "Finalizar revisao"
        );

        // Tirar o prazo e mandar `None`, e nao omitir o campo.
        let limpa = storage
            .update_task(
                filha.id,
                EditTask {
                    due_at: None,
                    ..EditTask::from_task(&editada)
                },
            )
            .unwrap();
        assert!(limpa.due_at.is_none());
        assert_eq!(limpa.priority, Priority::High, "so o prazo saiu");
    }

    #[test]
    fn uma_task_nao_e_subtask_nem_bloqueio_de_si_mesma() {
        let (_guarda, storage) = storage();
        let task = task_de(&storage, "Uma");
        assert!(storage
            .update_task(
                task.id,
                EditTask {
                    parent_task_id: Some(task.id),
                    ..EditTask::from_task(&task)
                }
            )
            .is_err());
        assert!(storage
            .update_task(
                task.id,
                EditTask {
                    blocked_by_task_id: Some(task.id),
                    ..EditTask::from_task(&task)
                }
            )
            .is_err());
    }

    /// A referencia da Task e o mesmo Resource da Library.
    #[test]
    fn a_referencia_da_task_e_um_resource_de_verdade() {
        use mos_core::{NewResource, ResourceKind, ResourceRepository};

        let (_guarda, storage) = storage();
        let task = task_de(&storage, "Revisar projeto");
        let resource = storage
            .create_resource(
                NewResource::create(
                    ResourceKind::Site,
                    "Projeto estrutural R02",
                    "https://exemplo.test/r02.pdf",
                    "",
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        storage
            .set_task_reference(task.id, resource.id, true)
            .unwrap();
        let detalhe = storage.task_detail(task.id).unwrap();
        assert_eq!(detalhe.references.len(), 1);
        assert_eq!(detalhe.references[0].id, resource.id);

        storage
            .set_task_reference(task.id, resource.id, false)
            .unwrap();
        assert!(storage.task_detail(task.id).unwrap().references.is_empty());
    }

    /// Task, Reminder e o processamento da Capture caem juntos ou nao caem.
    ///
    /// O que este teste protege e o instante que nao pode existir: a Task
    /// gravada e o aviso dela nao. Numa acao derivada de voz esse instante e
    /// especialmente caro, porque ninguem digitou nada — a pessoa falou, leu
    /// "lembrete criado" e foi embora confiando.
    #[test]
    fn voz_cria_task_e_reminder_na_mesma_transacao() {
        use mos_core::{AttentionRepository, Clock, FixedClock, NewReminder, ReminderSource};

        let (_guard, storage) = storage();
        let clock = FixedClock::at(OffsetDateTime::from_unix_timestamp(1_787_000_000).unwrap());
        let capture = storage
            .create(
                NewCapture::create(
                    "me lembra amanha as nove de revisar o memorial",
                    CaptureSource::Voice,
                )
                .unwrap(),
            )
            .unwrap();

        let quando = clock.now() + time::Duration::hours(18);
        let (task, reminder) = storage
            .create_task_from_capture_with_reminder(
                capture.id,
                NewTask::create("Revisar o memorial", "", None).unwrap(),
                Some(
                    NewReminder::at("Revisar o memorial", "", quando, &clock)
                        .unwrap()
                        .from_source(ReminderSource::Capture),
                ),
            )
            .unwrap();

        let reminder = reminder.expect("o lembrete faz parte da mesma acao");
        assert_eq!(task.source_capture_id, Some(capture.id));
        // O lembrete aponta para a TASK: quando ele tocar, "o que eu tenho de
        // fazer?" tem resposta sem passar pela Inbox.
        assert_eq!(
            reminder.target,
            Some(mos_core::ReminderTarget::Task(task.id))
        );
        assert_eq!(reminder.source, ReminderSource::Capture);
        // E o lembrete esta de verdade no banco, e nao so no valor devolvido.
        assert_eq!(
            AttentionRepository::reminder(&storage, reminder.id)
                .unwrap()
                .id,
            reminder.id
        );
        // A Capture saiu da Inbox pela mesma transacao.
        assert_eq!(
            storage.get(capture.id).unwrap().processing_state,
            mos_core::ProcessingState::Processed
        );
    }

    /// Uma Capture tem no maximo uma Task derivada, e a segunda tentativa nao
    /// pode deixar um Reminder orfao para tras.
    #[test]
    fn a_segunda_acao_sobre_a_mesma_capture_nao_deixa_lembrete_orfao() {
        use mos_core::{AttentionRepository, Clock, FixedClock, NewReminder};

        let (_guard, storage) = storage();
        let clock = FixedClock::at(OffsetDateTime::from_unix_timestamp(1_787_000_000).unwrap());
        let capture = storage
            .create(NewCapture::create("comprar cafe", CaptureSource::Voice).unwrap())
            .unwrap();
        storage
            .create_task_from_capture_with_reminder(
                capture.id,
                NewTask::create("Comprar cafe", "", None).unwrap(),
                None,
            )
            .unwrap();

        let erro = storage
            .create_task_from_capture_with_reminder(
                capture.id,
                NewTask::create("Comprar cafe de novo", "", None).unwrap(),
                Some(
                    NewReminder::at(
                        "Comprar cafe de novo",
                        "",
                        clock.now() + time::Duration::hours(2),
                        &clock,
                    )
                    .unwrap(),
                ),
            )
            .unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidTransition);
        // A transacao inteira voltou atras: nenhum lembrete ficou agendado.
        assert!(AttentionRepository::open_reminders(&storage)
            .unwrap()
            .is_empty());
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

        storage
            .set_widget_hidden(None, "inbox_pulse", true)
            .unwrap();
        let hidden = storage.hidden_widgets().unwrap();

        assert_eq!(hidden.len(), 1);
        assert_eq!(
            hidden[0].workspace_id, None,
            "nao pertence a Workspace nenhum"
        );
        assert_eq!(hidden[0].widget_id, "inbox_pulse");

        storage
            .set_widget_hidden(None, "inbox_pulse", false)
            .unwrap();
        assert!(
            storage.hidden_widgets().unwrap().is_empty(),
            "e volta a aparecer"
        );
    }

    /// A armadilha que o indice unico da 0019 fecha: no SQLite, coluna de
    /// PRIMARY KEY aceita NULL e NULL nunca colide com NULL. Sem o indice, o
    /// `INSERT OR IGNORE` nao teria com o que conflitar e cada clique em
    /// "ocultar" empilharia mais uma linha para "Todos".
    #[test]
    fn hiding_twice_without_a_workspace_does_not_pile_up_rows() {
        let (_directory, storage) = storage();

        for _ in 0..3 {
            storage
                .set_widget_hidden(None, "system_health", true)
                .unwrap();
        }

        assert_eq!(
            storage.hidden_widgets().unwrap().len(),
            1,
            "uma linha, nao tres"
        );
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
            saved
                .iter()
                .map(|p| p.widget_id.as_str())
                .collect::<Vec<_>>(),
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
        let timer = mudou_de_faixa
            .iter()
            .find(|p| p.widget_id == "timer")
            .unwrap();
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
            restante
                .iter()
                .filter(|p| p.workspace_id == Some(outro.id))
                .count(),
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
        assert!(apos_todos
            .iter()
            .all(|p| p.workspace_id == Some(estudio.id)));
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
            saved
                .iter()
                .find(|p| p.widget_id == "now")
                .and_then(|p| p.span),
            Some(8)
        );
    }

    #[test]
    fn petala_grava_le_e_respeita_o_escopo_de_todos() {
        let (_directory, storage) = storage();

        // Vazio significa o desenho, e nao lista vazia de erro.
        assert!(storage.radial_pins().unwrap().is_empty());

        let pins = storage
            .set_radial_pin(
                None,
                mos_core::RadialPinInput {
                    slot: 0,
                    kind: "pagina".into(),
                    target: "calendario".into(),
                },
            )
            .unwrap();
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].slot, 0);
        assert_eq!(pins[0].kind, "pagina");
        assert!(pins[0].workspace_id.is_none());

        // O MESMO slot em "Todos" substitui, e nao acumula. E o buraco que o
        // indice sobre COALESCE existe para fechar.
        let pins = storage
            .set_radial_pin(
                None,
                mos_core::RadialPinInput {
                    slot: 0,
                    kind: "app".into(),
                    target: "019ffc4f-2936-7152-84b7-672d7bdb5bfc".into(),
                },
            )
            .unwrap();
        assert_eq!(pins.len(), 1, "slot 0 de Todos nao pode ter duas linhas");
        assert_eq!(pins[0].kind, "app");

        // Kind fora de forma nunca chega ao banco.
        assert!(storage
            .set_radial_pin(
                None,
                mos_core::RadialPinInput {
                    slot: 1,
                    kind: "Pagina".into(),
                    target: "x".into(),
                },
            )
            .is_err());

        // Alvo vazio tambem nao: uma petala sem alvo e uma petala que nao faz
        // nada quando clicada.
        assert!(storage
            .set_radial_pin(
                None,
                mos_core::RadialPinInput {
                    slot: 1,
                    kind: "pagina".into(),
                    target: "   ".into(),
                },
            )
            .is_err());

        // Limpar devolve o slot ao desenho.
        let pins = storage.clear_radial_pin(None, 0).unwrap();
        assert!(pins.is_empty());
    }
}
