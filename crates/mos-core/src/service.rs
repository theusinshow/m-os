use std::{path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    validate_title, ActiveTimer, ActivityEvent, ActivityEventId, AppCapabilities, AppId,
    AppLaunchKind, AppRepository, BackupInspection, BackupReceipt, Capture, CaptureId,
    CaptureRepository, CaptureSource, Client, ClientId, ClientInput, Conversation, ConversationId,
    ConversationRepository, ConversationSummary, CoreError, DataMaintenance, HiddenWidget,
    LifecycleState, Message, MessageId, MessageStatus, MonitoredApp, MonitoringRepository,
    NewActivityEvent, NewCapture, NewConversation, NewMessage, NewProject, NewRegisteredApp,
    NewTask, NewTimeEntry, NewWorkspace, PartBody, ProcessingState, Project, ProjectId,
    ProjectTracking, RegisteredApp, SearchItem, SearchRequest, StartTimer, Task, TaskId, TaskState,
    TimeEntry, TimeEntryEdit, TimeEntryId, TimeTrackingRepository, Totals, TrackedSession,
    TrackingSettings, WorkRepository, Workspace, WorkspaceId,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCaptureInput {
    pub content: String,
    pub source: CaptureSource,
}

#[derive(Clone)]
pub struct CaptureService {
    repository: Arc<dyn CaptureRepository>,
}

impl CaptureService {
    pub fn new(repository: Arc<dyn CaptureRepository>) -> Self {
        Self { repository }
    }

    pub fn create(&self, input: CreateCaptureInput) -> Result<Capture, CoreError> {
        self.repository
            .create(NewCapture::create(&input.content, input.source)?)
    }

    pub fn get(&self, id: &str) -> Result<Capture, CoreError> {
        self.repository.get(CaptureId::parse(id)?)
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<Capture>, CoreError> {
        self.repository.recent(limit.min(50))
    }

    pub fn inbox(&self, limit: usize) -> Result<Vec<Capture>, CoreError> {
        self.repository.inbox(limit.min(200))
    }

    pub fn archived(&self, limit: usize) -> Result<Vec<Capture>, CoreError> {
        self.repository
            .by_lifecycle(LifecycleState::Archived, limit.min(200))
    }

    pub fn trashed(&self, limit: usize) -> Result<Vec<Capture>, CoreError> {
        self.repository
            .by_lifecycle(LifecycleState::Trashed, limit.min(200))
    }

    pub fn search(
        &self,
        query: &str,
        include_archived: bool,
        limit: usize,
    ) -> Result<Vec<Capture>, CoreError> {
        self.repository.search(SearchRequest {
            query: query.trim().to_owned(),
            include_archived,
            limit: limit.min(100),
        })
    }

    pub fn mark_processed(&self, id: &str) -> Result<Capture, CoreError> {
        self.repository
            .set_processing_state(CaptureId::parse(id)?, ProcessingState::Processed)
    }

    pub fn move_to_inbox(&self, id: &str) -> Result<Capture, CoreError> {
        self.repository
            .set_processing_state(CaptureId::parse(id)?, ProcessingState::Inbox)
    }

    pub fn archive(&self, id: &str) -> Result<Capture, CoreError> {
        self.repository
            .set_lifecycle_state(CaptureId::parse(id)?, LifecycleState::Archived)
    }

    pub fn trash(&self, id: &str) -> Result<Capture, CoreError> {
        self.repository
            .set_lifecycle_state(CaptureId::parse(id)?, LifecycleState::Trashed)
    }

    pub fn restore(&self, id: &str) -> Result<Capture, CoreError> {
        self.repository
            .set_lifecycle_state(CaptureId::parse(id)?, LifecycleState::Active)
    }

    pub fn delete_capture(&self, id: &str) -> Result<(), CoreError> {
        self.repository.delete_capture(CaptureId::parse(id)?)
    }

    pub fn rebuild_search(&self) -> Result<usize, CoreError> {
        self.repository.rebuild_search()
    }
}

/// Rastreio de tempo por Project (ADR-032).
#[derive(Clone)]
pub struct TrackingService {
    repository: Arc<dyn TimeTrackingRepository>,
}

impl TrackingService {
    pub fn new(repository: Arc<dyn TimeTrackingRepository>) -> Self {
        Self { repository }
    }

    pub fn record(&self, entry: NewTimeEntry) -> Result<TimeEntry, CoreError> {
        self.repository.create_time_entry(entry)
    }

    pub fn entries(&self, project_id: Option<ProjectId>) -> Result<Vec<TimeEntry>, CoreError> {
        self.repository.time_entries(project_id)
    }

    pub fn edit(&self, id: TimeEntryId, edit: TimeEntryEdit) -> Result<TimeEntry, CoreError> {
        self.repository.update_time_entry(id, edit)
    }

    pub fn trash(&self, id: TimeEntryId) -> Result<(), CoreError> {
        self.repository.trash_time_entry(id)
    }

    pub fn restore(&self, id: TimeEntryId) -> Result<(), CoreError> {
        self.repository.restore_time_entry(id)
    }

    pub fn set_project_tracking(
        &self,
        tracking: ProjectTracking,
    ) -> Result<ProjectTracking, CoreError> {
        self.repository.set_project_tracking(tracking)
    }

    pub fn project_tracking(&self) -> Result<Vec<ProjectTracking>, CoreError> {
        self.repository.project_tracking()
    }

    pub fn active_timer(&self) -> Result<Option<ActiveTimer>, CoreError> {
        self.repository.active_timer()
    }

    pub fn start_timer(&self, start: StartTimer) -> Result<ActiveTimer, CoreError> {
        self.repository.start_timer(start)
    }

    pub fn set_timer_running(&self, running: bool) -> Result<ActiveTimer, CoreError> {
        self.repository.set_timer_running(running)
    }

    pub fn stop_timer(&self) -> Result<TimeEntry, CoreError> {
        self.repository.stop_timer()
    }

    /// Joga fora o cronometro sem gravar. Quem chama confirma antes.
    pub fn discard_timer(&self) -> Result<(), CoreError> {
        self.repository.discard_timer()
    }

    pub fn settings(&self) -> Result<TrackingSettings, CoreError> {
        self.repository.tracking_settings()
    }

    pub fn set_settings(&self, settings: TrackingSettings) -> Result<TrackingSettings, CoreError> {
        self.repository.set_tracking_settings(settings)
    }

    pub fn clients(&self, include_archived: bool) -> Result<Vec<Client>, CoreError> {
        self.repository.clients(include_archived)
    }

    pub fn create_client(&self, input: ClientInput) -> Result<Client, CoreError> {
        self.repository.create_client(input)
    }

    pub fn update_client(&self, id: &str, input: ClientInput) -> Result<Client, CoreError> {
        self.repository.update_client(ClientId::parse(id)?, input)
    }

    pub fn set_client_archived(&self, id: &str, archived: bool) -> Result<Client, CoreError> {
        self.repository
            .set_client_archived(ClientId::parse(id)?, archived)
    }

    /// Totais por Project, ja com o arredondamento configurado aplicado.
    ///
    /// E aqui, e em nenhum lugar antes, que o arredondamento entra: o
    /// repositorio devolve o tempo real e esta funcao compoe a regra pura de
    /// `tracking`. Quem quiser o tempo cru continua tendo `entries()`.
    pub fn totals_by_project(
        &self,
    ) -> Result<std::collections::HashMap<String, Totals>, CoreError> {
        let rounding = self.settings()?.rounding;
        let sessions: Vec<TrackedSession> = self
            .entries(None)?
            .into_iter()
            .map(|entry| TrackedSession {
                project_id: entry.project_id.to_string(),
                duration_seconds: entry.duration_seconds,
                idle_seconds: entry.idle_seconds,
                billable: entry.billable,
                hourly_rate_snapshot_cents: entry.hourly_rate_snapshot_cents,
            })
            .collect();
        Ok(crate::aggregate_by_project(&sessions, rounding))
    }
}

/// O que o sistema observa (ADR-032).
///
/// Servico proprio, e nao metodos no `TrackingService`, pelo mesmo motivo que
/// os repositorios sao dois: observacao nao vira hora sozinha, e manter os dois
/// separados torna essa fronteira visivel na assinatura em vez de depender de
/// alguem lembrar dela.
#[derive(Clone)]
pub struct MonitoringService {
    repository: Arc<dyn MonitoringRepository>,
}

impl MonitoringService {
    pub fn new(repository: Arc<dyn MonitoringRepository>) -> Self {
        Self { repository }
    }

    pub fn apps(&self) -> Result<Vec<MonitoredApp>, CoreError> {
        self.repository.monitored_apps()
    }

    pub fn save_app(&self, app: MonitoredApp) -> Result<MonitoredApp, CoreError> {
        self.repository.save_monitored_app(app)
    }

    pub fn delete_app(&self, id: &str) -> Result<(), CoreError> {
        self.repository.delete_monitored_app(id)
    }

    /// Os eventos de uma janela, do mais antigo para o mais novo.
    pub fn events(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
    ) -> Result<Vec<ActivityEvent>, CoreError> {
        if until < since {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidInput,
                "O fim da janela vem antes do inicio.",
                false,
            ));
        }
        self.repository.activity_events(since, until)
    }

    pub fn record(&self, event: NewActivityEvent) -> Result<ActivityEvent, CoreError> {
        self.repository.record_activity(event)
    }

    pub fn mark_processed(&self, id: &str) -> Result<(), CoreError> {
        self.repository
            .mark_activity_processed(ActivityEventId::parse(id)?)
    }
}

