mod action;
mod attention;
mod app;
mod calendar;
mod capture;
mod clock;
mod conversation;
mod error;
mod functions;
mod monitoring;
mod ports;
mod resource;
mod service;
mod tracking;
mod work;

pub use action::{
    action_contract, parse_action, preview_of, ActionArgs, ActionEffect, ActionKind, ActionLine,
    ActionPreview, UndoStep,
};
pub use attention::{
    apply, next_wake, reconcile, Channel, ContentPrivacy, DeliveryPolicy, NewNotification,
    NewReminder, Notification, NotificationId, NotificationStatus, Priority, Reconciliation,
    ReconcileReason, Reminder, ReminderId, ReminderSource, ReminderStatus, ReminderTarget,
    Transition, Trigger, VisualLevel, MISS_GRACE,
};
pub use app::{
    app_catalog, app_targeting_host, validate_launch_target, validate_source_url, AppCapabilities,
    AppCatalogEntry,
    AppId, AppLaunchKind, NewRegisteredApp, RegisteredApp,
};
pub use calendar::{compose, CalendarItem, CalendarKind, ComposeInput};
pub use clock::{Clock, FixedClock, SystemClock};
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
pub use monitoring::{
    diff_transitions, open_periods, uncovered, ActivityEvent, ActivityEventId, ActivityKind,
    MonitoredApp, MonitoringSettings, NewActivityEvent, Period,
};
pub use ports::{
    AppRepository, AttentionRepository, BackupInspection, BackupReceipt, CaptureRepository, ConversationRepository,
    DataMaintenance, MonitoringRepository, ResourceRepository, SearchRequest,
    TimeTrackingRepository, WorkRepository,
};
pub use resource::{
    validate_resource_url, NewResource, Resource, ResourceId, ResourceKind, ResourceWorkspace,
};
pub use service::{
    AppService, AttentionService, CaptureService, ConversationService, CreateAppInput, CreateCaptureInput,
    CreateProjectInput, CreateResourceInput, CreateTaskInput, CreateWorkspaceInput, DataService,
    MemoryService, MonitoringService, TrackingService, UpdateAppInput, UpdateProjectInput,
    UpdateResourceInput, UpdateTaskInput, UpdateWorkspaceInput, WorkService,
};
pub use tracking::{
    aggregate_by_project, amount_for_duration, billable_duration, elapsed_seconds, net_duration,
    parse_moment, round_duration, settle, ActiveTimer, ActivityType, Client, ClientId, ClientInput,
    EntrySource, Issuer, NewTimeEntry, ProjectTracking, ReportLine, Rounding, RoundingMode,
    StartTimer, TimeEntry, TimeEntryEdit, TimeEntryId, TimerSnapshot, TimerStatus, Totals,
    TrackedSession, TrackingSettings, TrackingStatus,
};
pub use work::{
    order_widgets, validate_widget_id, HiddenWidget, NewProject, NewTask, NewWorkspace, Project, ProjectId,
    SearchItem, Task, TaskId, TaskState, WidgetPosition, Workspace, WorkspaceId,
};
