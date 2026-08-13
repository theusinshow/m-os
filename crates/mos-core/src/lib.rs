mod app;
mod capture;
mod error;
mod ports;
mod service;
mod work;

pub use app::{validate_launch_target, AppId, AppLaunchKind, NewRegisteredApp, RegisteredApp};
pub use capture::{Capture, CaptureId, CaptureSource, LifecycleState, NewCapture, ProcessingState};
pub use error::{CoreError, ErrorCode};
pub use ports::{
    AppRepository, BackupInspection, BackupReceipt, CaptureRepository, DataMaintenance,
    SearchRequest, WorkRepository,
};
pub use service::{
    AppService, CaptureService, CreateAppInput, CreateCaptureInput, CreateProjectInput,
    CreateTaskInput, DataService, UpdateAppInput, UpdateProjectInput, UpdateTaskInput, WorkService,
};
pub use work::{NewProject, NewTask, Project, ProjectId, SearchItem, Task, TaskId, TaskState};