#[derive(Clone)]
pub struct DataService {
    maintenance: Arc<dyn DataMaintenance>,
}

impl DataService {
    pub fn new(maintenance: Arc<dyn DataMaintenance>) -> Self {
        Self { maintenance }
    }

    pub fn create_backup(&self, destination: &Path) -> Result<BackupReceipt, CoreError> {
        self.maintenance.create_backup(destination)
    }

    pub fn inspect_backup(&self, source: &Path) -> Result<BackupInspection, CoreError> {
        self.maintenance.inspect_backup(source)
    }

    pub fn restore_backup(&self, source: &Path) -> Result<BackupReceipt, CoreError> {
        self.maintenance.restore_backup(source)
    }

    pub fn ensure_daily_snapshot(&self) -> Result<Option<BackupReceipt>, CoreError> {
        self.maintenance.ensure_daily_snapshot()
    }

    pub fn export_json(&self, destination: &Path) -> Result<BackupReceipt, CoreError> {
        self.maintenance.export_json(destination)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub repository: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub repository: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub project_id: Option<String>,
    pub source_capture_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub project_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source_url: Option<String>,
    pub launch_kind: Option<AppLaunchKind>,
    pub launch_target: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source_url: Option<String>,
    pub launch_kind: Option<AppLaunchKind>,
    pub launch_target: Option<String>,
    /// Capacidades declaradas. Ausentes no payload significam nao declaradas,
    /// e capacidade nao declarada e capacidade que o Hermes nao tenta usar.
    #[serde(default)]
    pub can_open: bool,
    #[serde(default)]
    pub can_read: bool,
    #[serde(default)]
    pub can_write: bool,
    #[serde(default)]
    pub can_automate: bool,
}

#[derive(Clone)]
pub struct AppService {
    repository: Arc<dyn AppRepository>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResourceInput {
    pub kind: crate::ResourceKind,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub source_capture_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResourceInput {
    pub id: String,
    pub kind: crate::ResourceKind,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone)]
pub struct MemoryService {
    repository: Arc<dyn crate::ResourceRepository>,
}

impl MemoryService {
    pub fn new(repository: Arc<dyn crate::ResourceRepository>) -> Self {
        Self { repository }
    }

