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

    /// As Captures de uma janela, sem teto.
    ///
    /// Quem pede uma janela ja sabe o tamanho dela — limitar aqui seria o
    /// mesmo silencio que `recent` produz quando bate no teto.
    pub fn between(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
    ) -> Result<Vec<Capture>, CoreError> {
        self.repository.captures_between(since, until)
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

    pub fn trashed(&self) -> Result<Vec<TimeEntry>, CoreError> {
        self.repository.trashed_time_entries()
    }

    pub fn issuer(&self) -> Result<crate::Issuer, CoreError> {
        self.repository.issuer()
    }

    pub fn set_issuer(&self, issuer: crate::Issuer) -> Result<crate::Issuer, CoreError> {
        self.repository.set_issuer(issuer)
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

    /// As sessões de um período, cada uma com o que vale.
    ///
    /// O recorte é aplicado AQUI e não na tela porque o arredondamento acontece
    /// por sessão: filtrar depois de somar daria um total diferente de somar
    /// depois de filtrar, e o segundo e o certo.
    pub fn report(
        &self,
        since: Option<time::OffsetDateTime>,
        until: Option<time::OffsetDateTime>,
    ) -> Result<Vec<crate::ReportLine>, CoreError> {
        let rounding = self.settings()?.rounding;
        Ok(self
            .entries(None)?
            .into_iter()
            .filter(|entry| {
                since.is_none_or(|from| entry.started_at >= from)
                    && until.is_none_or(|to| entry.started_at <= to)
            })
            .map(|entry| crate::ReportLine {
                totals: crate::settle(
                    &TrackedSession {
                        project_id: entry.project_id.to_string(),
                        duration_seconds: entry.duration_seconds,
                        idle_seconds: entry.idle_seconds,
                        billable: entry.billable,
                        hourly_rate_snapshot_cents: entry.hourly_rate_snapshot_cents,
                    },
                    rounding,
                ),
                raw_amount_cents: crate::amount_for_duration(
                    crate::net_duration(entry.duration_seconds, entry.idle_seconds),
                    entry.hourly_rate_snapshot_cents,
                ),
                entry_id: entry.id,
                project_id: entry.project_id,
                started_at: entry.started_at,
                activity_type: entry.activity_type,
                source: entry.source,
                billable: entry.billable,
                description: entry.description,
                hourly_rate_snapshot_cents: entry.hourly_rate_snapshot_cents,
            })
            .collect())
    }
}

/// O que o sistema observa (ADR-032).
///
/// Servico proprio, e nao metodos no `TrackingService`, pelo mesmo motivo que
/// os repositorios sao dois: observacao nao vira hora sozinha, e manter os dois
/// separados torna essa fronteira visivel na assinatura em vez de depender de
/// alguem lembrar dela.
#[derive(Clone)]
/// O Attention System, do lado do dominio.
///
/// Toda mudanca de estado passa por aqui, e nunca pelo repositorio direto: e
/// este servico que garante a ordem "validar, persistir, so entao agendar" do
/// `ATTENTION-SYSTEM.md` §7.5. Um Reminder que existe no agendador e nao no
/// banco e um Reminder que o proximo restart apaga.
pub struct AttentionService {
    repository: Arc<dyn crate::AttentionRepository>,
    clock: Arc<dyn crate::Clock>,
}

impl AttentionService {
    pub fn new(
        repository: Arc<dyn crate::AttentionRepository>,
        clock: Arc<dyn crate::Clock>,
    ) -> Self {
        Self { repository, clock }
    }

    pub fn create_at(
        &self,
        title: &str,
        body: &str,
        instant: time::OffsetDateTime,
        target: Option<crate::ReminderTarget>,
        source: crate::ReminderSource,
    ) -> Result<crate::Reminder, CoreError> {
        let mut draft =
            crate::NewReminder::at(title, body, instant, self.clock.as_ref())?.from_source(source);
        if let Some(target) = target {
            draft = draft.with_target(target);
        }
        self.repository.create_reminder(draft)
    }

    pub fn reminder(&self, id: crate::ReminderId) -> Result<crate::Reminder, CoreError> {
        self.repository.reminder(id)
    }

    /// O que a superficie mostra.
    pub fn open(&self) -> Result<Vec<crate::Reminder>, CoreError> {
        self.repository.open_reminders()
    }

    /// O que o agendador precisa ver.
    pub fn waiting(&self) -> Result<Vec<crate::Reminder>, CoreError> {
        self.repository.waiting_reminders()
    }

    /// Quantos itens realmente esperam uma acao da pessoa (§21.1).
    pub fn needs_attention_count(&self) -> Result<usize, CoreError> {
        Ok(self
            .repository
            .open_reminders()?
            .iter()
            .filter(|reminder| reminder.status.needs_attention())
            .count())
    }

    /// Aplica uma transicao e grava.
    ///
    /// Le do banco antes de decidir, para nao decidir sobre um estado que a
    /// interface tinha em cache — a tela pode estar aberta desde antes de o
    /// lembrete vencer.
    pub fn transition(
        &self,
        id: crate::ReminderId,
        transition: crate::Transition,
    ) -> Result<crate::Reminder, CoreError> {
        let current = self.repository.reminder(id)?;
        let next = crate::apply(&current, transition, self.clock.now())?;
        self.repository.save_reminder(&next)
    }

    pub fn set_lifecycle(
        &self,
        id: crate::ReminderId,
        state: crate::LifecycleState,
    ) -> Result<crate::Reminder, CoreError> {
        self.repository.set_reminder_lifecycle(id, state)
    }

    /// Quando o agendador precisa acordar, se precisar.
    pub fn next_wake(&self) -> Result<Option<time::OffsetDateTime>, CoreError> {
        Ok(crate::next_wake(&self.repository.waiting_reminders()?))
    }

    /// O que venceu enquanto ninguem olhava, e o que acabou de vencer.
    ///
    /// Aplica as transicoes e devolve o que mudou, para quem chamou poder
    /// entregar. Idempotente: rodar duas vezes seguidas nao produz nada na
    /// segunda, porque a primeira tirou os Reminders do estado de espera.
    pub fn reconcile(&self) -> Result<Vec<(crate::Reminder, crate::ReconcileReason)>, CoreError> {
        let waiting = self.repository.waiting_reminders()?;
        let now = self.clock.now();
        let mut changed = Vec::new();

        for found in crate::reconcile(&waiting, now) {
            let current = self.repository.reminder(found.id)?;
            let transition = match found.reason {
                crate::ReconcileReason::DueNow => crate::Transition::Ring,
                crate::ReconcileReason::MissedWhileAway => crate::Transition::Miss,
            };
            let next = crate::apply(&current, transition, now)?;
            changed.push((self.repository.save_reminder(&next)?, found.reason));
        }

        Ok(changed)
    }

    /// Registra uma entrega, respeitando o dedupe (§17).
    ///
    /// Devolve `None` quando ja existe entrega viva com a mesma chave — e o que
    /// impede "Task atrasada" quatro vezes seguidas.
    pub fn queue_delivery(
        &self,
        reminder: crate::ReminderId,
        channel: crate::Channel,
        subject: &str,
        level: crate::VisualLevel,
    ) -> Result<Option<crate::Notification>, CoreError> {
        let key = crate::NewNotification::dedupe_key(subject, reminder);
        if self.repository.live_notification(&key)?.is_some() {
            return Ok(None);
        }
        let queued =
            crate::NewNotification::queued(reminder, channel, subject, level, self.clock.now());
        self.repository.record_notification(queued).map(Some)
    }

    /// Marca a entrega como entregue e conta no Reminder.
    pub fn mark_delivered(
        &self,
        notification: &crate::Notification,
    ) -> Result<crate::Notification, CoreError> {
        let mut next = notification.clone();
        next.status = crate::NotificationStatus::Delivered;
        next.delivered_at = Some(self.clock.now());
        let saved = self.repository.save_notification(&next)?;
        // A falha aqui NAO desfaz a entrega: o toast ja apareceu. Contar de
        // menos e melhor que afirmar que nao entregou o que entregou.
        let _ = self.transition(notification.reminder_id, crate::Transition::Deliver);
        Ok(saved)
    }

    /// Registra falha de canal.
    ///
    /// NAO mexe no Reminder: falha de entrega nunca resolve uma intencao (§27).
    pub fn mark_failed(
        &self,
        notification: &crate::Notification,
        reason: &str,
    ) -> Result<crate::Notification, CoreError> {
        let mut next = notification.clone();
        next.status = crate::NotificationStatus::Failed;
        next.failure = Some(reason.to_owned());
        next.resolved_at = Some(self.clock.now());
        self.repository.save_notification(&next)
    }
}

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

    pub fn settings(&self) -> Result<crate::MonitoringSettings, CoreError> {
        self.repository.monitoring_settings()
    }

    pub fn set_settings(
        &self,
        settings: crate::MonitoringSettings,
    ) -> Result<crate::MonitoringSettings, CoreError> {
        self.repository.set_monitoring_settings(settings)
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

    pub fn widget_positions(&self) -> Result<Vec<crate::WidgetPosition>, CoreError> {
        self.repository.widget_positions()
    }

    /// Grava a ordem de uma secao da Home.
    ///
    /// Recebe a lista inteira porque a regra de o que acontece com quem estava
    /// na posicao ja e do front — e ele que conhece a secao e o catalogo.
    pub fn set_widget_order(
        &self,
        workspace: &str,
        ordered: &[String],
    ) -> Result<Vec<crate::WidgetPosition>, CoreError> {
        self.repository
            .set_widget_order(crate::WorkspaceId::parse(workspace)?, ordered)
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

/// Camada de aplicacao do Meeting Agent.
///
/// Ela coordena a maquina de estados e a persistencia, e **nao conhece WASAPI,
/// Whisper nem o Hermes**. As tres portas dos estagios chegam nas fases delas;
/// o que existe aqui e o que sobrevive a todas: a reuniao, o estado dela e a
/// regra de quem pode transitar para onde.
pub struct MeetingService {
    repository: Arc<dyn crate::MeetingRepository>,
    clock: Arc<dyn crate::Clock>,
}

/// O que a captura mediu quando os arquivos fecharam.
///
/// Chega do adapter de audio, e a duracao vem em FRAMES GRAVADOS. O servico nao
/// a recalcula, e nao existe caminho aqui que a derive de `ended_at - started_at`
/// — seria justamente no caso em que um canal caiu que esse numero mentiria.
#[derive(Clone, Debug)]
pub struct AudioOutcome {
    pub duration_ms: i64,
    pub mic: crate::ChannelOutcome,
    pub system: crate::ChannelOutcome,
}

impl MeetingService {
    pub fn new(repository: Arc<dyn crate::MeetingRepository>, clock: Arc<dyn crate::Clock>) -> Self {
        Self { repository, clock }
    }

    /// Comeca a gravar.
    ///
    /// **Recusa quando ja existe uma gravacao em curso.** Recusar em vez de
    /// substituir e a mesma regra do cronometro em `TimeTrackingRepository`, e
    /// pelo mesmo motivo: encerrar a anterior por conta descartaria uma sessao
    /// que o usuario nao mandou descartar. Aqui o custo seria pior — dois
    /// gravadores disputando o mesmo dispositivo.
    pub fn start(
        &self,
        title: &str,
        project_id: Option<&str>,
    ) -> Result<crate::Meeting, CoreError> {
        if let Some(current) = self.recording()? {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidTransition,
                format!("Ja existe uma gravacao em curso: \"{}\".", current.title),
                false,
            ));
        }
        let project_id = project_id.map(crate::ProjectId::parse).transpose()?;
        self.repository.create_meeting(crate::NewMeeting::start(
            title,
            // `Manual` fixo, e nao um parametro. A §17.2 promete que nenhum
            // caminho de codigo inicia gravacao sem clique; um parametro de
            // origem abriria exatamente esse caminho.
            crate::MeetingSource::Manual,
            project_id,
            self.clock.now(),
        ))
    }

    /// A gravacao em curso, se houver.
    pub fn recording(&self) -> Result<Option<crate::Meeting>, CoreError> {
        Ok(self.repository.capturing_meetings()?.into_iter().next())
    }

    /// O usuario clicou em Parar. O audio ainda esta fechando.
    pub fn stop(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.transition(id, crate::MeetingTransition::Stop)
    }

    /// A captura fechou os arquivos e mediu o que gravou.
    pub fn settle_audio(
        &self,
        id: &str,
        outcome: AudioOutcome,
    ) -> Result<crate::Meeting, CoreError> {
        let meeting = self.repository.meeting(crate::MeetingId::parse(id)?)?;
        let mut settled = crate::apply_meeting(
            &meeting,
            crate::MeetingTransition::AudioSettled,
            self.clock.now(),
        )?;
        settled.duration_ms = outcome.duration_ms;
        settled.mic = outcome.mic;
        settled.system = outcome.system;

        // Os DOIS canais mudos e uma gravacao que nao existe. Ela nao vira
        // `Recorded`, porque `Recorded` promete audio processavel — e transcrever
        // silencio produziria uma reuniao vazia com cara de reuniao real.
        if !settled.mic.has_audio() && !settled.system.has_audio() {
            let mut failed = settled;
            failed.status = crate::MeetingStatus::Failed(crate::FailedStage::Audio);
            failed.failure = Some(crate::MeetingFailure {
                stage: crate::FailedStage::Audio,
                message: "Nenhum dos dois canais capturou audio.".into(),
            });
            return self.repository.save_meeting(&failed);
        }
        self.repository.save_meeting(&settled)
    }

    /// Reconcilia a abertura do M/OS.
    ///
    /// Uma reuniao em captura num processo recem-nascido significa,
    /// necessariamente, que o anterior morreu sem terminar. **Nada e apagado**:
    /// ela vira `Interrupted` com a duracao que o disco sustenta, e quem decide
    /// entre processar e descartar e a pessoa (§9.2).
    ///
    /// `recovered` diz, por reuniao, quanto de audio existe em disco. Reunioes
    /// ausentes do mapa recebem zero — e zero e um fato a mostrar, nao um motivo
    /// para apagar.
    pub fn reconcile_on_open(
        &self,
        recovered: &dyn Fn(&crate::Meeting) -> i64,
    ) -> Result<Vec<crate::Meeting>, CoreError> {
        let now = self.clock.now();
        let mut interrupted = Vec::new();
        for meeting in self.repository.capturing_meetings()? {
            let mut next =
                crate::apply_meeting(&meeting, crate::MeetingTransition::DetectInterrupted, now)?;
            next.duration_ms = recovered(&meeting);
            // O canal para de estar "capturando" — ninguem esta capturando. Se
            // ele tinha audio, ele o tem ate onde chegou.
            next.mic = settle_channel(next.mic, next.duration_ms);
            next.system = settle_channel(next.system, next.duration_ms);
            interrupted.push(self.repository.save_meeting(&next)?);
        }
        Ok(interrupted)
    }

    /// O usuario escolheu [Processar] numa reuniao recuperada.
    pub fn process_recovered(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.transition(id, crate::MeetingTransition::ProcessRecovered)
    }

    /// O usuario escolheu [Descartar].
    pub fn cancel(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.transition(id, crate::MeetingTransition::Cancel)
    }

    pub fn start_transcription(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.transition(id, crate::MeetingTransition::StartTranscription)
    }

    /// Grava a transcricao e fecha o estagio, numa ordem que importa.
    ///
    /// A transcricao entra ANTES da transicao de estado. Se a escrita falhar, a
    /// reuniao continua `Transcribing` e o retry a encontra; se o estado mudasse
    /// primeiro, uma falha deixaria uma reuniao `Transcribed` sem transcricao —
    /// que e a forma mais silenciosa de perder o trabalho.
    pub fn finish_transcription(
        &self,
        id: &str,
        segments: Vec<crate::TranscriptSegment>,
    ) -> Result<crate::Meeting, CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        self.repository.replace_transcript(meeting_id, segments)?;
        self.transition(id, crate::MeetingTransition::TranscriptionDone)
    }

    pub fn start_analysis(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.transition(id, crate::MeetingTransition::StartAnalysis)
    }

    /// Grava a analise e fecha o estagio. Mesma ordem, mesma razao.
    pub fn finish_analysis(
        &self,
        analysis: crate::MeetingAnalysis,
        insights: Vec<crate::MeetingInsight>,
    ) -> Result<crate::Meeting, CoreError> {
        let id = analysis.meeting_id;
        self.repository.replace_analysis(analysis, insights)?;
        self.transition(&id.to_string(), crate::MeetingTransition::AnalysisDone)
    }

    /// Registra uma falha de estagio, preservando o insumo do estagio anterior.
    pub fn fail(
        &self,
        id: &str,
        stage: crate::FailedStage,
        message: &str,
    ) -> Result<crate::Meeting, CoreError> {
        let meeting = self.repository.meeting(crate::MeetingId::parse(id)?)?;
        let mut failed = crate::apply_meeting(
            &meeting,
            crate::MeetingTransition::Fail(stage),
            self.clock.now(),
        )?;
        failed.failure = Some(crate::MeetingFailure {
            stage,
            // A mensagem e para a PESSOA, e nunca carrega texto de transcricao
            // (§16.3). Quem constroi a string e quem chama; o servico so a
            // guarda, e a guarda inteira para que o diagnostico nao vire
            // adivinhacao.
            message: message.trim().to_owned(),
        });
        self.repository.save_meeting(&failed)
    }

    pub fn retry(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.transition(id, crate::MeetingTransition::Retry)
    }

    pub fn meeting(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.repository.meeting(crate::MeetingId::parse(id)?)
    }

    pub fn meetings(&self, include_archived: bool) -> Result<Vec<crate::Meeting>, CoreError> {
        self.repository.meetings(include_archived)
    }

    pub fn transcript(&self, id: &str) -> Result<Vec<crate::TranscriptSegment>, CoreError> {
        self.repository.transcript(crate::MeetingId::parse(id)?)
    }

    pub fn analysis(&self, id: &str) -> Result<Option<crate::MeetingAnalysis>, CoreError> {
        self.repository.analysis(crate::MeetingId::parse(id)?)
    }

    pub fn insights(&self, id: &str) -> Result<Vec<crate::MeetingInsight>, CoreError> {
        self.repository.insights(crate::MeetingId::parse(id)?)
    }

    /// Os itens que podem virar Task num clique so.
    ///
    /// A regra mora no dominio (`eligible_for_bulk`), e o servico apenas a
    /// aplica. Duplicar o criterio aqui criaria duas definicoes de "elegivel",
    /// e a interface acabaria oferecendo um lote que a criacao recusa.
    pub fn bulk_candidates(&self, id: &str) -> Result<Vec<crate::MeetingInsight>, CoreError> {
        Ok(self
            .insights(id)?
            .into_iter()
            .filter(crate::MeetingInsight::eligible_for_bulk)
            .collect())
    }

    pub fn set_project(
        &self,
        id: &str,
        project_id: Option<&str>,
    ) -> Result<crate::Meeting, CoreError> {
        let project_id = project_id.map(crate::ProjectId::parse).transpose()?;
        self.repository
            .set_meeting_project(crate::MeetingId::parse(id)?, project_id)
    }

    pub fn set_title(&self, id: &str, title: &str) -> Result<crate::Meeting, CoreError> {
        self.repository
            .set_meeting_title(crate::MeetingId::parse(id)?, title)
    }

    pub fn set_lifecycle(
        &self,
        id: &str,
        lifecycle: LifecycleState,
    ) -> Result<crate::Meeting, CoreError> {
        self.repository
            .set_meeting_lifecycle(crate::MeetingId::parse(id)?, lifecycle)
    }

    /// *"Quais compromissos de reunioes eu ainda nao conclui?"*
    ///
    /// Responde por SQL, e nao por modelo. Onde a regra deterministica serve,
    /// ela ganha da IA (§15.3).
    pub fn open_commitments(&self) -> Result<Vec<crate::MeetingInsight>, CoreError> {
        self.repository.open_commitments()
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<crate::Meeting>, CoreError> {
        self.repository.search_meetings(crate::SearchRequest {
            query: query.to_owned(),
            include_archived: false,
            limit,
        })
    }

    pub fn search_transcripts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(crate::Meeting, String)>, CoreError> {
        self.repository.search_transcripts(crate::SearchRequest {
            query: query.to_owned(),
            include_archived: false,
            limit,
        })
    }

    /// As reunioes cujo audio ja pode sair do disco.
    ///
    /// O servico so LISTA. Quem apaga bytes e o adapter, e ele marca depois de
    /// apagar — a ordem importa: marcar antes deixaria uma reuniao dizendo que o
    /// audio sumiu com o audio ainda la, ocupando disco para sempre porque
    /// ninguem mais o procuraria.
    pub fn audio_to_clean(&self) -> Result<Vec<crate::Meeting>, CoreError> {
        self.repository
            .meetings_with_deletable_audio(self.clock.now())
    }

    pub fn mark_audio_deleted(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.repository
            .mark_audio_deleted(crate::MeetingId::parse(id)?, self.clock.now())
    }

    /// O preview de todos os itens de uma reuniao.
    ///
    /// A interface pede isto ANTES de qualquer criacao. Todo item mostra
    /// preview, inclusive os de confianca alta (§13.2).
    pub fn previews(&self, id: &str) -> Result<Vec<crate::InsightPreview>, CoreError> {
        Ok(self
            .insights(id)?
            .iter()
            .map(crate::MeetingInsight::preview)
            .collect())
    }

    /// Aceita um item: cria a Task, opcionalmente o Reminder, e liga os tres.
    ///
    /// **O Meeting Agent nao escreve em Tasks.** Ele monta um `NewTask` validado
    /// pelo mesmo construtor que a interface usa, e a escrita acontece numa
    /// transacao do repositorio. O caminho e o mesmo; o que muda e quem propos.
    pub fn accept_insight(
        &self,
        accept: crate::AcceptInsight,
    ) -> Result<crate::AcceptedInsight, CoreError> {
        let task = crate::NewTask::create(&accept.title, &accept.description, accept.project_id)?;

        let reminder = match accept.remind_at {
            Some(instant) => {
                // O corpo do lembrete cita a reuniao, e nao a Task: quando ele
                // tocar amanha as 9h, "de onde veio isto?" precisa ter resposta
                // sem abrir mais nada.
                let meeting = self.repository.meeting(
                    self.repository
                        .insights_meeting(accept.insight_id)?,
                )?;
                Some(
                    crate::NewReminder::at(
                        &accept.title,
                        &format!("Da reuniao \"{}\"", meeting.title),
                        instant,
                        self.clock.as_ref(),
                    )?
                    .with_target(crate::ReminderTarget::Task(task.id)),
                )
            }
            None => None,
        };

        self.repository.accept_insight(accept, task, reminder)
    }

    /// Descarta um item. Ele some da lista de propostas e continua no banco.
    pub fn dismiss_insight(&self, insight_id: &str) -> Result<crate::MeetingInsight, CoreError> {
        self.repository.set_insight_status(
            crate::InsightId::parse(insight_id)?,
            crate::InsightStatus::Dismissed,
        )
    }

    /// Devolve um item aceito ao estado de proposta.
    ///
    /// E a metade do desfazer que mora no dominio da reuniao; arquivar a Task e
    /// cancelar o Reminder acontecem pelos servicos deles.
    pub fn reopen_insight(&self, insight_id: &str) -> Result<crate::MeetingInsight, CoreError> {
        self.repository.set_insight_status(
            crate::InsightId::parse(insight_id)?,
            crate::InsightStatus::Proposed,
        )
    }

    fn transition(
        &self,
        id: &str,
        transition: crate::MeetingTransition,
    ) -> Result<crate::Meeting, CoreError> {
        let meeting = self.repository.meeting(crate::MeetingId::parse(id)?)?;
        let next = crate::apply_meeting(&meeting, transition, self.clock.now())?;
        self.repository.save_meeting(&next)
    }
}

/// Fecha um canal que ficou "capturando" quando o processo morreu.
///
/// `Capturing` num processo novo e mentira: ninguem esta capturando. O que ele
/// tinha, ele tem ate onde o disco alcanca.
fn settle_channel(outcome: crate::ChannelOutcome, duration_ms: i64) -> crate::ChannelOutcome {
    match outcome {
        crate::ChannelOutcome::Capturing if duration_ms > 0 => crate::ChannelOutcome::Captured,
        crate::ChannelOutcome::Capturing => crate::ChannelOutcome::Unavailable {
            reason: "A gravacao foi interrompida sem produzir audio.".into(),
        },
        outro => outro,
    }
}
