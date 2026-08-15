use std::path::Path;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    AppId, Capture, CaptureId, CoreError, HiddenWidget, LifecycleState, NewCapture, NewProject,
    NewRegisteredApp, NewTask, NewWorkspace, ProcessingState, Project, ProjectId, RegisteredApp,
    SearchItem, Task, TaskId, TaskState, Workspace, WorkspaceId,
};

#[derive(Clone, Debug)]
pub struct SearchRequest {
    pub query: String,
    pub include_archived: bool,
    pub limit: usize,
}

pub trait CaptureRepository: Send + Sync {
    fn create(&self, capture: NewCapture) -> Result<Capture, CoreError>;
    fn get(&self, id: CaptureId) -> Result<Capture, CoreError>;
    fn recent(&self, limit: usize) -> Result<Vec<Capture>, CoreError>;
    fn inbox(&self, limit: usize) -> Result<Vec<Capture>, CoreError>;
    fn by_lifecycle(
        &self,
        lifecycle: LifecycleState,
        limit: usize,
    ) -> Result<Vec<Capture>, CoreError>;
    fn search(&self, request: SearchRequest) -> Result<Vec<Capture>, CoreError>;
    fn set_processing_state(
        &self,
        id: CaptureId,
        state: ProcessingState,
    ) -> Result<Capture, CoreError>;
    fn set_lifecycle_state(
        &self,
        id: CaptureId,
        state: LifecycleState,
    ) -> Result<Capture, CoreError>;
    fn rebuild_search(&self) -> Result<usize, CoreError>;
}