    pub fn create_resource(
        &self,
        input: CreateResourceInput,
    ) -> Result<crate::Resource, CoreError> {
        let source_capture_id = input
            .source_capture_id
            .as_deref()
            .map(crate::CaptureId::parse)
            .transpose()?;
        self.repository.create_resource(crate::NewResource::create(
            input.kind,
            &input.title,
            &input.url,
            &input.note,
            source_capture_id,
        )?)
    }

    pub fn update_resource(
        &self,
        input: UpdateResourceInput,
    ) -> Result<crate::Resource, CoreError> {
        let validated =
            crate::NewResource::create(input.kind, &input.title, &input.url, &input.note, None)?;
        self.repository.update_resource(
            crate::ResourceId::parse(&input.id)?,
            validated.kind,
            &validated.title,
            &validated.url,
            &validated.note,
        )
    }

    pub fn resource(&self, id: &str) -> Result<crate::Resource, CoreError> {
        self.repository.get_resource(crate::ResourceId::parse(id)?)
    }

    pub fn resources(&self, include_archived: bool) -> Result<Vec<crate::Resource>, CoreError> {
        self.repository.resources(include_archived)
    }

    pub fn trashed_resources(&self) -> Result<Vec<crate::Resource>, CoreError> {
        self.repository.trashed_resources()
    }

