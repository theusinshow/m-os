use std::{path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    BackupInspection, BackupReceipt, Capture, CaptureId, CaptureRepository, CaptureSource,
    CoreError, DataMaintenance, LifecycleState, NewCapture, NewProject, NewTask, ProcessingState,
    Project, ProjectId, SearchItem, SearchRequest, Task, TaskId, TaskState, WorkRepository,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
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

#[derive(Clone)]
pub struct WorkService {
    repository: Arc<dyn WorkRepository>,
}

impl WorkService {
    pub fn new(repository: Arc<dyn WorkRepository>) -> Self {
        Self { repository }
    }

    pub fn create_project(&self, input: CreateProjectInput) -> Result<Project, CoreError> {
        self.repository
            .create_project(NewProject::create(&input.name, &input.description)?)
    }

    pub fn update_project(&self, input: UpdateProjectInput) -> Result<Project, CoreError> {
        let validated = NewProject::create(&input.name, &input.description)?;
        self.repository.update_project(
            ProjectId::parse(&input.id)?,
            &validated.name,
            &validated.description,
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