pub trait WorkRepository: Send + Sync {
    fn create_workspace(&self, workspace: NewWorkspace) -> Result<Workspace, CoreError>;
    fn update_workspace(
        &self,
        id: WorkspaceId,
        name: &str,
        description: &str,
    ) -> Result<Workspace, CoreError>;
    fn get_workspace(&self, id: WorkspaceId) -> Result<Workspace, CoreError>;
    fn workspaces(&self, include_archived: bool) -> Result<Vec<Workspace>, CoreError>;
    fn set_workspace_lifecycle(
        &self,
        id: WorkspaceId,
        lifecycle: LifecycleState,
    ) -> Result<Workspace, CoreError>;
    fn workspace_projects(
        &self,
        id: WorkspaceId,
        include_archived: bool,
    ) -> Result<Vec<Project>, CoreError>;
    fn workspace_apps(
        &self,
        id: WorkspaceId,
        include_archived: bool,
    ) -> Result<Vec<RegisteredApp>, CoreError>;
    fn project_workspaces(&self, id: ProjectId) -> Result<Vec<Workspace>, CoreError>;
    fn app_workspaces(&self, id: AppId) -> Result<Vec<Workspace>, CoreError>;
    fn set_project_workspace(
        &self,
        project_id: ProjectId,
        workspace_id: WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError>;
    fn set_app_workspace(
        &self,
        app_id: AppId,
        workspace_id: WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError>;
    fn set_widget_hidden(
        &self,
        workspace_id: WorkspaceId,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError>;
    fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError>;
    fn create_project(&self, project: NewProject) -> Result<Project, CoreError>;
    fn update_project(
        &self,
        id: ProjectId,
        name: &str,
        description: &str,
        repository: &str,
    ) -> Result<Project, CoreError>;
    fn get_project(&self, id: ProjectId) -> Result<Project, CoreError>;
    fn projects(&self, include_archived: bool) -> Result<Vec<Project>, CoreError>;
    fn set_project_lifecycle(
        &self,
        id: ProjectId,
        lifecycle: LifecycleState,
    ) -> Result<Project, CoreError>;
    fn create_task(&self, task: NewTask) -> Result<Task, CoreError>;
    fn create_task_from_capture(
        &self,
        capture_id: CaptureId,
        task: NewTask,
    ) -> Result<Task, CoreError>;
    fn update_task(
        &self,
        id: TaskId,
        title: &str,
        description: &str,
        project_id: Option<ProjectId>,
    ) -> Result<Task, CoreError>;
    fn get_task(&self, id: TaskId) -> Result<Task, CoreError>;
    fn tasks(&self, include_archived: bool) -> Result<Vec<Task>, CoreError>;
    fn set_task_state(&self, id: TaskId, state: TaskState) -> Result<Task, CoreError>;
    fn set_task_lifecycle(&self, id: TaskId, lifecycle: LifecycleState) -> Result<Task, CoreError>;
    fn search_all(&self, request: SearchRequest) -> Result<Vec<SearchItem>, CoreError>;
    fn rebuild_all_search(&self) -> Result<usize, CoreError>;
}

pub trait AppRepository: Send + Sync {
    fn create_app(&self, app: NewRegisteredApp) -> Result<RegisteredApp, CoreError>;
    fn register_catalog_apps(
        &self,
        apps: Vec<NewRegisteredApp>,
    ) -> Result<Vec<RegisteredApp>, CoreError>;
    /// `fields` carrega os campos editaveis ja validados; so `id` e
    /// `created_at` sao ignorados. Agrupar aqui evita uma assinatura de oito
    /// parametros onde trocar dois `Option<&str>` de lugar compila em silencio.
    fn update_app(
        &self,
        id: AppId,
        fields: &crate::NewRegisteredApp,
        capabilities: crate::AppCapabilities,
    ) -> Result<RegisteredApp, CoreError>;
    fn get_app(&self, id: AppId) -> Result<RegisteredApp, CoreError>;
    fn apps(&self, include_archived: bool) -> Result<Vec<RegisteredApp>, CoreError>;
    fn set_app_lifecycle(
        &self,
        id: AppId,
        lifecycle: LifecycleState,
    ) -> Result<RegisteredApp, CoreError>;
    fn mark_app_opened(&self, id: AppId) -> Result<RegisteredApp, CoreError>;
    fn search_apps(&self, request: SearchRequest) -> Result<Vec<RegisteredApp>, CoreError>;
    fn rebuild_app_search(&self) -> Result<usize, CoreError>;
}

pub trait ResourceRepository: Send + Sync {
    fn create_resource(&self, resource: crate::NewResource) -> Result<crate::Resource, CoreError>;
    fn update_resource(
        &self,
        id: crate::ResourceId,
        kind: crate::ResourceKind,
        title: &str,
        url: &str,
        note: &str,
    ) -> Result<crate::Resource, CoreError>;
    fn get_resource(&self, id: crate::ResourceId) -> Result<crate::Resource, CoreError>;
    fn resources(&self, include_archived: bool) -> Result<Vec<crate::Resource>, CoreError>;
    fn trashed_resources(&self) -> Result<Vec<crate::Resource>, CoreError>;
    fn set_resource_lifecycle(
        &self,
        id: crate::ResourceId,
        lifecycle: LifecycleState,
    ) -> Result<crate::Resource, CoreError>;
    fn search_resources(&self, request: SearchRequest) -> Result<Vec<crate::Resource>, CoreError>;
    fn rebuild_resource_search(&self) -> Result<usize, CoreError>;
    fn set_resource_workspace(
        &self,
        resource_id: crate::ResourceId,
        workspace_id: crate::WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError>;
    fn resource_workspaces(&self) -> Result<Vec<crate::ResourceWorkspace>, CoreError>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupReceipt {
    pub path: String,
    pub bytes: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInspection {
    pub path: String,
    pub schema_version: u32,
    pub capture_count: u64,
    pub bytes: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

pub trait DataMaintenance: Send + Sync {
    fn create_backup(&self, destination: &Path) -> Result<BackupReceipt, CoreError>;
    fn inspect_backup(&self, source: &Path) -> Result<BackupInspection, CoreError>;
    fn restore_backup(&self, source: &Path) -> Result<BackupReceipt, CoreError>;
    fn ensure_daily_snapshot(&self) -> Result<Option<BackupReceipt>, CoreError>;
    fn export_json(&self, destination: &Path) -> Result<BackupReceipt, CoreError>;
}
