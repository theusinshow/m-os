mod action;
mod app;
mod capture;
mod conversation;
mod error;
mod functions;
mod ports;
mod resource;
mod service;
mod tracking;
mod work;

pub use action::{
    action_contract, parse_action, preview_of, ActionArgs, ActionEffect, ActionKind, ActionLine,
    ActionPreview, UndoStep,
};
pub use app::{
    app_catalog, validate_launch_target, validate_source_url, AppCapabilities, AppCatalogEntry,
    AppId, AppLaunchKind, NewRegisteredApp, RegisteredApp,
};
pub use capture::{Capture, CaptureId, CaptureSource, LifecycleState, NewCapture, ProcessingState};
pub use conversation::{
    validate_title, ContextEntity, ContextOrigin, Conversation, ConversationId,
    ConversationSummary, Message, MessageId, MessagePart, MessagePartId, MessageRole,
    MessageStatus, NewConversation, NewMessage, PartBody, ProposalStatus, ToolRunState,
};
pub use error::{CoreError, ErrorCode};
pub use functions::{
    function_registry, search_functions, FunctionCategory, FunctionConfirmation,
    FunctionDefinition, FunctionRisk,
};
pub use ports::{
    AppRepository, BackupInspection, BackupReceipt, CaptureRepository, ConversationRepository,
    DataMaintenance, ResourceRepository, SearchRequest, TimeTrackingRepository, WorkRepository,
};
pub use resource::{
    validate_resource_url, NewResource, Resource, ResourceId, ResourceKind, ResourceWorkspace,
};
pub use service::{
    AppService, CaptureService, ConversationService, CreateAppInput, CreateCaptureInput,
    CreateProjectInput, CreateResourceInput, CreateTaskInput, CreateWorkspaceInput, DataService,
    MemoryService, TrackingService, UpdateAppInput, UpdateProjectInput, UpdateResourceInput,
    UpdateTaskInput, UpdateWorkspaceInput, WorkService,
};
pub use tracking::{
    aggregate_by_project, amount_for_duration, billable_duration, elapsed_seconds, net_duration,
    round_duration, ActiveTimer, ActivityType, EntrySource, NewTimeEntry, ProjectTracking,
    Rounding, RoundingMode, StartTimer, TimeEntry, TimeEntryId, TimerSnapshot, TimerStatus, Totals,
    TrackedSession, TrackingSettings, TrackingStatus,
};
pub use work::{
    validate_widget_id, HiddenWidget, NewProject, NewTask, NewWorkspace, Project, ProjectId,
    SearchItem, Task, TaskId, TaskState, Workspace, WorkspaceId,
};