    pub fn set_resource_lifecycle(
        &self,
        id: &str,
        lifecycle: LifecycleState,
    ) -> Result<crate::Resource, CoreError> {
        self.repository
            .set_resource_lifecycle(crate::ResourceId::parse(id)?, lifecycle)
    }

    pub fn delete_resource(&self, id: &str) -> Result<(), CoreError> {
        self.repository
            .delete_resource(crate::ResourceId::parse(id)?)
    }

    pub fn set_resource_workspace(
        &self,
        resource_id: &str,
        workspace_id: &str,
        linked: bool,
    ) -> Result<(), CoreError> {
        self.repository.set_resource_workspace(
            crate::ResourceId::parse(resource_id)?,
            crate::WorkspaceId::parse(workspace_id)?,
            linked,
        )
    }

    pub fn resource_workspaces(&self) -> Result<Vec<crate::ResourceWorkspace>, CoreError> {
        self.repository.resource_workspaces()
    }

    pub fn search(
        &self,
        query: &str,
        include_archived: bool,
        limit: usize,
    ) -> Result<Vec<crate::Resource>, CoreError> {
        self.repository.search_resources(SearchRequest {
            query: query.trim().to_owned(),
            include_archived,
            limit: limit.min(100),
        })
    }

    pub fn rebuild_search(&self) -> Result<usize, CoreError> {
        self.repository.rebuild_resource_search()
    }
}

impl AppService {
    pub fn new(repository: Arc<dyn AppRepository>) -> Self {
        Self { repository }
    }

    pub fn create_app(&self, input: CreateAppInput) -> Result<RegisteredApp, CoreError> {
        self.repository
            .create_app(NewRegisteredApp::create_with_source(
                &input.name,
                &input.description,
                input.source_url.as_deref(),
                input.launch_kind,
                input.launch_target.as_deref(),
            )?)
    }

    pub fn catalog(&self) -> Vec<crate::AppCatalogEntry> {
        crate::app_catalog()
    }

    pub fn register_catalog(&self, ids: &[String]) -> Result<Vec<RegisteredApp>, CoreError> {
        let catalog = crate::app_catalog();
        let mut selected = Vec::new();
        for id in ids {
            if selected
                .iter()
                .any(|entry: &crate::AppCatalogEntry| entry.id == *id)
            {
                continue;
            }
            let entry = catalog
                .iter()
                .find(|entry| entry.id == *id)
                .cloned()
                .ok_or_else(|| {
                    CoreError::new(
                        crate::ErrorCode::InvalidInput,
                        format!("App conhecido desconhecido: {id}."),
                        false,
                    )
                })?;
            selected.push(entry);
        }
        let apps = selected
            .into_iter()
            .map(crate::AppCatalogEntry::into_new_app)
            .collect::<Result<Vec<_>, _>>()?;
        self.repository.register_catalog_apps(apps)
    }

    pub fn update_app(&self, input: UpdateAppInput) -> Result<RegisteredApp, CoreError> {
        let validated = NewRegisteredApp::create_with_source(
            &input.name,
            &input.description,
            input.source_url.as_deref(),
            input.launch_kind,
            input.launch_target.as_deref(),
        )?;
        self.repository.update_app(
            AppId::parse(&input.id)?,
            &validated,
            AppCapabilities {
                can_open: input.can_open,
                can_read: input.can_read,
                can_write: input.can_write,
                can_automate: input.can_automate,
            },
        )
    }

    pub fn app(&self, id: &str) -> Result<RegisteredApp, CoreError> {
        self.repository.get_app(AppId::parse(id)?)
    }

    pub fn apps(&self, include_archived: bool) -> Result<Vec<RegisteredApp>, CoreError> {
        self.repository.apps(include_archived)
    }

    pub fn set_app_archived(&self, id: &str, archived: bool) -> Result<RegisteredApp, CoreError> {
        self.repository.set_app_lifecycle(
            AppId::parse(id)?,
            if archived {
                LifecycleState::Archived
            } else {
                LifecycleState::Active
            },
        )
    }

    pub fn mark_app_opened(&self, id: &str) -> Result<RegisteredApp, CoreError> {
        self.repository.mark_app_opened(AppId::parse(id)?)
    }

