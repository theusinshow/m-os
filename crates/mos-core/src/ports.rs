use std::path::Path;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    AppId, Capture, CaptureId, CoreError, LifecycleState, NewCapture, NewProject, NewRegisteredApp,
    NewTask, ProcessingState, Project, ProjectId, RegisteredApp, SearchItem, Task, TaskId,
    TaskState,
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
    fn create_project(&self, project: NewProject) -> Result<Project, CoreError>;
    fn update_project(
        &self,
        id: ProjectId,
        name: &str,
        description: &str,
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
    fn update_app(
        &self,
        id: AppId,
        name: &str,
        description: &str,
        launch_kind: Option<crate::AppLaunchKind>,
        launch_target: Option<&str>,
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
