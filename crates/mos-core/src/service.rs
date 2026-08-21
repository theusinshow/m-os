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

    /// Monta um Reminder SEM gravar.
    ///
    /// Existe para quem precisa grava-lo dentro de outra transacao — a acao
    /// derivada de voz cria Task e Reminder juntos, e um `create_at` aqui
    /// abriria uma segunda escrita fora daquela transacao. O instante entre as
    /// duas e exatamente o que a atomicidade existe para nao ter.
    pub fn draft_at(
        &self,
        title: &str,
        body: &str,
        instant: time::OffsetDateTime,
        source: crate::ReminderSource,
    ) -> Result<crate::NewReminder, CoreError> {
        Ok(crate::NewReminder::at(title, body, instant, self.clock.as_ref())?.from_source(source))
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

/// Traduz o escopo que vem da interface para o do dominio.
///
/// O front nao tem `Option` no caminho de um seletor: "Todos" chega como string
/// vazia, porque e o valor que o botao carrega. Aqui essa string vira `None`, e
/// dai para baixo o escopo e um `Option` honesto. A traducao mora num lugar so
/// de proposito — espalhada, um `""` esqueceria de virar `None` e o arranjo de
/// "Todos" iria parar num Workspace de id invalido.
fn parse_scope(workspace: Option<&str>) -> Result<Option<crate::WorkspaceId>, CoreError> {
    match workspace.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Ok(Some(crate::WorkspaceId::parse(value)?)),
        None => Ok(None),
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
        workspace_id: Option<&str>,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError> {
        self.repository
            .set_widget_hidden(parse_scope(workspace_id)?, widget_id, hidden)
    }

    pub fn widget_placements(&self) -> Result<Vec<crate::WidgetPlacement>, CoreError> {
        self.repository.widget_placements()
    }

    /// Grava o arranjo das faixas que mudaram.
    ///
    /// Recebe a lista inteira porque a regra de o que acontece com quem estava
    /// na posicao ja e do front — e ele que conhece a faixa e o catalogo.
    pub fn set_widget_layout(
        &self,
        workspace: Option<&str>,
        placements: &[crate::WidgetPlacementInput],
    ) -> Result<Vec<crate::WidgetPlacement>, CoreError> {
        self.repository
            .set_widget_layout(parse_scope(workspace)?, placements)
    }

    /// Devolve uma Home ao desenho, apagando o arranjo dela.
    pub fn reset_widget_layout(
        &self,
        workspace: Option<&str>,
    ) -> Result<Vec<crate::WidgetPlacement>, CoreError> {
        self.repository
            .reset_widget_layout(parse_scope(workspace)?)
    }
    pub fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError> {
        self.repository.hidden_widgets()
    }

    /// As petalas fixadas. Repasse puro: a regra do leque — padrao de fabrica e
    /// geometria — mora em `lequePetalas.ts`, e uma segunda copia aqui e exatamente o
    /// que o `homeLayout.ts` conta ter dado errado com o arranjo da Home.
    pub fn radial_pins(&self) -> Result<Vec<crate::RadialPin>, CoreError> {
        self.repository.radial_pins()
    }

    pub fn set_radial_pin(
        &self,
        workspace: Option<&str>,
        pin: crate::RadialPinInput,
    ) -> Result<Vec<crate::RadialPin>, CoreError> {
        self.repository.set_radial_pin(parse_scope(workspace)?, pin)
    }

    pub fn clear_radial_pin(
        &self,
        workspace: Option<&str>,
        slot: i64,
    ) -> Result<Vec<crate::RadialPin>, CoreError> {
        self.repository
            .clear_radial_pin(parse_scope(workspace)?, slot)
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

    /// Task, Reminder e o processamento da Capture, numa transacao so.
    ///
    /// E o caminho das acoes derivadas de voz. O `title` vem de
    /// `voice::title_from`, e nao da fala inteira: a Capture guarda o que foi
    /// dito, e a Task guarda o que ha para fazer.
    pub fn create_task_from_capture_with_reminder(
        &self,
        capture_id: &str,
        title: &str,
        description: &str,
        project_id: Option<ProjectId>,
        reminder: Option<crate::NewReminder>,
    ) -> Result<(Task, Option<crate::Reminder>), CoreError> {
        let task = NewTask::create(title, description, project_id)?;
        self.repository.create_task_from_capture_with_reminder(
            CaptureId::parse(capture_id)?,
            task,
            reminder,
        )
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

    /// O usuario clicou em Pausar. Os dois canais param de escrever juntos, e o
    /// tempo pausado nao vira frame — logo nao vira duracao.
    pub fn pause(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.transition(id, crate::MeetingTransition::Pause)
    }

    pub fn resume(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        self.transition(id, crate::MeetingTransition::Resume)
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

    /// As anotacoes. Vazio e valido: apagar tudo e uma escolha.
    pub fn set_notes(&self, id: &str, notes: &str) -> Result<crate::Meeting, CoreError> {
        self.repository
            .set_meeting_notes(crate::MeetingId::parse(id)?, notes)
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

/// Voice Inbox: o ciclo de vida de uma nota de voz.
///
/// **Casca fina sobre o dominio.** Toda decisao — se a transicao e legitima, se
/// a transcricao vale, se o audio ainda e necessario — vive em `voice.rs` e e
/// testada la. O que sobra aqui e traduzir id de texto para id de dominio e
/// mandar gravar, que e exatamente o que os outros servicos fazem.
#[derive(Clone)]
pub struct VoiceService {
    notes: Arc<dyn crate::VoiceRepository>,
    clock: Arc<dyn crate::Clock>,
}

impl VoiceService {
    pub fn new(notes: Arc<dyn crate::VoiceRepository>, clock: Arc<dyn crate::Clock>) -> Self {
        Self { notes, clock }
    }

    /// Abre a nota ANTES de o microfone abrir.
    ///
    /// Mesma ordem do `meeting_start`, e pela mesma razao: se a captura falhar,
    /// existe uma nota em `recording` que a proxima abertura reconcilia — e uma
    /// gravacao sem linha no banco seria audio que ninguem encontraria.
    pub fn start(
        &self,
        project_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<crate::VoiceNote, CoreError> {
        let project_id = project_id
            .filter(|value| !value.trim().is_empty())
            .map(ProjectId::parse)
            .transpose()?;
        let task_id = task_id
            .filter(|value| !value.trim().is_empty())
            .map(TaskId::parse)
            .transpose()?;
        self.notes.create_note(crate::NewVoiceNote::create(
            self.clock.now(),
            project_id,
            task_id,
        ))
    }

    pub fn note(&self, id: &str) -> Result<crate::VoiceNote, CoreError> {
        self.notes.note(crate::VoiceNoteId::parse(id)?)
    }

    pub fn unfinished(&self) -> Result<Vec<crate::VoiceNote>, CoreError> {
        self.notes.unfinished_notes()
    }

    pub fn recorded(
        &self,
        id: &str,
        duration_ms: i64,
        peak_level: u64,
    ) -> Result<crate::VoiceNote, CoreError> {
        self.transition(
            id,
            crate::VoiceTransition::Recorded {
                duration_ms,
                peak_level,
            },
        )
    }

    pub fn transcribing(&self, id: &str) -> Result<crate::VoiceNote, CoreError> {
        self.transition(id, crate::VoiceTransition::Transcribing)
    }

    pub fn failed(&self, id: &str, message: &str) -> Result<crate::VoiceNote, CoreError> {
        self.transition(
            id,
            crate::VoiceTransition::Failed {
                message: message.to_owned(),
            },
        )
    }

    pub fn cancel(&self, id: &str) -> Result<crate::VoiceNote, CoreError> {
        self.transition(id, crate::VoiceTransition::Cancelled)
    }

    /// A transcricao vira Capture, e a nota fecha sobre ela.
    ///
    /// A Capture nasce com a transcricao INTEIRA. Titulo, prazo e Project sao
    /// leitura, e leitura nao substitui o que foi dito.
    pub fn captured(
        &self,
        id: &str,
        transcript: &str,
        provider: &str,
    ) -> Result<(crate::VoiceNote, Capture), CoreError> {
        let note = self.note(id)?;
        let capture = NewCapture::create(transcript, CaptureSource::Voice)?;
        let closed = crate::apply_voice(
            &note,
            crate::VoiceTransition::Captured {
                capture_id: capture.id,
                transcript: transcript.trim().to_owned(),
                provider: provider.to_owned(),
            },
            self.clock.now(),
        )?;
        self.notes.capture_note(&closed, capture)
    }

    pub fn mark_audio_deleted(&self, id: &str) -> Result<crate::VoiceNote, CoreError> {
        self.notes
            .mark_audio_deleted(crate::VoiceNoteId::parse(id)?, self.clock.now())
    }

    pub fn discard(&self, id: &str) -> Result<(), CoreError> {
        self.notes.delete_note(crate::VoiceNoteId::parse(id)?)
    }

    fn transition(
        &self,
        id: &str,
        transition: crate::VoiceTransition,
    ) -> Result<crate::VoiceNote, CoreError> {
        let note = self.note(id)?;
        let next = crate::apply_voice(&note, transition, self.clock.now())?;
        self.notes.save_note(&next)
    }
}

/// A Daily Session, do lado da aplicacao.
///
/// **Toda regra de dia vive aqui ou no `daily.rs`, e nunca num componente
/// React.** E o que faz a interface e o Hermes chegarem ao mesmo resultado: os
/// dois chamam este servico, e nao ha um segundo caminho que pudesse divergir.
///
/// O servico NAO monta o contexto do dia (`DailyContext`). Aquilo le Tasks,
/// Projects, Reminders, Captures e Meetings, e um servico que dependesse dos
/// cinco repositorios so para desenhar uma tela seria um servico que nao da
/// para instanciar sem o sistema inteiro. Quem le e o comando do desktop, que
/// chama a funcao pura `daily::compose_context` — o mesmo desenho do
/// `calendar::compose`.
pub struct DailyService {
    repository: Arc<dyn crate::DailyRepository>,
    clock: Arc<dyn crate::Clock>,
}

impl DailyService {
    pub fn new(
        repository: Arc<dyn crate::DailyRepository>,
        clock: Arc<dyn crate::Clock>,
    ) -> Self {
        Self { repository, clock }
    }

    /// O dia inteiro, do jeito que a Home le.
    pub fn today(&self, day: &crate::Day) -> Result<crate::DailyToday, CoreError> {
        let session = self.repository.session_on(day)?;
        let (status, objectives, reflection) = match &session {
            Some(session) => (
                session.status,
                self.repository.objectives(session.id)?,
                self.repository.reflection(session.id)?,
            ),
            None => (crate::SessionStatus::NotStarted, Vec::new(), None),
        };

        // A sessao velha so e procurada quando a de hoje AINDA nao existe.
        // Depois de o dia comecar ela ja foi fechada pelo `start_day`, e
        // continuar perguntando seria uma consulta por render sem resposta
        // possivel.
        let stale = match &session {
            Some(_) => None,
            None => self.repository.stale_session(day)?,
        };
        let stale_objectives = match &stale {
            Some(stale) => self.repository.objectives(stale.id)?,
            None => Vec::new(),
        };

        Ok(crate::DailyToday {
            day: day.clone(),
            status,
            session,
            objectives,
            reflection,
            stale,
            stale_objectives,
        })
    }

    /// A ultima sessao antes desta data, com os objetivos dela. Alimenta o
    /// carry-over do contexto.
    pub fn previous(
        &self,
        day: &crate::Day,
    ) -> Result<Option<(crate::DailySession, Vec<crate::DailyObjective>)>, CoreError> {
        let Some(session) = self.repository.session_before(day)? else {
            return Ok(None);
        };
        let objectives = self.repository.objectives(session.id)?;
        Ok(Some((session, objectives)))
    }

    pub fn carry_depth(&self, id: crate::DailyObjectiveId) -> usize {
        // Falha aqui vira zero, e nao erro: este numero e um adorno ao lado de
        // um titulo ("adiado 3 vezes"). Deixar o Start My Day inteiro cair
        // porque um contador nao pode ser lido seria trocar a feature por um
        // detalhe dela.
        self.repository.carry_depth(id).unwrap_or(0)
    }

    /// Comeca o dia.
    ///
    /// Os rascunhos ja chegam com titulo: quem resolve "o titulo da Task
    /// vinculada" e quem conhece o banco, e nao este servico.
    pub fn start(
        &self,
        day: crate::Day,
        input: &crate::StartDayInput,
    ) -> Result<crate::DailyToday, CoreError> {
        let now = self.clock.now();
        let session = crate::NewDailySession::create(day.clone(), &input.note, now)?;

        let mut objectives = Vec::new();
        if let Some(main) = &input.main {
            objectives.push(main.build(session.id, crate::ObjectivePriority::Main, 0, now)?);
        }
        for draft in &input.secondaries {
            let position = objectives.len() as i64;
            objectives.push(draft.build(
                session.id,
                crate::ObjectivePriority::Secondary,
                position,
                now,
            )?);
        }

        self.repository.start_day(session, objectives, now)?;
        self.today(&day)
    }

    /// Acrescenta um objetivo ao dia que ja comecou.
    pub fn add_objective(
        &self,
        day: &crate::Day,
        draft: &crate::ObjectiveDraft,
        priority: crate::ObjectivePriority,
    ) -> Result<crate::DailyToday, CoreError> {
        let session = self.open_session(day)?;
        let now = self.clock.now();
        let position = self
            .repository
            .objectives(session.id)?
            .iter()
            .map(|objective| objective.position)
            .max()
            .map_or(0, |last| last + 1);
        let objective = draft.build(session.id, priority, position, now)?;
        self.repository.add_objective(objective)?;
        self.today(day)
    }

    /// Muda titulo e descricao. Nao mexe em status nem em vinculo: sao gestos
    /// diferentes, com botoes diferentes, e junta-los aqui faria um salvar de
    /// formulario apagar um vinculo em silencio.
    pub fn update_objective(
        &self,
        id: crate::DailyObjectiveId,
        title: &str,
        description: &str,
    ) -> Result<crate::DailyObjective, CoreError> {
        let current = self.repository.objective(id)?;
        let draft = crate::NewDailyObjective::create(
            current.session_id,
            title,
            description,
            current.link.clone(),
            current.priority,
            current.position,
            current.created_at,
        )?;
        let next = crate::DailyObjective {
            title: draft.title,
            description: draft.description,
            updated_at: self.clock.now(),
            ..current
        };
        self.repository.save_objective(&next)
    }

    /// Concluir, carregar, largar ou devolver a pendente.
    ///
    /// `completed_at` e exclusivo de `completed`: entrar carimba, sair limpa.
    /// E a mesma regra que `tasks.completed_at` e `reminders.completed_at` ja
    /// seguem — e o que impede um objetivo devolvido a pendente de continuar
    /// dizendo a que horas foi concluido.
    pub fn set_objective_status(
        &self,
        id: crate::DailyObjectiveId,
        status: crate::ObjectiveStatus,
    ) -> Result<crate::DailyObjective, CoreError> {
        let current = self.repository.objective(id)?;
        let now = self.clock.now();
        let next = crate::DailyObjective {
            status,
            completed_at: (status == crate::ObjectiveStatus::Completed).then_some(now),
            updated_at: now,
            ..current
        };
        self.repository.save_objective(&next)
    }

    pub fn set_main(
        &self,
        id: crate::DailyObjectiveId,
    ) -> Result<Vec<crate::DailyObjective>, CoreError> {
        self.repository.set_main_objective(id, self.clock.now())
    }

    /// Rebaixa um objetivo a secundario.
    ///
    /// Existe para o desfazer de uma promocao num dia que NAO tinha principal:
    /// ali nao ha quem promover de volta, e a unica reversao honesta e tirar o
    /// peso de quem ganhou. Sem isto, desfazer daria ao dia um principal que ele
    /// nunca teve.
    pub fn set_secondary(
        &self,
        id: crate::DailyObjectiveId,
    ) -> Result<crate::DailyObjective, CoreError> {
        let current = self.repository.objective(id)?;
        let next = crate::DailyObjective {
            priority: crate::ObjectivePriority::Secondary,
            updated_at: self.clock.now(),
            ..current
        };
        self.repository.save_objective(&next)
    }

    pub fn remove_objective(&self, id: crate::DailyObjectiveId) -> Result<(), CoreError> {
        self.repository.remove_objective(id)
    }

    pub fn reorder(
        &self,
        session: crate::DailySessionId,
        order: &[crate::DailyObjectiveId],
    ) -> Result<Vec<crate::DailyObjective>, CoreError> {
        self.repository
            .reorder_objectives(session, order, self.clock.now())
    }

    /// Encerra o dia de hoje.
    pub fn end(
        &self,
        day: &crate::Day,
        input: &crate::EndDayInput,
    ) -> Result<crate::DailyToday, CoreError> {
        let session = self.open_session(day)?;
        self.end_session(session.id, input)?;
        self.today(day)
    }

    /// Encerra UMA sessao pelo id. E o caminho do "encerrar ontem", que nao
    /// pode passar por `day` — a sessao velha e de outra data, por definicao.
    pub fn end_session(
        &self,
        session: crate::DailySessionId,
        input: &crate::EndDayInput,
    ) -> Result<crate::DailySession, CoreError> {
        let resolutions = input.parsed_resolutions()?;
        let reflection = input
            .reflection()?
            .map(|reflection| reflection.for_session(session));
        self.repository
            .end_day(session, &resolutions, reflection, self.clock.now())
    }

    pub fn reopen(
        &self,
        session: crate::DailySessionId,
    ) -> Result<crate::DailySession, CoreError> {
        self.repository.reopen_day(session, self.clock.now())
    }

    /// O historico, com o placar de cada dia ja calculado.
    ///
    /// As sessoes numa consulta e os objetivos de todas elas noutra, em vez de
    /// uma consulta por dia listado. E a diferenca entre a tela abrir e a tela
    /// pensar.
    pub fn history(&self, limit: usize) -> Result<Vec<crate::DailySessionSummary>, CoreError> {
        let sessions = self.repository.sessions(limit)?;
        let ids: Vec<_> = sessions.iter().map(|session| session.id).collect();
        let objectives = self.repository.objectives_of(&ids)?;
        // Tres consultas para N dias, e nao 2N+1: as sessoes, os objetivos de
        // todas elas, e as reflexoes de todas elas. A versao anterior lia a
        // reflexao de cada dia numa consulta propria.
        let reflections = self.repository.reflections_of(&ids)?;
        Ok(sessions
            .into_iter()
            .map(|session| {
                let mine: Vec<_> = objectives
                    .iter()
                    .filter(|objective| objective.session_id == session.id)
                    .cloned()
                    .collect();
                let mood = reflections
                    .iter()
                    .find(|reflection| reflection.session_id == session.id)
                    .and_then(|reflection| reflection.mood);
                crate::summarize(session, &mine, mood)
            })
            .collect())
    }

    /// Uma sessao passada, inteira. E o que a tela de historico abre.
    pub fn detail(&self, session: crate::DailySessionId) -> Result<crate::DailyToday, CoreError> {
        let session = self.repository.session(session)?;
        Ok(crate::DailyToday {
            day: session.day.clone(),
            status: session.status,
            objectives: self.repository.objectives(session.id)?,
            reflection: self.repository.reflection(session.id)?,
            stale: None,
            stale_objectives: Vec::new(),
            session: Some(session),
        })
    }

    /// As sessoes cruas, sem placar e sem reflexao.
    ///
    /// Separada de [`Self::history`] de proposito: aquela le a reflexao de cada
    /// dia para saber o humor, o que e uma consulta por sessao. A Linha do Tempo
    /// so precisa das bordas, e trezentas e sessenta e cinco consultas de humor
    /// para desenhar um mes de calendario seriam N+1 pago por nada.
    pub fn sessions(&self, limit: usize) -> Result<Vec<crate::DailySession>, CoreError> {
        self.repository.sessions(limit)
    }


    /// A semana em narrativa, com o fecho dela quando existe.
    ///
    /// `project_of` entra por parametro porque so quem conhece Tasks e Projects
    /// consegue resolver o Project de um vinculo — e esse alguem e o comando do
    /// desktop, nao este servico.
    pub fn week(
        &self,
        week: &crate::Week,
        project_of: &dyn Fn(&crate::ObjectiveLink) -> Option<String>,
    ) -> Result<crate::WeekSummary, CoreError> {
        let sessions = self.repository.sessions_between(week)?;
        let ids: Vec<_> = sessions.iter().map(|session| session.id).collect();
        let objectives = self.repository.objectives_of(&ids)?;
        let reflections = self.repository.reflections_of(&ids)?;
        let depth = |id: crate::DailyObjectiveId| self.carry_depth(id);

        let mut summary = crate::compose_week(crate::WeekInput {
            week: week.clone(),
            sessions: &sessions,
            objectives: &objectives,
            reflections: &reflections,
            project_of,
            carry_depth: &depth,
        })?;
        summary.review = self.repository.weekly_review(week)?;
        Ok(summary)
    }

    /// A semana mais recente, anterior a corrente, que teve sessao e nao tem
    /// fecho.
    ///
    /// # Por que aqui, e nao em SQL
    ///
    /// Daria para derivar a segunda-feira com `date(day, 'weekday 0', '-6
    /// days')`. Seria a regra da semana escrita num segundo lugar — e e assim
    /// que o `arrange_widgets` do Rust ficou para tras em silencio, com os
    /// testes dele passando. `Week::containing` continua sendo a unica copia.
    pub fn pending_week(&self, current: &crate::Week) -> Result<Option<crate::Week>, CoreError> {
        use std::collections::HashSet;

        // 120 sessoes sao ~quatro meses de uso diario. Alem disso, uma semana
        // nao fechada deixou de ser pendencia e virou historico.
        let sessions = self.repository.sessions(120)?;
        let fechadas: HashSet<crate::Week> = self
            .repository
            .weekly_reviews(60)?
            .into_iter()
            .map(|review| review.week)
            .collect();

        let mut candidatas: Vec<crate::Week> = Vec::new();
        for session in &sessions {
            let semana = crate::Week::containing(&session.day)?;
            if semana < *current && !fechadas.contains(&semana) {
                candidatas.push(semana);
            }
        }
        Ok(candidatas.into_iter().max())
    }

    /// Fecha a semana, ou corrige o texto de um fecho que ja existe.
    pub fn close_week(
        &self,
        week: &crate::Week,
        summary: &str,
    ) -> Result<crate::WeeklyReview, CoreError> {
        let now = self.clock.now();
        self.repository.save_weekly_review(
            crate::NewWeeklyReview::create(week.clone(), summary, now),
            now,
        )
    }

    /// Os objetivos de varias sessoes, numa consulta. E o que a Linha do Tempo
    /// usa para nao fazer uma ida ao banco por dia desenhado.
    pub fn objectives_of(
        &self,
        sessions: &[crate::DailySessionId],
    ) -> Result<Vec<crate::DailyObjective>, CoreError> {
        self.repository.objectives_of(sessions)
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(crate::DailyObjective, crate::Day)>, CoreError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        self.repository.search_objectives(crate::SearchRequest {
            query: query.to_owned(),
            include_archived: false,
            limit,
        })
    }

    /// A sessao de hoje, exigindo que ela exista e esteja aberta.
    ///
    /// Mensagem propria em vez de `NotFound` cru: "o dia ainda nao comecou" e
    /// uma instrucao, e "sessao nao encontrada" e um erro de banco vazando para
    /// a tela.
    fn open_session(&self, day: &crate::Day) -> Result<crate::DailySession, CoreError> {
        match self.repository.session_on(day)? {
            Some(session) if session.status == crate::SessionStatus::Active => Ok(session),
            Some(_) => Err(CoreError::new(
                crate::ErrorCode::InvalidInput,
                "O dia ja foi encerrado. Reabra antes de mudar os objetivos.",
                false,
            )),
            None => Err(CoreError::new(
                crate::ErrorCode::InvalidInput,
                "O dia ainda nao comecou.",
                false,
            )),
        }
    }
}