    pub fn search(
        &self,
        query: &str,
        include_archived: bool,
        limit: usize,
    ) -> Result<Vec<RegisteredApp>, CoreError> {
        self.repository.search_apps(SearchRequest {
            query: query.trim().to_owned(),
            include_archived,
            limit: limit.min(100),
        })
    }

    pub fn delete_app(&self, id: &str) -> Result<(), CoreError> {
        self.repository.delete_app(AppId::parse(id)?)
    }

    pub fn rebuild_search(&self) -> Result<usize, CoreError> {
        self.repository.rebuild_app_search()
    }
}

#[derive(Clone)]
pub struct WorkService {
    repository: Arc<dyn WorkRepository>,
}

impl WorkService {
    pub fn new(repository: Arc<dyn WorkRepository>) -> Self {
        Self { repository }
    }

    pub fn create_workspace(&self, input: CreateWorkspaceInput) -> Result<Workspace, CoreError> {
        self.repository
            .create_workspace(NewWorkspace::create(&input.name, &input.description)?)
    }

    pub fn update_workspace(&self, input: UpdateWorkspaceInput) -> Result<Workspace, CoreError> {
        let validated = NewWorkspace::create(&input.name, &input.description)?;
        self.repository.update_workspace(
            WorkspaceId::parse(&input.id)?,
            &validated.name,
            &validated.description,
        )
    }

    pub fn workspace(&self, id: &str) -> Result<Workspace, CoreError> {
        self.repository.get_workspace(WorkspaceId::parse(id)?)
    }

    pub fn workspaces(&self, include_archived: bool) -> Result<Vec<Workspace>, CoreError> {
        self.repository.workspaces(include_archived)
    }

    pub fn set_workspace_archived(&self, id: &str, archived: bool) -> Result<Workspace, CoreError> {
        self.repository.set_workspace_lifecycle(
            WorkspaceId::parse(id)?,
            if archived {
                LifecycleState::Archived
            } else {
                LifecycleState::Active
            },
        )
    }

    pub fn workspace_projects(
        &self,
        id: &str,
        include_archived: bool,
    ) -> Result<Vec<Project>, CoreError> {
        self.repository
            .workspace_projects(WorkspaceId::parse(id)?, include_archived)
    }

    pub fn workspace_apps(
        &self,
        id: &str,
        include_archived: bool,
    ) -> Result<Vec<RegisteredApp>, CoreError> {
        self.repository
            .workspace_apps(WorkspaceId::parse(id)?, include_archived)
    }

    pub fn project_workspaces(&self, id: &str) -> Result<Vec<Workspace>, CoreError> {
        self.repository.project_workspaces(ProjectId::parse(id)?)
    }

    pub fn app_workspaces(&self, id: &str) -> Result<Vec<Workspace>, CoreError> {
        self.repository.app_workspaces(AppId::parse(id)?)
    }

    pub fn set_project_workspace(
        &self,
        project_id: &str,
        workspace_id: &str,
        linked: bool,
    ) -> Result<(), CoreError> {
        self.repository.set_project_workspace(
            ProjectId::parse(project_id)?,
            WorkspaceId::parse(workspace_id)?,
            linked,
        )
    }

    pub fn set_app_workspace(
        &self,
        app_id: &str,
        workspace_id: &str,
        linked: bool,
    ) -> Result<(), CoreError> {
        self.repository.set_app_workspace(
            AppId::parse(app_id)?,
            WorkspaceId::parse(workspace_id)?,
            linked,
        )
    }

    pub fn delete_task(&self, id: &str) -> Result<(), CoreError> {
        self.repository.delete_task(TaskId::parse(id)?)
    }

    pub fn delete_project(&self, id: &str) -> Result<(), CoreError> {
        self.repository.delete_project(ProjectId::parse(id)?)
    }

    pub fn delete_workspace(&self, id: &str) -> Result<(), CoreError> {
        self.repository.delete_workspace(WorkspaceId::parse(id)?)
    }

