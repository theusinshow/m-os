mod academic;
mod action;
mod agent;
mod attention;
mod app;
mod calendar;
mod capture;
mod clock;
mod conversation;
mod daily;
mod error;
mod functions;
mod ingestion;
mod meeting;
mod meeting_analysis;
mod monitoring;
mod ports;
mod resource;
mod service;
mod stale;
mod tracking;
mod voice;
mod voice_when;
mod weekly;
mod work;

pub use academic::{
    compose_dashboard, compose_today, desempenho, horizonte_de, nota_necessaria,
    progresso_do_periodo, segundos_na_semana, segundos_no_dia, semestre_corrente, validate_accent,
    AcademicDashboard, AcademicToday, Assignment, AssignmentId, AssignmentStatus, Compromisso,
    DashboardInput, Desempenho, Exam, ExamId, ExamStatus, Horizonte, Nota, Semester, SemesterId,
    SemesterStatus, StudySession, StudySessionId, StudySuggestion, Subject, SubjectId,
    SubjectOverview, MAX_UPCOMING, SUBJECT_ACCENTS,
};
pub use action::{
    action_contract, parse_action, parse_action_at, preview_of, ActionArgs, ActionAudit,
    ActionEffect, ActionKind, ActionLine, ActionPreview, TargetRef, TouchedEntity, UndoStep,
};
pub use agent::{
    candidates_block, here_block, normalize, now_block, parse_query, preamble, query_answer,
    query_contract, resolution_error, resolve, search_terms, short_id, spoken_moment, split_fenced,
    system_context, Candidate, EntityKind, Here, Named, PreambleInput, QueryRequest, Resolved,
    MAX_CANDIDATES, MAX_QUERY_HOPS,
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
    TurnEnding,
};
pub use daily::{
    compose_context, completes_with_task, summarize, CarryOver, ContextInput, DailyContext,
    DailyObjective, DailyObjectiveId, DailyReflection, DailySession, DailySessionId,
    DailySessionSummary, DailyToday, Day, DayMood, EndDayInput, LinkKind, NewDailyObjective,
    NewDailyReflection, NewDailySession, ObjectiveDraft, ObjectiveLink, ObjectivePriority,
    ObjectiveResolution, ObjectiveStatus, ProjectSuggestion, SessionStatus, StartDayInput,
    TaskSuggestion, SUGGESTED_SECONDARIES,
};
pub use error::{CoreError, ErrorCode};
pub use functions::{
    function_registry, search_functions, FunctionCategory, FunctionConfirmation,
    FunctionDefinition, FunctionRisk,
};
pub use ingestion::{
    capture_content, clamp_extracted_text, detect_kind, extension_of, host_of, image_size,
    is_openable, normalize_url, plan_relations, resolve_mime, sanitize_file_name, stored_path,
    title_from_text, DetectedKind, DropContext, ExtractionState, ImageSize, Ingestion, IngestionId,
    IngestionReceipt, IngestionSource, IngestionState, NewIngestion, ProjectHint, RelationDecision,
    RelationPlan, CONFIDENCE_LINK, CONFIDENCE_SUGGEST, MAX_EXTRACTED_CHARS, MAX_INGEST_BYTES,
};
pub use meeting::{
    apply as apply_meeting, interleave, AudioRetention, Channel as MeetingChannel, ChannelOutcome,
    Confidence, FailedStage, InsightId, InsightKind, InsightStatus, Meeting, MeetingAnalysis,
    MeetingEvidence, MeetingFailure, MeetingId, MeetingInsight, MeetingSource, MeetingStatus,
    clean_segments, is_speech, AcceptInsight, AcceptedInsight, InsightPreview, NewMeeting,
    RawSegment, SegmentId, TranscriptSegment,
    Transition as MeetingTransition, TranscriptionError, TranscriptionProvider,
    TranscriptionRequest,
};
pub use meeting_analysis::{
    build_windows, instructions, parse_analysis, AnalysisError, AnalysisOutcome, PromptWindow,
    Rejections, WINDOW_BUDGET_CHARS,
};
pub use monitoring::{
    decidir_oferta, diff_transitions, open_periods, uncovered, ActivityEvent, ActivityEventId,
    ActivityKind, ContextoDaOferta, DecisaoDeOferta, MicrofoneAberto,
    MonitoredApp, MonitoringSettings, NewActivityEvent, Period,
};
pub use ports::{
    AppRepository, AttentionRepository, BackupInspection, BackupReceipt, CaptureRepository,
    ConversationRepository, DailyRepository, DataMaintenance, IngestionRepository,
    MeetingRepository,
    MonitoringRepository, ResourceRepository, SearchRequest, TimeTrackingRepository,
    VoiceRepository,
    WorkRepository,
};
pub use resource::{
    validate_resource_url, NewResource, Resource, ResourceId, ResourceKind, ResourceProject,
    ResourceWorkspace,
};
pub use service::{
    AppService, AttentionService, AudioOutcome, CaptureService, ConversationService, CreateAppInput,
    DailyService,
    CreateCaptureInput, CreateProjectInput, CreateResourceInput, CreateTaskInput,
    CreateWorkspaceInput, DataService, MeetingService, MemoryService, MonitoringService,
    TrackingService, UpdateAppInput, UpdateProjectInput, UpdateResourceInput, UpdateTaskInput,
    UpdateWorkspaceInput, VoiceService, WorkService,
};
pub use stale::{
    atividade_do_project, compose_stale, project_activity, tolerancia, trabalho_aberto, Parada,
    ProjectActivity, StaleInput, StaleKind, StaleView, TOLERANCIA_PROJECT,
};
pub use voice::{
    apply as apply_voice, heard, is_hallucination, title_from, understand, Heard, NewVoiceNote,
    ProjectSource, Understanding, VoiceAction, VoiceContext, VoiceNote, VoiceNoteId,
    VoiceNoteStatus, VoiceTransition, MAX_DURATION_MS, MIN_DURATION_MS, MIN_PEAK_LEVEL,
};
pub use voice_when::{resolve_when, ResolvedWhen};
pub use tracking::{
    aggregate_by_project, amount_for_duration, billable_duration, elapsed_seconds, net_duration,
    parse_moment, round_duration, settle, ActiveTimer, ActivityType, Client, ClientId, ClientInput,
    EntrySource, Issuer, NewTimeEntry, ProjectTracking, ReportLine, Rounding, RoundingMode,
    StartTimer, TimeEntry, TimeEntryEdit, TimeEntryId, TimerSnapshot, TimerStatus, Totals,
    TrackedSession, TrackingSettings, TrackingStatus,
};
pub use weekly::{
    compose_week, Dominant, NewWeeklyReview, Recurring, Week, WeekInput, WeekSummary, WeeklyReview,
    WeeklyReviewId,
};
pub use work::{
    validate_pin_kind, validate_section_id, validate_span, validate_widget_id, HiddenWidget,
    NewProject, NewTask, NewWorkspace, Project, ProjectId, RadialPin, RadialPinInput, SearchItem,
    Task, TaskId, TaskState, WidgetPlacement, WidgetPlacementInput, Workspace, WorkspaceId,
};
