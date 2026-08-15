use std::{path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    AppCapabilities, AppId, AppLaunchKind, AppRepository, BackupInspection, BackupReceipt, Capture,
    CaptureId, CaptureRepository, CaptureSource, CoreError, DataMaintenance, HiddenWidget,
    LifecycleState, NewCapture, NewProject, NewRegisteredApp, NewTask, NewWorkspace,
    ProcessingState, Project, ProjectId, RegisteredApp, SearchItem, SearchRequest, Task, TaskId,
    TaskState, WorkRepository, Workspace, WorkspaceId,
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
        self.repository.delete_resource(crate::ResourceId::parse(id)?)
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