    pub fn set_widget_hidden(
        &self,
        workspace_id: &str,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError> {
        self.repository
            .set_widget_hidden(WorkspaceId::parse(workspace_id)?, widget_id, hidden)
    }

    pub fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError> {
        self.repository.hidden_widgets()
    }

    pub fn create_project(&self, input: CreateProjectInput) -> Result<Project, CoreError> {
        self.repository.create_project(NewProject::create(
            &input.name,
            &input.description,
            &input.repository,
        )?)
    }

    pub fn update_project(&self, input: UpdateProjectInput) -> Result<Project, CoreError> {
        let validated = NewProject::create(&input.name, &input.description, &input.repository)?;
        self.repository.update_project(
            ProjectId::parse(&input.id)?,
            &validated.name,
            &validated.description,
            &validated.repository,
        )
    }

    pub fn project(&self, id: &str) -> Result<Project, CoreError> {
        self.repository.get_project(ProjectId::parse(id)?)
    }

    pub fn projects(&self, include_archived: bool) -> Result<Vec<Project>, CoreError> {
        self.repository.projects(include_archived)
    }

    pub fn set_project_archived(&self, id: &str, archived: bool) -> Result<Project, CoreError> {
        self.repository.set_project_lifecycle(
            ProjectId::parse(id)?,
            if archived {
                LifecycleState::Archived
            } else {
                LifecycleState::Active
            },
        )
    }

    pub fn create_task(&self, input: CreateTaskInput) -> Result<Task, CoreError> {
        let project_id = input
            .project_id
            .as_deref()
            .map(ProjectId::parse)
            .transpose()?;
        let task = NewTask::create(&input.title, &input.description, project_id)?;
        match input.source_capture_id {
            Some(capture_id) => self
                .repository
                .create_task_from_capture(CaptureId::parse(&capture_id)?, task),
            None => self.repository.create_task(task),
        }
    }

    pub fn update_task(&self, input: UpdateTaskInput) -> Result<Task, CoreError> {
        let project_id = input
            .project_id
            .as_deref()
            .map(ProjectId::parse)
            .transpose()?;
        let validated = NewTask::create(&input.title, &input.description, project_id)?;
        self.repository.update_task(
            TaskId::parse(&input.id)?,
            &validated.title,
            &validated.description,
            project_id,
        )
    }

    pub fn task(&self, id: &str) -> Result<Task, CoreError> {
        self.repository.get_task(TaskId::parse(id)?)
    }

    pub fn tasks(&self, include_archived: bool) -> Result<Vec<Task>, CoreError> {
        self.repository.tasks(include_archived)
    }

    pub fn set_task_state(&self, id: &str, state: TaskState) -> Result<Task, CoreError> {
        self.repository.set_task_state(TaskId::parse(id)?, state)
    }

    pub fn set_task_archived(&self, id: &str, archived: bool) -> Result<Task, CoreError> {
        self.repository.set_task_lifecycle(
            TaskId::parse(id)?,
            if archived {
                LifecycleState::Archived
            } else {
                LifecycleState::Active
            },
        )
    }

    pub fn search(
        &self,
        query: &str,
        include_archived: bool,
    ) -> Result<Vec<SearchItem>, CoreError> {
        self.repository.search_all(SearchRequest {
            query: query.trim().to_owned(),
            include_archived,
            limit: 100,
        })
    }

    pub fn rebuild_search(&self) -> Result<usize, CoreError> {
        self.repository.rebuild_all_search()
    }
}

/// Servico da conversa do Hermes.
///
/// Ele nao conhece a ponte e nao conhece rede. O orquestrador do desktop e quem
/// traduz `Outcome` em parte de mensagem e chama estes metodos — e e por isso
/// que `mos-hermes` continua sem `mos-core` e sem SQLite (ADR-024, ADR-025).
#[derive(Clone)]
pub struct ConversationService {
    repository: Arc<dyn ConversationRepository>,
}

impl ConversationService {
    pub fn new(repository: Arc<dyn ConversationRepository>) -> Self {
        Self { repository }
    }

    pub fn create(&self) -> Result<Conversation, CoreError> {
        self.repository
            .create_conversation(NewConversation::create())
    }

    pub fn get(&self, id: &str) -> Result<Conversation, CoreError> {
        self.repository.get_conversation(ConversationId::parse(id)?)
    }

    pub fn list(&self, include_archived: bool) -> Result<Vec<ConversationSummary>, CoreError> {
        self.repository.conversations(include_archived, 200)
    }

    /// A conversa mais recente, ou uma nova quando nao ha nenhuma.
    ///
    /// E o que a tela abre. Sem isto o app precisaria escolher entre comecar
    /// sempre do zero — perdendo a continuidade que a ADR-025 existe para dar —
    /// ou deixar o renderer decidir qual conversa e a corrente, que e regra de
    /// aplicacao e nao de apresentacao.
    pub fn current_or_new(&self) -> Result<Conversation, CoreError> {
        match self.repository.conversations(false, 1)?.first() {
            Some(summary) => self.repository.get_conversation(summary.id),
            None => self.create(),
        }
    }

