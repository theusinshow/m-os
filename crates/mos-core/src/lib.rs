mod app;
mod capture;
mod error;
mod functions;
mod ports;
mod resource;
mod service;
mod work;

pub use app::{
    app_catalog, validate_launch_target, validate_source_url, AppCapabilities, AppCatalogEntry,
    AppId, AppLaunchKind, NewRegisteredApp, RegisteredApp,
};
pub use capture::{Capture, CaptureId, CaptureSource, LifecycleState, NewCapture, ProcessingState};
pub use error::{CoreError, ErrorCode};
pub use functions::{
    function_registry, search_functions, FunctionCategory, FunctionConfirmation,
    FunctionDefinition, FunctionRisk,
};
pub use ports::{
    AppRepository, BackupInspection, BackupReceipt, CaptureRepository, DataMaintenance,
    ResourceRepository, SearchRequest, WorkRepository,
};
pub use resource::{
    validate_resource_url, NewResource, Resource, ResourceId, ResourceKind, ResourceWorkspace,
};
pub use service::{
    AppService, CaptureService, CreateAppInput, CreateCaptureInput, CreateProjectInput,
    CreateResourceInput, CreateTaskInput, CreateWorkspaceInput, DataService, MemoryService,
    UpdateAppInput, UpdateProjectInput, UpdateResourceInput, UpdateTaskInput, UpdateWorkspaceInput,
    WorkService,
};
pub use work::{
    validate_widget_id, HiddenWidget, NewProject, NewTask, NewWorkspace, Project, ProjectId,
    SearchItem, Task, TaskId, TaskState, Workspace, WorkspaceId,
};