    pub fn rename(&self, id: &str, title: &str) -> Result<Conversation, CoreError> {
        let title = validate_title(title)?;
        self.repository
            .set_conversation_title(ConversationId::parse(id)?, &title)
    }

    /// Guarda o vinculo com a sessao da VPS.
    pub fn bind_session(
        &self,
        id: &str,
        hermes_session_id: Option<&str>,
    ) -> Result<Conversation, CoreError> {
        self.repository
            .set_conversation_session(ConversationId::parse(id)?, hermes_session_id)
    }

    pub fn set_archived(&self, id: &str, archived: bool) -> Result<Conversation, CoreError> {
        self.repository.set_conversation_lifecycle(
            ConversationId::parse(id)?,
            if archived {
                LifecycleState::Archived
            } else {
                LifecycleState::Active
            },
        )
    }

    pub fn delete(&self, id: &str) -> Result<(), CoreError> {
        self.repository
            .delete_conversation(ConversationId::parse(id)?)
    }

    pub fn messages(&self, id: &str) -> Result<Vec<Message>, CoreError> {
        self.repository.messages(ConversationId::parse(id)?)
    }

    pub fn message(&self, id: &str) -> Result<Message, CoreError> {
        self.repository.message(MessageId::parse(id)?)
    }

    pub fn append_user_message(&self, id: &str, text: &str) -> Result<Message, CoreError> {
        let conversation_id = ConversationId::parse(id)?;
        self.repository
            .append_message(NewMessage::user(conversation_id, text)?)
    }

    /// Acrescenta partes a uma mensagem do usuario ja gravada.
    ///
    /// Serve para os chips de contexto: eles sao registrados junto da pergunta,
    /// e o registro precisa dizer o que EFETIVAMENTE foi enviado (ADR-027) —
    /// o que so se sabe depois de montar o bloco.
    pub fn attach_parts(
        &self,
        message_id: &str,
        status: MessageStatus,
        parts: Vec<PartBody>,
    ) -> Result<Message, CoreError> {
        self.repository
            .finish_message(MessageId::parse(message_id)?, status, parts)
    }

    pub fn start_answer(&self, id: &str) -> Result<Message, CoreError> {
        let conversation_id = ConversationId::parse(id)?;
        self.repository
            .append_message(NewMessage::pending_assistant(conversation_id))
    }

    /// Fecha a resposta com o que chegou.
    ///
    /// Uma escrita por mensagem, nunca por delta: sob `synchronous=FULL` um
    /// INSERT por token seria um fsync por token (ADR-017).
    pub fn finish_answer(
        &self,
        message_id: &str,
        status: MessageStatus,
        parts: Vec<PartBody>,
    ) -> Result<Message, CoreError> {
        self.repository
            .finish_message(MessageId::parse(message_id)?, status, parts)
    }

    pub fn note(&self, id: &str, text: &str) -> Result<Message, CoreError> {
        let conversation_id = ConversationId::parse(id)?;
        self.repository
            .append_message(NewMessage::system(conversation_id, text))
    }

    /// Descarta uma mensagem e tudo que veio depois. Regenerate e edicao.
    pub fn truncate_from(&self, message_id: &str) -> Result<(), CoreError> {
        self.repository.truncate_from(MessageId::parse(message_id)?)
    }

    /// Substitui a conversa local pelo historico da VPS.
    pub fn replace_with_history(
        &self,
        id: &str,
        messages: Vec<NewMessage>,
    ) -> Result<(), CoreError> {
        self.repository
            .replace_messages(ConversationId::parse(id)?, messages)
    }

    pub fn search(&self, query: &str) -> Result<Vec<ConversationSummary>, CoreError> {
        self.repository.search_conversations(SearchRequest {
            query: query.trim().to_owned(),
            include_archived: false,
            limit: 50,
        })
    }

    /// Reparo de abertura: mensagem que ficou em curso vira interrompida.
    pub fn settle_unfinished(&self) -> Result<usize, CoreError> {
        self.repository.settle_unfinished_messages()
    }

    pub fn rebuild_search(&self) -> Result<usize, CoreError> {
        self.repository.rebuild_conversation_search()
    }
}
