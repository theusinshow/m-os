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

    /// Quanto vale a hora deste Project agora — a tarifa dele, ou a padrao.
    pub fn hourly_rate_for(&self, project: ProjectId) -> Result<i64, CoreError> {
        self.repository.hourly_rate_for_project(project)
    }

    /// Carimba a tarifa vigente nas horas que ficaram sem valor nenhum.
    ///
    /// Devolve quantas mudaram. Nao e automatico de proposito: reescrever
    /// snapshot e mexer no que ja foi faturado, e isso e um ato de quem esta
    /// olhando a tela, nao um efeito colateral de abrir o app.
    pub fn apply_default_rate_to_unpriced(&self) -> Result<usize, CoreError> {
        self.repository.apply_default_rate_to_unpriced()
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
/// O Attention System, do lado do dominio.
///
/// Toda mudanca de estado passa por aqui, e nunca pelo repositorio direto: e
/// este servico que garante a ordem "validar, persistir, so entao agendar" do
/// `ATTENTION-SYSTEM.md` §7.5. Um Reminder que existe no agendador e nao no
/// banco e um Reminder que o proximo restart apaga.
#[derive(Clone)]
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
    /// O historico: o que ja foi resolvido, do mais recente para tras.
    pub fn resolved(&self, limit: usize) -> Result<Vec<crate::Reminder>, CoreError> {
        self.repository.resolved_reminders(limit)
    }

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
    /// Edita um lembrete: titulo, corpo, hora ou prioridade.
    ///
    /// # Por que aqui, e nao numa rota
    ///
    /// Porque as duas telas editam o MESMO lembrete. Uma edicao que existisse so
    /// no `mos-web` seria uma operacao que o Desktop nao sabe fazer sobre um
    /// dado que ele tambem tem — e o pedido que originou isto era explicito em
    /// nao criar sistemas paralelos.
    ///
    /// Le do banco antes de aplicar, e nao confia no que a tela mandou: entre a
    /// tela abrir o lembrete e o toque em salvar, o outro aparelho pode ter
    /// mexido nele. A regra de conflito continua sendo do sync, por campo — o
    /// que esta funcao garante e que a edicao parte do que esta gravado.
    pub fn update(
        &self,
        id: crate::ReminderId,
        mudanca: crate::EditReminder,
    ) -> Result<crate::Reminder, CoreError> {
        let atual = self.repository.reminder(id)?;
        let novo = crate::edit(&atual, mudanca, self.clock.now())?;
        self.repository.save_reminder(&novo)
    }

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

    // ------------------------------------------------------------- criacao

    /// Tudo que se pode pedir ao criar um lembrete, num lugar so.
    ///
    /// Struct e nao oito parametros porque as tres superficies criam lembretes —
    /// desktop, web e Hermes — e uma assinatura posicional de oito campos e uma
    /// troca de dois booleanos esperando para acontecer.
    pub fn create(&self, pedido: crate::CreateReminder) -> Result<crate::Reminder, CoreError> {
        let now = self.clock.now();
        let mut draft = match pedido.at {
            Some(instant) => {
                crate::NewReminder::at(&pedido.title, &pedido.body, instant, self.clock.as_ref())?
            }
            // Sem hora: "algum dia". Nao e um lembrete pela metade — e a
            // resposta honesta para o que se quer nao esquecer sem se querer ser
            // interrompido.
            None => crate::NewReminder::someday(&pedido.title, &pedido.body, self.clock.as_ref())?,
        };

        draft = draft
            .from_source(pedido.source)
            .with_priority(pedido.priority);
        if let Some(target) = pedido.target {
            draft = draft.with_target(target);
        }
        if pedido.persistent {
            draft = draft.persisting();
        }
        if let Some(who) = pedido
            .waiting_for
            .as_deref()
            .filter(|w| !w.trim().is_empty())
        {
            draft = draft.following_up(who);
        }
        if let Some(recurrence) = pedido.recurrence {
            draft = draft.repeating(recurrence)?;
        }

        let id = draft.id;
        let criado = self.repository.create_reminder(draft)?;
        self.log(id, crate::ReminderEventKind::Created, now, None);

        // A pilha, se houver. Cada alerta e uma linha, e o lembrete continua um.
        if !pedido.leads.is_empty() {
            if let Some(prazo) = criado.next_due_at {
                self.build_stack(&criado, prazo, &pedido.leads, now)?;
                // O vencimento do lembrete passa a ser o PRIMEIRO alerta da
                // pilha: o agendador continua olhando uma coluna so, e a pilha
                // nao vaza para dentro dele.
                return self.sync_next_from_stack(id, now);
            }
        }

        Ok(criado)
    }

    /// Monta a pilha: um alerta no prazo mais um por adiantamento pedido.
    fn build_stack(
        &self,
        reminder: &crate::Reminder,
        prazo: time::OffsetDateTime,
        leads: &[u32],
        now: time::OffsetDateTime,
    ) -> Result<(), CoreError> {
        self.repository
            .create_trigger(crate::NewReminderTrigger::at_due(reminder.id, prazo, now))?;
        for minutos in leads {
            if let Some(alerta) = crate::NewReminderTrigger::lead(reminder.id, prazo, *minutos, now)
            {
                self.repository.create_trigger(alerta)?;
            }
        }
        Ok(())
    }

    /// Acrescenta um alerta a um lembrete que ja existe.
    pub fn add_trigger(
        &self,
        id: crate::ReminderId,
        instant: time::OffsetDateTime,
        lead_minutes: Option<u32>,
    ) -> Result<crate::Reminder, CoreError> {
        let now = self.clock.now();
        let reminder = self.repository.reminder(id)?;
        let novo = match (lead_minutes, reminder.next_due_at) {
            (Some(minutos), Some(prazo)) => {
                crate::NewReminderTrigger::lead(id, prazo, minutos, now).ok_or_else(|| {
                    CoreError::new(
                        crate::ErrorCode::InvalidInput,
                        "Esse adiantamento cairia no passado.",
                        false,
                    )
                })?
            }
            _ => crate::NewReminderTrigger::extra(id, instant, now),
        };
        self.repository.create_trigger(novo)?;
        self.sync_next_from_stack(id, now)
    }

    pub fn triggers(
        &self,
        id: crate::ReminderId,
    ) -> Result<Vec<crate::ReminderTrigger>, CoreError> {
        self.repository.triggers_for(id)
    }

    pub fn cancel_trigger(
        &self,
        reminder: crate::ReminderId,
        trigger: crate::ReminderTriggerId,
    ) -> Result<crate::Reminder, CoreError> {
        self.repository
            .set_trigger_status(trigger, crate::StackTriggerStatus::Cancelled, None)?;
        self.sync_next_from_stack(reminder, self.clock.now())
    }

    /// Alinha `next_due_at` com o proximo alerta pendente da pilha.
    ///
    /// So faz sentido para lembrete que ESPERA. Um lembrete ja vencido tem o
    /// proprio atraso a contar, e reescrever a coluna dele com o horario de um
    /// alerta futuro apagaria o tamanho desse atraso.
    fn sync_next_from_stack(
        &self,
        id: crate::ReminderId,
        now: time::OffsetDateTime,
    ) -> Result<crate::Reminder, CoreError> {
        let reminder = self.repository.reminder(id)?;
        let pilha = self.repository.triggers_for(id)?;
        let Some(proximo) = crate::next_pending_trigger(&pilha) else {
            return Ok(reminder);
        };
        if !reminder.status.is_waiting() {
            return Ok(reminder);
        }
        if reminder.next_due_at == Some(proximo.scheduled_at) {
            return Ok(reminder);
        }
        let mut novo = reminder;
        novo.next_due_at = Some(proximo.scheduled_at);
        novo.updated_at = now;
        self.repository.save_reminder(&novo)
    }

    // ------------------------------------------------------------ historico

    /// Registra um evento. **Nunca derruba a operacao que o gerou.**
    ///
    /// Um lembrete que falhasse ao ser concluido porque o historico nao coube no
    /// disco seria o sistema perdendo o essencial para salvar o acessorio. O
    /// erro vai para o log e a vida segue.
    fn log(
        &self,
        id: crate::ReminderId,
        kind: crate::ReminderEventKind,
        at: time::OffsetDateTime,
        detail: Option<String>,
    ) {
        let mut evento = crate::NewReminderEvent::new(id, kind, at);
        evento.detail = detail;
        if let Err(erro) = self.repository.record_event(evento) {
            eprintln!("[attention] historico nao gravou: {}", erro.message);
        }
    }

    pub fn history(
        &self,
        id: crate::ReminderId,
        limit: usize,
    ) -> Result<Vec<crate::ReminderEvent>, CoreError> {
        self.repository.events_for(id, limit)
    }

    // ------------------------------------------------------------ decisoes

    /// Adiar: empurra, conta fadiga e registra.
    pub fn snooze(
        &self,
        id: crate::ReminderId,
        until: time::OffsetDateTime,
    ) -> Result<crate::Reminder, CoreError> {
        let now = self.clock.now();
        let atualizado = self.transition(id, crate::Transition::Snooze { until })?;
        self.log(
            id,
            crate::ReminderEventKind::Snoozed,
            now,
            Some(format!("ate {until}")),
        );
        Ok(atualizado)
    }

    /// Remarcar: muda a hora PLANEJADA, e nao conta fadiga.
    ///
    /// A distincao e de produto e nao de banco: adiar quinze vezes e um sinal de
    /// que a pessoa nao esta conseguindo decidir; corrigir a hora que se digitou
    /// errado nao e. Colapsar as duas faria o sistema oferecer ajuda a quem nao
    /// precisa e calar para quem precisa.
    pub fn reschedule(
        &self,
        id: crate::ReminderId,
        instant: time::OffsetDateTime,
    ) -> Result<crate::Reminder, CoreError> {
        let now = self.clock.now();
        let atualizado = self.update(
            id,
            crate::EditReminder {
                instant: Some(instant),
                ..Default::default()
            },
        )?;
        self.log(
            id,
            crate::ReminderEventKind::Rescheduled,
            now,
            Some(format!("para {instant}")),
        );
        Ok(atualizado)
    }

    /// Concluir. Cancela a pilha pendente e, se houver serie, ja marca a proxima.
    pub fn complete(&self, id: crate::ReminderId) -> Result<crate::Reminder, CoreError> {
        let now = self.clock.now();
        let atualizado = self.transition(id, crate::Transition::Complete)?;
        // A pilha pendente morre junto: quatro alertas para algo ja resolvido
        // sao quatro interrupcoes que nao significam nada.
        let _ = self.repository.cancel_pending_triggers(id);
        self.log(id, crate::ReminderEventKind::Completed, now, None);
        if atualizado.status == crate::ReminderStatus::Scheduled {
            self.log(
                id,
                crate::ReminderEventKind::RecurrenceGenerated,
                now,
                atualizado.next_due_at.map(|proxima| proxima.to_string()),
            );
        }
        Ok(atualizado)
    }

    pub fn cancel(&self, id: crate::ReminderId) -> Result<crate::Reminder, CoreError> {
        let now = self.clock.now();
        let atualizado = self.transition(id, crate::Transition::Cancel)?;
        let _ = self.repository.cancel_pending_triggers(id);
        self.log(id, crate::ReminderEventKind::Cancelled, now, None);
        Ok(atualizado)
    }

    pub fn acknowledge(&self, id: crate::ReminderId) -> Result<crate::Reminder, CoreError> {
        let now = self.clock.now();
        let atualizado = self.transition(id, crate::Transition::Acknowledge)?;
        self.log(id, crate::ReminderEventKind::Acknowledged, now, None);
        Ok(atualizado)
    }

    // ------------------------------------------------------ needs attention

    /// O que esta sendo esquecido, e por que. Ver [`crate::needs_attention`].
    pub fn attention_list(
        &self,
    ) -> Result<Vec<(crate::Reminder, crate::AttentionItem)>, CoreError> {
        let abertos = self.repository.open_reminders()?;
        let now_local = self
            .clock
            .now()
            .to_offset(self.repository.attention_settings()?.offset());
        let itens = crate::needs_attention(&abertos, now_local);
        Ok(itens
            .into_iter()
            .filter_map(|item| {
                abertos
                    .iter()
                    .find(|reminder| reminder.id == item.reminder_id)
                    .cloned()
                    .map(|reminder| (reminder, item))
            })
            .collect())
    }

    // ------------------------------------------------------------ ajustes

    pub fn settings(&self) -> Result<crate::AttentionSettings, CoreError> {
        self.repository.attention_settings()
    }

    pub fn save_settings(
        &self,
        settings: crate::AttentionSettings,
    ) -> Result<crate::AttentionSettings, CoreError> {
        self.repository.save_attention_settings(settings)
    }

    // ------------------------------------------------------------- varredura

    /// Uma passada completa do agendador.
    ///
    /// **A unica fonte de regras sobre o que vence agora** (§56 do pedido): o
    /// que venceu, o que se perdeu, o que precisa insistir e o que a pilha
    /// dispara sai tudo daqui. O adaptador de plataforma so ENTREGA o que esta
    /// lista devolve — nao decide nada.
    ///
    /// Idempotente: rodar duas vezes seguidas nao produz nada na segunda, porque
    /// a primeira tirou os lembretes do estado de espera e reprogramou os
    /// re-alertas.
    pub fn sweep(&self) -> Result<Vec<crate::DueDelivery>, CoreError> {
        let now = self.clock.now();
        let mut saida: Vec<crate::DueDelivery> = Vec::new();
        let ajustes = self.repository.attention_settings()?;
        let (silencio, offset) = (ajustes.quiet, ajustes.offset());

        // 1. Os alertas de pilha que venceram. Antes da reconciliacao porque um
        //    alerta disparado muda o `next_due_at` do lembrete dono.
        for alerta in self.repository.triggers_due(now)? {
            self.repository.set_trigger_status(
                alerta.id,
                crate::StackTriggerStatus::Fired,
                Some(now),
            )?;
        }

        // 2. O que venceu, e o que se perdeu.
        for achado in crate::reconcile(&self.repository.waiting_reminders()?, now) {
            let atual = self.repository.reminder(achado.id)?;
            let transicao = match achado.reason {
                crate::ReconcileReason::DueNow => crate::Transition::Ring,
                crate::ReconcileReason::MissedWhileAway => crate::Transition::Miss,
            };
            let seguinte = crate::apply(&atual, transicao, now)?;
            let gravado = self.repository.save_reminder(&seguinte)?;
            self.log(
                achado.id,
                match achado.reason {
                    crate::ReconcileReason::DueNow => crate::ReminderEventKind::Triggered,
                    crate::ReconcileReason::MissedWhileAway => crate::ReminderEventKind::Missed,
                },
                now,
                None,
            );

            // A pilha ainda tem alerta pela frente? Entao este vencimento era um
            // AVISO ANTECIPADO, e nao o prazo: o lembrete volta a esperar.
            let gravado = self.rearm_from_stack(gravado, now)?;
            saida.push(crate::DueDelivery {
                reminder: gravado,
                reason: match achado.reason {
                    crate::ReconcileReason::DueNow => crate::DueReason::DueNow,
                    crate::ReconcileReason::MissedWhileAway => crate::DueReason::MissedWhileAway,
                },
            });
        }

        // 3. Quem ja pode insistir.
        for lembrete in self.repository.reminders_to_retry(now)? {
            let seguinte = crate::apply(&lembrete, crate::Transition::Escalate, now)?;
            let gravado = self.repository.save_reminder(&seguinte)?;
            self.log(
                gravado.id,
                crate::ReminderEventKind::Escalated,
                now,
                Some(format!("degrau {}", gravado.escalation_step)),
            );
            saida.push(crate::DueDelivery {
                reminder: gravado,
                reason: crate::DueReason::Retry,
            });
        }

        // 4. O silencio. Segura a ENTREGA, nunca a intencao: o lembrete continua
        //    vencido, continua contando o atraso e continua no Attention Center.
        let mut entregaveis = Vec::new();
        for pedido in saida {
            match silencio.defer(now, offset, pedido.reminder.priority) {
                None => entregaveis.push(pedido),
                Some(quando) => {
                    let adiado = crate::apply(
                        &pedido.reminder,
                        crate::Transition::Defer { until: quando },
                        now,
                    )?;
                    self.repository.save_reminder(&adiado)?;
                }
            }
        }

        Ok(entregaveis)
    }

    /// Depois de um vencimento, devolve o lembrete a espera se a pilha ainda tem
    /// alerta pela frente.
    fn rearm_from_stack(
        &self,
        reminder: crate::Reminder,
        now: time::OffsetDateTime,
    ) -> Result<crate::Reminder, CoreError> {
        let pilha = self.repository.triggers_for(reminder.id)?;
        if pilha.is_empty() {
            return Ok(reminder);
        }
        let Some(proximo) = crate::next_pending_trigger(&pilha) else {
            // Era o ultimo alerta: agora o lembrete fica cobrando de verdade.
            return Ok(reminder);
        };
        let mut novo = reminder;
        novo.status = crate::ReminderStatus::Scheduled;
        novo.next_due_at = Some(proximo.scheduled_at);
        novo.retry_at = None;
        novo.escalation_step = 0;
        novo.updated_at = now;
        self.repository.save_reminder(&novo)
    }

    /// Marca a entrega, CONTA no Reminder e registra no historico.
    ///
    /// Use uma vez por RODADA de entrega, e nao uma vez por canal — ver
    /// [`Self::record_channel_delivery`].
    pub fn record_delivered(
        &self,
        notification: &crate::Notification,
    ) -> Result<crate::Notification, CoreError> {
        let entregue = self.mark_delivered(notification)?;
        self.log(
            notification.reminder_id,
            crate::ReminderEventKind::Delivered,
            self.clock.now(),
            Some(notification.channel.as_str().to_owned()),
        );
        Ok(entregue)
    }

    /// A MESMA cobranca saindo por um segundo canal.
    ///
    /// Marca a entrega e registra no historico, e **nao** conta no Reminder.
    ///
    /// # Por que a distincao existe
    ///
    /// `delivered_count` responde *"quantas vezes eu fui cobrado disto?"*, e a
    /// resposta certa para um toque que saiu no app E no Windows ao mesmo tempo
    /// e UMA. Contando por canal, o numero dobrava — e a regra de Needs
    /// Attention que chama de `ignored` quem foi avisado duas vezes passou a
    /// acusar de ignorado quem tinha sido avisado uma vez so. Apareceu na tela,
    /// em 2026-09-08, no primeiro lembrete que tocou depois de o canal do
    /// Windows entrar.
    pub fn record_channel_delivery(
        &self,
        notification: &crate::Notification,
    ) -> Result<crate::Notification, CoreError> {
        let mut proximo = notification.clone();
        proximo.status = crate::NotificationStatus::Delivered;
        proximo.delivered_at = Some(self.clock.now());
        let gravado = self.repository.save_notification(&proximo)?;
        self.log(
            notification.reminder_id,
            crate::ReminderEventKind::Delivered,
            self.clock.now(),
            Some(notification.channel.as_str().to_owned()),
        );
        Ok(gravado)
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub project_id: Option<String>,
    pub source_capture_id: Option<String>,
    /// Os campos da 0039. Todos `#[serde(default)]`: a criacao rapida continua
    /// mandando titulo e mais nada, e o payload dela nao muda de forma.
    #[serde(default)]
    pub due_at: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub estimate_minutes: Option<i64>,
    #[serde(default)]
    pub parent_task_id: Option<String>,
    /// Os passos com que a Task nasce, ja como linhas.
    #[serde(default)]
    pub checklist: Vec<String>,
}

/// A edicao de uma Task.
///
/// **Autoritativa, campo por campo** — `dueAt: null` significa *tire o prazo*,
/// e nao *nao mexi nisso*. Quem tem edicao parcial (o bolso manda so o que
/// mudou) le a Task e preenche o resto, que e o unico lugar onde essa regra
/// pode morar sem virar um `COALESCE` no banco que torna impossivel voltar
/// atras. E a mesma leitura que `WidgetPlacementInput` ja tinha.
impl UpdateTaskInput {
    /// A edicao que nao muda nada, a partir da Task como ela esta.
    ///
    /// Existe porque a escrita e autoritativa e a lista de campos cresceu de
    /// quatro para onze. Quem so quer mover o Project — o Hermes, o Undo — nao
    /// pode ser obrigado a repetir prazo, prioridade, estimativa, pai, bloqueio
    /// e waiting-for; e o dia em que esquecesse um deles, o campo esquecido
    /// seria APAGADO em silencio ao mover a Task de Project.
    pub fn from_task(task: &Task) -> Self {
        Self {
            id: task.id.to_string(),
            title: task.title.clone(),
            description: task.description.clone(),
            project_id: task.project_id.map(|id| id.to_string()),
            due_at: task.due_at.and_then(format_instant),
            priority: Some(task.priority.as_str().to_owned()),
            estimate_minutes: task.estimate_minutes,
            parent_task_id: task.parent_task_id.map(|id| id.to_string()),
            blocked_by_task_id: task.blocked_by_task_id.map(|id| id.to_string()),
            waiting_for: task.waiting_for.clone(),
            follow_up_at: task.follow_up_at.and_then(format_instant),
        }
    }
}

/// Um instante em RFC 3339, para voltar pelo mesmo caminho por onde entrou.
fn format_instant(instant: time::OffsetDateTime) -> Option<String> {
    instant
        .format(&time::format_description::well_known::Rfc3339)
        .ok()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub project_id: Option<String>,
    #[serde(default)]
    pub due_at: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub estimate_minutes: Option<i64>,
    #[serde(default)]
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub blocked_by_task_id: Option<String>,
    #[serde(default)]
    pub waiting_for: String,
    #[serde(default)]
    pub follow_up_at: Option<String>,
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

/// Um instante que pode nao existir.
///
/// Vazio e `None` sao a MESMA coisa aqui, de proposito: um `<input type=
/// "datetime-local">` limpo manda `""`, e tratar isso como data invalida faria
/// apagar o prazo devolver erro em vez de apagar o prazo.
fn parse_instant(value: Option<&str>) -> Result<Option<time::OffsetDateTime>, CoreError> {
    match value.map(str::trim) {
        None | Some("") => Ok(None),
        Some(texto) => crate::parse_moment(texto).map(Some),
    }
}

/// Ausente e `normal`, que e o valor neutro. Assim nenhuma superficie precisa
/// mandar prioridade para criar uma Task comum.
fn parse_priority(value: Option<&str>) -> Result<crate::Priority, CoreError> {
    match value.map(str::trim) {
        None | Some("") => Ok(crate::Priority::Normal),
        Some(texto) => crate::Priority::parse(texto),
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
        self.repository.reset_widget_layout(parse_scope(workspace)?)
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
        let task = NewTask::create(&input.title, &input.description, project_id)?
            .with_due_at(parse_instant(input.due_at.as_deref())?)
            .with_priority(parse_priority(input.priority.as_deref())?)
            .with_estimate(input.estimate_minutes)
            .with_parent(
                input
                    .parent_task_id
                    .as_deref()
                    .map(TaskId::parse)
                    .transpose()?,
            )
            .with_checklist(&input.checklist);
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
        let id = TaskId::parse(&input.id)?;
        self.repository.update_task(
            id,
            crate::EditTask {
                title: input.title,
                description: input.description,
                project_id: input
                    .project_id
                    .as_deref()
                    .map(ProjectId::parse)
                    .transpose()?,
                due_at: parse_instant(input.due_at.as_deref())?,
                priority: parse_priority(input.priority.as_deref())?,
                estimate_minutes: input.estimate_minutes,
                parent_task_id: input
                    .parent_task_id
                    .as_deref()
                    .map(TaskId::parse)
                    .transpose()?,
                blocked_by_task_id: input
                    .blocked_by_task_id
                    .as_deref()
                    .map(TaskId::parse)
                    .transpose()?,
                waiting_for: input.waiting_for,
                follow_up_at: parse_instant(input.follow_up_at.as_deref())?,
            },
        )
    }

    pub fn task(&self, id: &str) -> Result<Task, CoreError> {
        self.repository.get_task(TaskId::parse(id)?)
    }

    /// A Task com checklist, subtasks, referencias e lembretes.
    pub fn task_detail(&self, id: &str) -> Result<crate::TaskDetail, CoreError> {
        self.repository.task_detail(TaskId::parse(id)?)
    }

    pub fn tasks(&self, include_archived: bool) -> Result<Vec<Task>, CoreError> {
        self.repository.tasks(include_archived)
    }

    // ------------------------------------------------------------- checklist

    /// Acrescenta um passo. Devolve a TASK, porque quem escreveu precisa do
    /// progresso novo — e nao do item que ele acabou de digitar.
    pub fn add_checklist_item(&self, task_id: &str, label: &str) -> Result<Task, CoreError> {
        self.repository
            .add_checklist_item(crate::NewChecklistItem::create(
                TaskId::parse(task_id)?,
                label,
            )?)
    }

    /// O caminho da colagem: texto solto vira N itens, numa transacao so.
    ///
    /// A regra de "o que e uma linha" vive em `parse_checklist_lines`, no
    /// dominio, e nao em cada interface — senao o desktop e o bolso acabariam
    /// discordando sobre se `- item` tem hifen no texto.
    pub fn add_checklist_lines(&self, task_id: &str, text: &str) -> Result<Vec<Task>, CoreError> {
        let itens = crate::parse_checklist_lines(text);
        if itens.is_empty() {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidInput,
                "Nao ha nenhuma linha para virar item de checklist.",
                false,
            ));
        }
        let id = TaskId::parse(task_id)?;
        self.repository.add_checklist_items(id, &itens)?;
        Ok(vec![self.repository.get_task(id)?])
    }

    pub fn rename_checklist_item(
        &self,
        id: &str,
        label: &str,
    ) -> Result<crate::ChecklistItem, CoreError> {
        self.repository
            .rename_checklist_item(crate::ChecklistItemId::parse(id)?, label)
    }

    pub fn set_checklist_item_done(&self, id: &str, done: bool) -> Result<Task, CoreError> {
        self.repository
            .set_checklist_item_done(crate::ChecklistItemId::parse(id)?, done)
    }

    pub fn delete_checklist_item(&self, id: &str) -> Result<Task, CoreError> {
        self.repository
            .delete_checklist_item(crate::ChecklistItemId::parse(id)?)
    }

    pub fn reorder_checklist(
        &self,
        task_id: &str,
        ids: &[String],
    ) -> Result<Vec<crate::ChecklistItem>, CoreError> {
        let ids = ids
            .iter()
            .map(|id| crate::ChecklistItemId::parse(id))
            .collect::<Result<Vec<_>, _>>()?;
        self.repository
            .reorder_checklist(TaskId::parse(task_id)?, &ids)
    }

    pub fn set_task_reference(
        &self,
        task_id: &str,
        resource_id: &str,
        linked: bool,
    ) -> Result<(), CoreError> {
        self.repository.set_task_reference(
            TaskId::parse(task_id)?,
            crate::ResourceId::parse(resource_id)?,
            linked,
        )
    }

    pub fn set_task_state(&self, id: &str, state: TaskState) -> Result<Task, CoreError> {
        self.repository.set_task_state(TaskId::parse(id)?, state)
    }

    /// Planeja para um dia. `adiando` marca o gesto como adiamento.
    pub fn plan_task(
        &self,
        id: &str,
        scheduled_for: Option<crate::Day>,
        adiando: bool,
    ) -> Result<Task, CoreError> {
        self.repository
            .plan_task(TaskId::parse(id)?, scheduled_for, adiando)
    }

    /// "Comecar": registra o instante e poe em `doing`.
    pub fn start_task(&self, id: &str, now: time::OffsetDateTime) -> Result<Task, CoreError> {
        self.repository
            .set_task_started(TaskId::parse(id)?, Some(now))
    }

    /// "Continuar depois": limpa o instante sem mexer no estado.
    pub fn stop_task(&self, id: &str) -> Result<Task, CoreError> {
        self.repository.set_task_started(TaskId::parse(id)?, None)
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
#[derive(Clone)]
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
    pub fn new(
        repository: Arc<dyn crate::MeetingRepository>,
        clock: Arc<dyn crate::Clock>,
    ) -> Self {
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
        self.start_with(
            title,
            project_id,
            crate::MeetingSource::Manual,
            None,
            crate::AudioRetention::default(),
        )
    }

    /// Comeca a gravar, com o que a V2 sabe no clique.
    ///
    /// `source` distingue o clique na pagina do clique na oferta de deteccao —
    /// **os dois sao cliques**. Nao existe origem que nao seja uma pessoa
    /// autorizando (§17.2). `associated_app` e o programa que tinha o microfone;
    /// o Guardian o acompanha para perceber quando a chamada acabou.
    pub fn start_with(
        &self,
        title: &str,
        project_id: Option<&str>,
        source: crate::MeetingSource,
        associated_app: Option<String>,
        retention: crate::AudioRetention,
    ) -> Result<crate::Meeting, CoreError> {
        if let Some(current) = self.recording()? {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidTransition,
                format!("Ja existe uma gravacao em curso: \"{}\".", current.title),
                false,
            ));
        }
        let project_id = project_id.map(crate::ProjectId::parse).transpose()?;
        self.repository.create_meeting(
            crate::NewMeeting::start(title, source, project_id, self.clock.now())
                .with_associated_app(associated_app)
                .with_retention(retention),
        )
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

    /// Apaga a reuniao de vez. Devolve onde o audio dela estava.
    ///
    /// # Por que isto existe ao lado de arquivar
    ///
    /// `set_lifecycle(Archived)` guarda; isto some. Sao pedidos diferentes, e
    /// ate 2026-08-25 so o primeiro tinha resposta — a tela de Reunioes oferecia
    /// arquivar e descartar, e nenhum dos dois faz o que alguem quer dizer com
    /// "apaga essa reuniao". `cancel` nem sequer se aplica: ele e uma TRANSICAO
    /// da maquina de estados, valido so em `interrupted`, e uma reuniao pronta
    /// que a pessoa quer fora nao passa por ali.
    ///
    /// O caso real e a reuniao que nunca deveria ter existido: gravacao aberta
    /// por engano, teste de microfone, deteccao que pegou o filme errado.
    /// Arquivar uma dessas e guardar lixo com carinho.
    ///
    /// # A unica recusa
    ///
    /// **Uma gravacao em curso nao se apaga.** Nao por preciosismo: o gravador
    /// esta com arquivos abertos naquele diretorio neste instante, e apagar a
    /// linha do banco por baixo dele deixaria a thread de captura escrevendo
    /// para uma reuniao que nao existe — o `settle_audio` seguinte falharia sem
    /// ninguem entender por que. Parar primeiro e uma ordem, e nao um estorvo.
    pub fn delete(&self, id: &str) -> Result<String, CoreError> {
        let meeting = self.meeting(id)?;
        if meeting.status.is_capturing() {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidTransition,
                "Pare a gravacao antes de apagar esta reuniao.",
                false,
            ));
        }
        self.repository.delete_meeting(meeting.id)
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
        // Um item e um lote de um. O caminho e o mesmo, e com ele vem o que o
        // lote sabe fazer: prazo nativo, espera por outra pessoa e o vinculo com
        // uma Task igual que ja exista.
        self.accept_batch(vec![accept])?
            .pop()
            .ok_or_else(|| CoreError::new(crate::ErrorCode::NotFound, "Nada foi aceito.", false))
    }

    /// O caminho da V1, mantido para quem ainda o chama pelo repositorio.
    #[allow(dead_code)]
    fn accept_insight_single(
        &self,
        accept: crate::AcceptInsight,
    ) -> Result<crate::AcceptedInsight, CoreError> {
        let task = crate::NewTask::create(&accept.title, &accept.description, accept.project_id)?
            .with_due_at(accept.due_at);

        let reminder = match accept.remind_at {
            Some(instant) => {
                // O corpo do lembrete cita a reuniao, e nao a Task: quando ele
                // tocar amanha as 9h, "de onde veio isto?" precisa ter resposta
                // sem abrir mais nada.
                let meeting = self
                    .repository
                    .meeting(self.repository.insights_meeting(accept.insight_id)?)?;
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

/// O que o Guardian sabia quando a gravacao parou.
#[derive(Clone, Debug, Default)]
pub struct GuardianSettle {
    pub associated_app: Option<String>,
    pub suggested_end_ms: Option<i64>,
    /// O corte seguro, quando o Guardian o sustenta (`confident_trim_end`).
    pub confident_trim_end_ms: Option<i64>,
}

/// O que mudou ao ajustar o corte.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimOutcome {
    pub meeting: crate::Meeting,
    /// O estagio que voltou para a fila, quando o corte exigiu.
    pub requeued: Option<crate::JobStage>,
    pub removed_segments: usize,
}

/// Uma linha da lista de reunioes, com o que a lista precisa mostrar.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingOverview {
    pub meeting: crate::Meeting,
    pub phase: crate::MeetingPhase,
    pub progress: crate::PipelineProgress,
    pub job: Option<crate::MeetingJob>,
    /// Acoes minhas ainda por revisar.
    pub pending_actions: usize,
    pub tasks_created: usize,
    pub decisions: usize,
    /// Compromissos de outras pessoas.
    pub waiting: usize,
    pub questions: usize,
    /// A frase para a pessoa quando algo precisa dela. Vazia quando nao.
    pub attention: String,
}

/// Quantos dias para tras a deduplicacao procura uma Task igual.
pub const DEDUPE_TASK_DAYS: i64 = 14;
/// Quantos dias para tras a abertura enfileira reunioes da V1 que ficaram paradas.
pub const LEGACY_REQUEUE_DAYS: i64 = 7;

impl MeetingService {
    // ------------------------------------------------------------------------
    // Parar
    // ------------------------------------------------------------------------

    /// A captura fechou, com o motivo da parada e o que o Guardian sabia.
    ///
    /// Alem do `settle_audio` da V1: grava `stop_reason`, guarda o fim provavel,
    /// aplica o corte automatico quando ele e seguro (e estica a retencao para o
    /// desfazer ter audio), le os marcadores das notas e enfileira a
    /// transcricao. **Parar e o unico gesto; o resto anda sozinho.**
    pub fn settle_recording(
        &self,
        id: &str,
        outcome: AudioOutcome,
        reason: crate::StopReason,
        guardian: GuardianSettle,
    ) -> Result<crate::Meeting, CoreError> {
        let mut settled = self.settle_audio(id, outcome)?;
        settled.stop_reason = Some(reason);
        if settled.associated_app.is_none() {
            settled.associated_app = guardian.associated_app;
        }
        settled.suggested_end_ms = guardian.suggested_end_ms;
        if let Some(end) = guardian
            .confident_trim_end_ms
            .filter(|end| *end > 0 && *end < settled.duration_ms)
        {
            settled.trim_end_ms = Some(end);
            settled.trim_origin = Some(crate::TrimOrigin::Auto);
            if settled.retention == crate::AudioRetention::DeleteAfterProcessing {
                // O corte automatico precisa de volta. Sem audio, "incluir de
                // novo" seria um botao que nao faz nada.
                settled.retention = crate::AudioRetention::Keep24h;
            }
        }
        let settled = self.repository.save_meeting(&settled)?;
        self.apply_written_insights(id)?;
        if settled.status == crate::MeetingStatus::Recorded {
            self.queue(id, crate::JobStage::Transcription)?;
        }
        Ok(settled)
    }

    // ------------------------------------------------------------------------
    // Pipeline
    // ------------------------------------------------------------------------

    pub fn job(&self, id: &str) -> Result<Option<crate::MeetingJob>, CoreError> {
        self.repository.meeting_job(crate::MeetingId::parse(id)?)
    }

    pub fn jobs(&self) -> Result<Vec<crate::MeetingJob>, CoreError> {
        self.repository.meeting_jobs()
    }

    /// Poe um estagio na fila, reaproveitando a linha da reuniao.
    pub fn queue(&self, id: &str, stage: crate::JobStage) -> Result<crate::MeetingJob, CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        let now = self.clock.now();
        let job = match self.repository.meeting_job(meeting_id)? {
            Some(mut job) => {
                job.advance(stage, now);
                job
            }
            None => crate::MeetingJob::queue(meeting_id, stage, now),
        };
        self.repository.save_meeting_job(&job)?;
        Ok(job)
    }

    /// Os jobs que devem rodar agora, na ordem em que entraram.
    ///
    /// Reuniao fora de `active` nao roda: arquivada ainda pode rodar? Pode — o
    /// arquivo e da pessoa, o processamento e do sistema. Lixeira nao.
    pub fn due_jobs(&self) -> Result<Vec<crate::MeetingJob>, CoreError> {
        let now = self.clock.now();
        let mut due: Vec<crate::MeetingJob> = self
            .repository
            .meeting_jobs()?
            .into_iter()
            .filter(|job| job.is_due(now))
            .collect();
        due.retain(|job| {
            self.repository
                .meeting(job.meeting_id)
                .map(|meeting| meeting.lifecycle_state != crate::LifecycleState::Trashed)
                .unwrap_or(false)
        });
        due.sort_by_key(|job| job.updated_at);
        Ok(due)
    }

    /// Comeca o estagio do job: marca `running` e move a reuniao.
    pub fn begin_job(&self, id: &str) -> Result<(crate::Meeting, crate::MeetingJob), CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        let now = self.clock.now();
        let mut job = self.repository.meeting_job(meeting_id)?.ok_or_else(|| {
            CoreError::new(
                crate::ErrorCode::NotFound,
                "Nao ha job para esta reuniao.",
                false,
            )
        })?;
        let mut meeting = self.repository.meeting(meeting_id)?;

        // Um job que volta depois de desistir encontra a reuniao em `failed`:
        // o retry a devolve ao repouso antes de comecar.
        if matches!(meeting.status, crate::MeetingStatus::Failed(_)) {
            meeting = crate::apply_meeting(&meeting, crate::MeetingTransition::Retry, now)?;
        }
        // Interrompida com audio: processar e o caminho, e ele nao espera clique.
        if meeting.status == crate::MeetingStatus::Interrupted {
            meeting =
                crate::apply_meeting(&meeting, crate::MeetingTransition::ProcessRecovered, now)?;
        }
        let transition = match job.stage {
            crate::JobStage::Transcription => crate::MeetingTransition::StartTranscription,
            crate::JobStage::Analysis => crate::MeetingTransition::StartAnalysis,
        };
        let meeting = crate::apply_meeting(&meeting, transition, now)?;
        let meeting = self.repository.save_meeting(&meeting)?;
        job.start(now);
        self.repository.save_meeting_job(&job)?;
        Ok((meeting, job))
    }

    pub fn job_progress(&self, id: &str, fraction: f32) -> Result<(), CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        if let Some(mut job) = self.repository.meeting_job(meeting_id)? {
            job.set_progress(fraction, self.clock.now());
            self.repository.save_meeting_job(&job)?;
        }
        Ok(())
    }

    /// A transcricao terminou. Enfileira a analise quando ela e permitida.
    pub fn complete_transcription_job(
        &self,
        id: &str,
        segments: Vec<crate::TranscriptSegment>,
        analysis_allowed: bool,
    ) -> Result<crate::Meeting, CoreError> {
        let meeting = self.finish_transcription(id, segments)?;
        let meeting_id = meeting.id;
        let now = self.clock.now();
        let mut job = self.repository.meeting_job(meeting_id)?.unwrap_or_else(|| {
            crate::MeetingJob::queue(meeting_id, crate::JobStage::Transcription, now)
        });
        if analysis_allowed {
            job.advance(crate::JobStage::Analysis, now);
        } else {
            job.succeed(now);
        }
        self.repository.save_meeting_job(&job)?;
        Ok(meeting)
    }

    /// A analise terminou: grava, aplica titulo e Project quando cabem, fecha.
    pub fn complete_analysis_job(
        &self,
        analysis: crate::MeetingAnalysis,
        insights: Vec<crate::MeetingInsight>,
        title: Option<String>,
        project: Option<crate::meeting_text::ProjectInference>,
    ) -> Result<crate::Meeting, CoreError> {
        let meeting_id = analysis.meeting_id;
        let mut meeting = self.finish_analysis(analysis, insights)?;
        if let Some(title) = title {
            if crate::meeting_text::is_default_title(&meeting.title) {
                meeting = self.repository.set_meeting_title(meeting_id, &title)?;
            }
        }
        if let Some(project) = project {
            // So a confianca ALTA associa sozinha; media fica como sugestao na
            // tela. E so onde a pessoa ainda nao escolheu.
            if meeting.project_id.is_none() && project.confidence == crate::Confidence::High {
                meeting = self
                    .repository
                    .set_meeting_project(meeting_id, Some(project.project_id))?;
            }
        }
        if let Some(mut job) = self.repository.meeting_job(meeting_id)? {
            job.succeed(self.clock.now());
            self.repository.save_meeting_job(&job)?;
        }
        Ok(meeting)
    }

    /// Um estagio falhou. Decide, pela politica pura, entre esperar e desistir.
    pub fn fail_job(
        &self,
        id: &str,
        failure: &crate::StageFailure,
    ) -> Result<(crate::Meeting, crate::AfterFailure), CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        let now = self.clock.now();
        let mut job = self.repository.meeting_job(meeting_id)?.ok_or_else(|| {
            CoreError::new(
                crate::ErrorCode::NotFound,
                "Nao ha job para esta reuniao.",
                false,
            )
        })?;
        let after = job.fail(failure, now);
        self.repository.save_meeting_job(&job)?;

        let meeting = self.repository.meeting(meeting_id)?;
        let meeting = match after {
            crate::AfterFailure::RetryAt(_) | crate::AfterFailure::WaitForConfiguration => {
                match meeting.status {
                    crate::MeetingStatus::Transcribing | crate::MeetingStatus::Analyzing => {
                        self.repository.save_meeting(&crate::apply_meeting(
                            &meeting,
                            crate::MeetingTransition::Requeue,
                            now,
                        )?)?
                    }
                    _ => meeting,
                }
            }
            crate::AfterFailure::GiveUp => {
                let stage = job.stage.failed_stage();
                match meeting.status {
                    crate::MeetingStatus::Transcribing | crate::MeetingStatus::Analyzing => {
                        self.fail(id, stage, &failure.message)?
                    }
                    _ => meeting,
                }
            }
        };
        Ok((meeting, after))
    }

    /// A pessoa pediu para tentar de novo. Zera a contagem.
    pub fn retry_job(&self, id: &str) -> Result<crate::MeetingJob, CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        let now = self.clock.now();
        let meeting = self.repository.meeting(meeting_id)?;
        let mut job = match self.repository.meeting_job(meeting_id)? {
            Some(job) => job,
            None => {
                let stage = match meeting.status {
                    crate::MeetingStatus::Transcribed
                    | crate::MeetingStatus::Failed(crate::FailedStage::Analysis)
                    | crate::MeetingStatus::Ready => crate::JobStage::Analysis,
                    _ => crate::JobStage::Transcription,
                };
                crate::MeetingJob::queue(meeting_id, stage, now)
            }
        };
        // Uma analise que ja terminou pode ser pedida de novo ("analisar de
        // novo"): o estagio volta a ser o da analise.
        if job.status == crate::JobStatus::Done {
            job.advance(crate::JobStage::Analysis, now);
        }
        job.manual_retry(now);
        self.repository.save_meeting_job(&job)?;
        Ok(job)
    }

    /// A configuracao que faltava apareceu: libera os jobs que esperavam ESTE
    /// codigo (`transcriber_missing` ou `consent_missing`), e so eles — liberar
    /// um job de analise porque o transcritor voltou faria o laco girar
    /// gravando a mesma falha a cada volta.
    pub fn release_waiting(&self, code: &str) -> Result<usize, CoreError> {
        let now = self.clock.now();
        let mut freed = 0;
        for mut job in self.repository.meeting_jobs()? {
            if job.last_error_code.as_deref() == Some(code) && job.configuration_ready(now) {
                self.repository.save_meeting_job(&job)?;
                freed += 1;
            }
        }
        Ok(freed)
    }

    /// Um job que nao consegue comecar — a reuniao esta num estado de onde o
    /// estagio nao parte — e cancelado, e nao deixado na fila.
    ///
    /// Sem isto, um "transcrever" pedido sobre uma reuniao que ja nao aceita
    /// transcricao ficaria `queued` para sempre, e o laco tentaria a cada volta.
    pub fn abandon_job(&self, id: &str, message: &str) -> Result<(), CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        if let Some(mut job) = self.repository.meeting_job(meeting_id)? {
            job.cancel(self.clock.now());
            job.last_error_code = Some("not_startable".into());
            job.last_error_message = Some(message.trim().to_owned());
            self.repository.save_meeting_job(&job)?;
        }
        Ok(())
    }

    /// "Processar automaticamente" desligado: o job fica esperando a pessoa.
    pub fn hold_for_manual_start(&self, id: &str) -> Result<(), CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        if let Some(mut job) = self.repository.meeting_job(meeting_id)? {
            if job.status == crate::JobStatus::Queued {
                job.status = crate::JobStatus::NeedsAttention;
                job.last_error_code = Some("manual_start".into());
                job.last_error_message =
                    Some("A gravação está salva. O processamento espera você pedir.".into());
                job.updated_at = self.clock.now();
                self.repository.save_meeting_job(&job)?;
            }
        }
        Ok(())
    }

    /// A abertura: jobs orfaos voltam para a fila, reunioes presas no meio de um
    /// estagio voltam ao repouso, e reunioes paradas da V1 entram no pipeline.
    ///
    /// E o §9.3 do `MEETING-AGENT.md`, finalmente escrito.
    pub fn recover_pipeline_on_open(&self, analysis_allowed: bool) -> Result<usize, CoreError> {
        let now = self.clock.now();
        let mut touched = 0usize;
        let jobs = self.repository.meeting_jobs()?;

        for mut job in jobs.iter().cloned() {
            if job.status == crate::JobStatus::Running {
                job.recover_orphan(now);
                self.repository.save_meeting_job(&job)?;
                touched += 1;
            }
        }

        for meeting in self.repository.meetings(true)? {
            let id = meeting.id.to_string();
            match meeting.status {
                crate::MeetingStatus::Transcribing | crate::MeetingStatus::Analyzing => {
                    let requeued =
                        crate::apply_meeting(&meeting, crate::MeetingTransition::Requeue, now)?;
                    self.repository.save_meeting(&requeued)?;
                    if !jobs.iter().any(|job| job.meeting_id == meeting.id) {
                        let stage = if meeting.status == crate::MeetingStatus::Transcribing {
                            crate::JobStage::Transcription
                        } else {
                            crate::JobStage::Analysis
                        };
                        self.queue(&id, stage)?;
                    }
                    touched += 1;
                }
                // A V1 deixava reunioes paradas esperando um clique. As recentes
                // entram no pipeline; as antigas ficam como estao — acordar um
                // mes de reunioes de uma vez mandaria tudo ao Hermes sem ninguem
                // ter pedido naquele dia.
                crate::MeetingStatus::Recorded | crate::MeetingStatus::Transcribed
                    if now - meeting.started_at <= time::Duration::days(LEGACY_REQUEUE_DAYS)
                        && !jobs.iter().any(|job| job.meeting_id == meeting.id) =>
                {
                    if meeting.status == crate::MeetingStatus::Recorded {
                        if meeting.audio_deleted_at.is_none() {
                            self.queue(&id, crate::JobStage::Transcription)?;
                            touched += 1;
                        }
                    } else if analysis_allowed {
                        self.queue(&id, crate::JobStage::Analysis)?;
                        touched += 1;
                    }
                }
                _ => {}
            }
        }
        Ok(touched)
    }

    /// Reunioes interrompidas com audio entram no pipeline sem pedir decisao.
    pub fn auto_process_recovered(&self) -> Result<Vec<crate::Meeting>, CoreError> {
        let mut processed = Vec::new();
        for meeting in self.repository.meetings(true)? {
            if meeting.status != crate::MeetingStatus::Interrupted || meeting.duration_ms == 0 {
                continue;
            }
            let id = meeting.id.to_string();
            let mut recovered = meeting.clone();
            recovered
                .stop_reason
                .get_or_insert(crate::StopReason::CrashRecovery);
            let recovered = self.repository.save_meeting(&recovered)?;
            if self.repository.meeting_job(meeting.id)?.is_none() {
                self.queue(&id, crate::JobStage::Transcription)?;
            }
            processed.push(recovered);
        }
        Ok(processed)
    }

    // ------------------------------------------------------------------------
    // Lixeira
    // ------------------------------------------------------------------------

    /// Manda para a lixeira. Cancela o que estava na fila.
    ///
    /// **Gravando nao vai.** Quem chama precisa parar antes — a tela pergunta
    /// "Encerrar e apagar".
    pub fn trash(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        let meeting = self.meeting(id)?;
        if meeting.status.is_capturing() {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidTransition,
                "Esta reuniao ainda esta sendo gravada. Encerre antes de apagar.",
                false,
            ));
        }
        if let Some(mut job) = self.repository.meeting_job(meeting.id)? {
            if job.status.is_open() {
                job.cancel(self.clock.now());
                self.repository.save_meeting_job(&job)?;
            }
        }
        self.repository
            .set_meeting_trashed(meeting.id, Some(self.clock.now()))
    }

    /// Tira da lixeira. O pipeline que foi cancelado volta, se ainda havia o
    /// que fazer.
    pub fn restore(&self, id: &str) -> Result<crate::Meeting, CoreError> {
        let meeting_id = crate::MeetingId::parse(id)?;
        let restored = self.repository.set_meeting_trashed(meeting_id, None)?;
        if let Some(mut job) = self.repository.meeting_job(meeting_id)? {
            if job.status == crate::JobStatus::Cancelled {
                job.manual_retry(self.clock.now());
                self.repository.save_meeting_job(&job)?;
            }
        }
        Ok(restored)
    }

    pub fn trashed(&self) -> Result<Vec<crate::Meeting>, CoreError> {
        self.repository.trashed_meetings()
    }

    /// As que passaram dos 30 dias na lixeira.
    pub fn expired_trash(&self) -> Result<Vec<crate::Meeting>, CoreError> {
        let now = self.clock.now();
        Ok(self
            .repository
            .trashed_meetings()?
            .into_iter()
            .filter(|meeting| crate::meeting_pipeline::trash_expired(meeting, now))
            .collect())
    }

    // ------------------------------------------------------------------------
    // Corte
    // ------------------------------------------------------------------------

    /// Ajusta a faixa que conta. Reprocessa so o que precisa.
    pub fn set_trim(
        &self,
        id: &str,
        start_ms: i64,
        end_ms: i64,
        origin: crate::TrimOrigin,
    ) -> Result<TrimOutcome, CoreError> {
        let mut meeting = self.meeting(id)?;
        if meeting.status.is_capturing() {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidTransition,
                "Encerre a gravacao antes de ajustar o inicio e o fim.",
                false,
            ));
        }
        if matches!(
            meeting.status,
            crate::MeetingStatus::Transcribing | crate::MeetingStatus::Analyzing
        ) {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidTransition,
                "A reuniao esta sendo processada. Ajuste o corte quando ela terminar.",
                true,
            ));
        }
        let duration = meeting.duration_ms.max(0);
        let start = start_ms.clamp(0, duration);
        let end = end_ms.clamp(0, duration);
        if end <= start {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidInput,
                "O fim precisa vir depois do inicio.",
                false,
            ));
        }

        let (old_start, old_end) = meeting.effective_range();
        let full = start == 0 && end == duration;
        meeting.trim_start_ms = (start > 0).then_some(start);
        meeting.trim_end_ms = (end < duration).then_some(end);
        meeting.trim_origin = (!full).then_some(origin);
        meeting.updated_at = self.clock.now();

        let has_transcript = matches!(
            meeting.status,
            crate::MeetingStatus::Transcribed
                | crate::MeetingStatus::Ready
                | crate::MeetingStatus::Failed(crate::FailedStage::Analysis)
        );
        let shrinks = start >= old_start && end <= old_end;

        if !has_transcript {
            let meeting = self.repository.save_meeting(&meeting)?;
            return Ok(TrimOutcome {
                meeting,
                requeued: None,
                removed_segments: 0,
            });
        }

        if shrinks {
            let meeting = self.repository.save_meeting(&meeting)?;
            let removed = self
                .repository
                .remove_segments_outside(meeting.id, start, end)?;
            let requeued = if removed > 0 && self.repository.analysis(meeting.id)?.is_some() {
                self.queue(id, crate::JobStage::Analysis)?;
                Some(crate::JobStage::Analysis)
            } else {
                None
            };
            return Ok(TrimOutcome {
                meeting: self.meeting(id)?,
                requeued,
                removed_segments: removed,
            });
        }

        if meeting.audio_deleted_at.is_some() {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidTransition,
                "O audio desta reuniao ja foi apagado, entao a faixa nao pode crescer.",
                false,
            ));
        }
        let meeting = crate::apply_meeting(
            &meeting,
            crate::MeetingTransition::Reprocess,
            self.clock.now(),
        )?;
        let meeting = self.repository.save_meeting(&meeting)?;
        self.queue(id, crate::JobStage::Transcription)?;
        Ok(TrimOutcome {
            meeting,
            requeued: Some(crate::JobStage::Transcription),
            removed_segments: 0,
        })
    }

    /// Volta a faixa inteira.
    pub fn clear_trim(&self, id: &str) -> Result<TrimOutcome, CoreError> {
        let meeting = self.meeting(id)?;
        self.set_trim(id, 0, meeting.duration_ms, crate::TrimOrigin::Manual)
    }

    /// A sugestao de corte feita pela transcricao, quando o Guardian nao sabia.
    pub fn transcript_trim_suggestion(&self, id: &str) -> Result<Option<i64>, CoreError> {
        let meeting = self.meeting(id)?;
        if meeting.trim_end_ms.is_some() {
            return Ok(None);
        }
        if let Some(end) = meeting
            .suggested_end_ms
            .filter(|end| meeting.duration_ms - end >= crate::meeting_guardian::TRIM_MIN_EXCESS_MS)
        {
            return Ok(Some(end));
        }
        let segments = self.transcript(id)?;
        Ok(crate::meeting_guardian::trim_suggestion_from_segments(
            segments.iter().map(|segment| segment.end_ms),
            meeting.duration_ms,
        ))
    }

    // ------------------------------------------------------------------------
    // Momentos e metrica
    // ------------------------------------------------------------------------

    pub fn add_bookmark(&self, id: &str, at_ms: i64) -> Result<crate::MeetingBookmark, CoreError> {
        let bookmark = crate::MeetingBookmark {
            id: crate::BookmarkId::new(),
            meeting_id: crate::MeetingId::parse(id)?,
            at_ms: at_ms.max(0),
            note: String::new(),
            created_at: self.clock.now(),
        };
        self.repository.add_meeting_bookmark(&bookmark)?;
        Ok(bookmark)
    }

    pub fn bookmarks(&self, id: &str) -> Result<Vec<crate::MeetingBookmark>, CoreError> {
        self.repository
            .meeting_bookmarks(crate::MeetingId::parse(id)?)
    }

    pub fn delete_bookmark(&self, bookmark_id: &str) -> Result<(), CoreError> {
        self.repository
            .delete_meeting_bookmark(crate::BookmarkId::parse(bookmark_id)?)
    }

    pub fn record_guardian_event(
        &self,
        id: &str,
        kind: &str,
        confidence: Option<f32>,
        trigger: &str,
        excess_ms: Option<i64>,
    ) -> Result<(), CoreError> {
        self.repository
            .record_guardian_event(&crate::GuardianEventRecord {
                meeting_id: crate::MeetingId::parse(id)?,
                at: self.clock.now(),
                kind: kind.to_owned(),
                confidence,
                trigger: trigger.to_owned(),
                excess_ms,
            })
    }

    pub fn guardian_counts(&self) -> Result<Vec<(String, i64)>, CoreError> {
        self.repository.guardian_event_counts()
    }

    // ------------------------------------------------------------------------
    // Itens
    // ------------------------------------------------------------------------

    /// Le os marcadores das notas e grava como itens escritos.
    pub fn apply_written_insights(&self, id: &str) -> Result<usize, CoreError> {
        let meeting = self.meeting(id)?;
        let insights = crate::meeting_text::written_insights(meeting.id, &meeting.notes);
        self.repository
            .replace_written_insights(meeting.id, insights)
    }

    /// Interpreta os prazos dos itens, na referencia do inicio da reuniao.
    ///
    /// `offset` e o fuso de quem gravou: o dominio nao conhece fuso, e "amanha"
    /// depende dele.
    pub fn resolve_dues(&self, id: &str, offset: time::UtcOffset) -> Result<usize, CoreError> {
        let meeting = self.meeting(id)?;
        let reference = meeting.started_at.to_offset(offset);
        let mut resolved = 0;
        for insight in self.repository.insights(meeting.id)? {
            if insight.due_at.is_some() {
                continue;
            }
            let Some(expression) = insight.due_hint.as_deref() else {
                continue;
            };
            if let Some(due) = crate::meeting_dates::resolve_due(expression, reference) {
                self.repository.set_insight_due(
                    insight.id,
                    Some(due.resolved_at),
                    Some(due.confidence),
                )?;
                resolved += 1;
            }
        }
        Ok(resolved)
    }

    /// Um item criado pela pessoa a partir de um trecho da transcricao.
    pub fn add_manual_insight(
        &self,
        id: &str,
        segment_id: &str,
        kind: crate::InsightKind,
        text: Option<&str>,
    ) -> Result<crate::MeetingInsight, CoreError> {
        let meeting = self.meeting(id)?;
        let segment_id = crate::SegmentId::parse(segment_id)?;
        let segment = self
            .transcript(id)?
            .into_iter()
            .find(|segment| segment.id == segment_id)
            .ok_or_else(|| {
                CoreError::new(crate::ErrorCode::NotFound, "Trecho nao encontrado.", false)
            })?;
        let text = text
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                segment
                    .text_normalized
                    .clone()
                    .unwrap_or_else(|| segment.text.clone())
            });
        self.repository.add_manual_insight(crate::MeetingInsight {
            id: crate::InsightId::new(),
            meeting_id: meeting.id,
            kind,
            seq: 0,
            text,
            owner: None,
            due_hint: crate::meeting_dates::find_due_expression(&segment.text),
            confidence: crate::Confidence::High,
            status: crate::InsightStatus::Proposed,
            created_task_id: None,
            created_reminder_id: None,
            evidence: vec![crate::MeetingEvidence {
                segment_id,
                seq: 0,
                char_start: None,
                char_end: None,
            }],
            origin: crate::InsightOrigin::Manual,
            due_at: None,
            due_confidence: None,
        })
    }

    /// A revisao em lote: cria as Tasks marcadas numa transacao so.
    ///
    /// Tres coisas que o item sozinho da V1 nao fazia:
    ///
    /// - **prazo nativo** — `Task.due_at` recebe o que a tela confirmou;
    /// - **espera** — compromisso de outra pessoa nasce aguardando ela, e entra
    ///   no Waiting For que o piloto ja cobra;
    /// - **sem duplicata** — Task ativa com o mesmo titulo no mesmo Project,
    ///   criada nos ultimos 14 dias, recebe o vinculo em vez de uma gemea.
    pub fn accept_batch(
        &self,
        accepts: Vec<crate::AcceptInsight>,
    ) -> Result<Vec<crate::AcceptedInsight>, CoreError> {
        if accepts.is_empty() {
            return Ok(Vec::new());
        }
        let now = self.clock.now();
        let recent = self
            .repository
            .recent_open_tasks(now - time::Duration::days(DEDUPE_TASK_DAYS))?;

        let mut items = Vec::with_capacity(accepts.len());
        let mut keys_in_batch: Vec<(String, Option<crate::ProjectId>)> = Vec::new();
        for accept in accepts {
            let meeting_id = self.repository.insights_meeting(accept.insight_id)?;
            let meeting = self.repository.meeting(meeting_id)?;
            let insight = self
                .repository
                .insights(meeting_id)?
                .into_iter()
                .find(|insight| insight.id == accept.insight_id)
                .ok_or_else(|| {
                    CoreError::new(
                        crate::ErrorCode::NotFound,
                        "Item de reuniao nao encontrado.",
                        false,
                    )
                })?;

            let key = crate::meeting_text::normalized_key(&accept.title);
            if keys_in_batch
                .iter()
                .any(|(other, project)| *other == key && *project == accept.project_id)
            {
                return Err(CoreError::new(
                    crate::ErrorCode::InvalidInput,
                    "Dois itens do lote viram a mesma Task. Desmarque um deles.",
                    false,
                ));
            }
            keys_in_batch.push((key.clone(), accept.project_id));

            let link_existing = recent
                .iter()
                .find(|(_, title, project)| {
                    *project == accept.project_id
                        && crate::meeting_text::normalized_key(title) == key
                })
                .map(|(task, _, _)| *task);

            let description = if accept.description.trim().is_empty() {
                format!("Da reuniao \"{}\"", meeting.title)
            } else {
                accept.description.clone()
            };
            let task = crate::NewTask::create(&accept.title, &description, accept.project_id)?
                .with_due_at(accept.due_at);
            let reminder = match accept.remind_at {
                Some(instant) if link_existing.is_none() => Some(
                    crate::NewReminder::at(
                        &accept.title,
                        &format!("Da reuniao \"{}\"", meeting.title),
                        instant,
                        self.clock.as_ref(),
                    )?
                    .with_target(crate::ReminderTarget::Task(task.id)),
                ),
                _ => None,
            };
            let waiting_for = insight.kind.is_external().then(|| {
                insight
                    .owner
                    .clone()
                    .filter(|owner| !owner.trim().is_empty())
                    .unwrap_or_else(|| "outra pessoa".to_owned())
            });
            items.push(crate::BatchAcceptItem {
                accept,
                task,
                reminder,
                waiting_for,
                link_existing,
            });
        }
        self.repository.accept_insights_batch(items)
    }

    /// A projecao V2 da analise.
    pub fn analysis_v2(&self, id: &str) -> Result<crate::MeetingAnalysisV2, CoreError> {
        let summary = self
            .analysis(id)?
            .map(|analysis| analysis.summary)
            .unwrap_or_default();
        Ok(crate::MeetingAnalysisV2::project(
            &summary,
            &self.insights(id)?,
        ))
    }

    // ------------------------------------------------------------------------
    // Lista
    // ------------------------------------------------------------------------

    /// A lista com fase, progresso e contagens.
    pub fn overview(&self, include_archived: bool) -> Result<Vec<MeetingOverview>, CoreError> {
        let jobs = self.repository.meeting_jobs()?;
        let mut rows = Vec::new();
        for meeting in self.repository.meetings(include_archived)? {
            rows.push(self.overview_of(meeting, &jobs)?);
        }
        Ok(rows)
    }

    pub fn overview_one(&self, id: &str) -> Result<MeetingOverview, CoreError> {
        let meeting = self.meeting(id)?;
        let jobs: Vec<crate::MeetingJob> = self
            .repository
            .meeting_job(meeting.id)?
            .into_iter()
            .collect();
        self.overview_of(meeting, &jobs)
    }

    fn overview_of(
        &self,
        meeting: crate::Meeting,
        jobs: &[crate::MeetingJob],
    ) -> Result<MeetingOverview, CoreError> {
        let job = jobs
            .iter()
            .find(|job| job.meeting_id == meeting.id)
            .cloned();
        let insights = self.repository.insights(meeting.id)?;
        let live = |kind: &[crate::InsightKind]| {
            insights
                .iter()
                .filter(|i| kind.contains(&i.kind) && i.status != crate::InsightStatus::Dismissed)
                .count()
        };
        let pending_actions = insights
            .iter()
            .filter(|i| i.status == crate::InsightStatus::Proposed && i.kind.is_actionable())
            .count();
        let tasks_created = insights
            .iter()
            .filter(|i| i.created_task_id.is_some())
            .count();
        let phase = crate::meeting_phase(&meeting, job.as_ref());
        let progress = crate::pipeline_progress(&meeting, job.as_ref());
        let attention = match phase {
            crate::MeetingPhase::NeedsAttention => job
                .as_ref()
                .and_then(|job| job.last_error_message.clone())
                .or_else(|| meeting.failure.as_ref().map(|f| f.message.clone()))
                .unwrap_or_else(|| "Esta reunião precisa de você.".to_owned()),
            crate::MeetingPhase::FailedRecoverable => {
                "A transcrição não deu certo. O áudio está seguro.".to_owned()
            }
            _ => String::new(),
        };
        Ok(MeetingOverview {
            decisions: live(&[crate::InsightKind::Decision]),
            waiting: live(&[
                crate::InsightKind::OtherAction,
                crate::InsightKind::Commitment,
            ]),
            questions: live(&[crate::InsightKind::OpenQuestion]),
            meeting,
            phase,
            progress,
            job,
            pending_actions,
            tasks_created,
            attention,
        })
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
#[derive(Clone)]
pub struct DailyService {
    repository: Arc<dyn crate::DailyRepository>,
    clock: Arc<dyn crate::Clock>,
}

impl DailyService {
    pub fn new(repository: Arc<dyn crate::DailyRepository>, clock: Arc<dyn crate::Clock>) -> Self {
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

    pub fn reopen(&self, session: crate::DailySessionId) -> Result<crate::DailySession, CoreError> {
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

/// O M/Academic.
///
/// Ele busca e delega: a composicao do painel e de hoje vive em
/// `mos_core::academic`, pura e testada, e este servico so junta as leituras.
/// Mesma divisao do `DailyService` com `compose_context`.
pub struct AcademicService {
    repository: Arc<dyn crate::AcademicRepository>,
}

impl AcademicService {
    pub fn new(repository: Arc<dyn crate::AcademicRepository>) -> Self {
        Self { repository }
    }

    // --- Semestre

    pub fn semesters(&self, include_archived: bool) -> Result<Vec<crate::Semester>, CoreError> {
        self.repository.semesters(include_archived)
    }

    pub fn create_semester(
        &self,
        name: &str,
        institution: &str,
        starts_on: &str,
        ends_on: &str,
    ) -> Result<crate::Semester, CoreError> {
        self.repository.create_semester(crate::NewSemester::create(
            name,
            institution,
            starts_on,
            ends_on,
        )?)
    }

    pub fn update_semester(
        &self,
        id: &str,
        name: &str,
        institution: &str,
        starts_on: &str,
        ends_on: &str,
    ) -> Result<crate::Semester, CoreError> {
        // Valida pelo mesmo caminho da criacao: um nome vazio ou um intervalo
        // invertido tem de morrer aqui nos dois casos, e nao so num deles.
        let validado = crate::NewSemester::create(name, institution, starts_on, ends_on)?;
        self.repository.update_semester(
            crate::SemesterId::parse(id)?,
            &validado.name,
            &validado.institution,
            &validado.starts_on,
            &validado.ends_on,
        )
    }

    pub fn set_semester_archived(
        &self,
        id: &str,
        archived: bool,
    ) -> Result<crate::Semester, CoreError> {
        self.repository.set_semester_lifecycle(
            crate::SemesterId::parse(id)?,
            if archived {
                crate::LifecycleState::Archived
            } else {
                crate::LifecycleState::Active
            },
        )
    }

    // --- Disciplina

    pub fn subjects(&self, include_archived: bool) -> Result<Vec<crate::Subject>, CoreError> {
        self.repository.subjects(include_archived)
    }

    pub fn create_subject(
        &self,
        semester_id: &str,
        name: &str,
        code: &str,
        teacher: &str,
        accent: &str,
        notes: &str,
    ) -> Result<crate::Subject, CoreError> {
        self.repository.create_subject(crate::NewSubject::create(
            crate::SemesterId::parse(semester_id)?,
            name,
            code,
            teacher,
            accent,
            notes,
        )?)
    }

    pub fn update_subject(
        &self,
        id: &str,
        name: &str,
        code: &str,
        teacher: &str,
        accent: &str,
        notes: &str,
    ) -> Result<crate::Subject, CoreError> {
        let accent = crate::validate_accent(accent)?;
        if name.trim().is_empty() {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidInput,
                "O nome da disciplina nao pode estar vazio.",
                false,
            ));
        }
        self.repository.update_subject(
            crate::SubjectId::parse(id)?,
            name.trim(),
            code.trim(),
            teacher.trim(),
            &accent,
            notes.trim(),
        )
    }

    pub fn set_subject_archived(
        &self,
        id: &str,
        archived: bool,
    ) -> Result<crate::Subject, CoreError> {
        self.repository.set_subject_lifecycle(
            crate::SubjectId::parse(id)?,
            if archived {
                crate::LifecycleState::Archived
            } else {
                crate::LifecycleState::Active
            },
        )
    }

    // --- Atividade

    pub fn assignments(&self, include_archived: bool) -> Result<Vec<crate::Assignment>, CoreError> {
        self.repository.assignments(include_archived)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_assignment(
        &self,
        subject_id: &str,
        title: &str,
        description: &str,
        due_at: Option<&str>,
        priority: &str,
        weight: f64,
        score: Option<f64>,
        max_score: Option<f64>,
    ) -> Result<crate::Assignment, CoreError> {
        self.repository
            .create_assignment(crate::NewAssignment::create(
                crate::SubjectId::parse(subject_id)?,
                title,
                description,
                due_at.map(crate::parse_moment).transpose()?,
                crate::Priority::parse(priority)?,
                weight,
                score,
                max_score,
            )?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_assignment(
        &self,
        id: &str,
        title: &str,
        description: &str,
        due_at: Option<&str>,
        priority: &str,
        weight: f64,
        score: Option<f64>,
        max_score: Option<f64>,
        status: &str,
    ) -> Result<crate::Assignment, CoreError> {
        if title.trim().is_empty() {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidInput,
                "O titulo da atividade nao pode estar vazio.",
                false,
            ));
        }
        self.repository.update_assignment(crate::UpdateAssignment {
            id: crate::AssignmentId::parse(id)?,
            title: title.trim().to_owned(),
            description: description.trim().to_owned(),
            due_at: due_at.map(crate::parse_moment).transpose()?,
            priority: crate::Priority::parse(priority)?,
            weight,
            score,
            max_score,
            status: crate::AssignmentStatus::parse(status)?,
        })
    }

    pub fn set_assignment_status(
        &self,
        id: &str,
        status: &str,
    ) -> Result<crate::Assignment, CoreError> {
        self.repository.set_assignment_status(
            crate::AssignmentId::parse(id)?,
            crate::AssignmentStatus::parse(status)?,
        )
    }

    pub fn set_assignment_archived(
        &self,
        id: &str,
        archived: bool,
    ) -> Result<crate::Assignment, CoreError> {
        self.repository.set_assignment_lifecycle(
            crate::AssignmentId::parse(id)?,
            if archived {
                crate::LifecycleState::Archived
            } else {
                crate::LifecycleState::Active
            },
        )
    }

    pub fn create_task_for_assignment(&self, id: &str) -> Result<crate::Task, CoreError> {
        self.repository
            .create_task_for_assignment(crate::AssignmentId::parse(id)?)
    }

    pub fn unlink_assignment_task(&self, id: &str) -> Result<crate::Assignment, CoreError> {
        self.repository
            .unlink_assignment_task(crate::AssignmentId::parse(id)?)
    }

    // --- Avaliacao

    pub fn exams(&self, include_archived: bool) -> Result<Vec<crate::Exam>, CoreError> {
        self.repository.exams(include_archived)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_exam(
        &self,
        subject_id: &str,
        name: &str,
        at: &str,
        location: &str,
        topics: &str,
        weight: f64,
        score: Option<f64>,
        max_score: Option<f64>,
    ) -> Result<crate::Exam, CoreError> {
        self.repository.create_exam(crate::NewExam::create(
            crate::SubjectId::parse(subject_id)?,
            name,
            crate::parse_moment(at)?,
            location,
            topics,
            weight,
            score,
            max_score,
        )?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_exam(
        &self,
        id: &str,
        name: &str,
        at: &str,
        location: &str,
        topics: &str,
        weight: f64,
        score: Option<f64>,
        max_score: Option<f64>,
        status: &str,
    ) -> Result<crate::Exam, CoreError> {
        if name.trim().is_empty() {
            return Err(CoreError::new(
                crate::ErrorCode::InvalidInput,
                "O nome da avaliacao nao pode estar vazio.",
                false,
            ));
        }
        self.repository.update_exam(crate::UpdateExam {
            id: crate::ExamId::parse(id)?,
            name: name.trim().to_owned(),
            at: crate::parse_moment(at)?,
            location: location.trim().to_owned(),
            topics: topics.trim().to_owned(),
            weight,
            score,
            max_score,
            status: crate::ExamStatus::parse(status)?,
        })
    }

    pub fn set_exam_archived(&self, id: &str, archived: bool) -> Result<crate::Exam, CoreError> {
        self.repository.set_exam_lifecycle(
            crate::ExamId::parse(id)?,
            if archived {
                crate::LifecycleState::Archived
            } else {
                crate::LifecycleState::Active
            },
        )
    }

    // --- Materiais

    pub fn subject_resources(&self, id: &str) -> Result<Vec<crate::Resource>, CoreError> {
        self.repository
            .subject_resources(crate::SubjectId::parse(id)?)
    }

    pub fn link_material(
        &self,
        subject_id: &str,
        resource_id: &str,
        linked: bool,
    ) -> Result<(), CoreError> {
        self.repository.link_material(
            crate::SubjectId::parse(subject_id)?,
            crate::ResourceId::parse(resource_id)?,
            linked,
        )
    }

    // --- Estudo

    pub fn study_sessions(&self, limit: usize) -> Result<Vec<crate::StudySession>, CoreError> {
        self.repository.study_sessions(limit)
    }

    pub fn start_study(
        &self,
        subject_id: &str,
        topic: &str,
    ) -> Result<crate::StudySession, CoreError> {
        self.repository
            .start_study(crate::SubjectId::parse(subject_id)?, topic)
    }

    pub fn finish_study(
        &self,
        id: &str,
        seconds: i64,
        notes: &str,
    ) -> Result<crate::StudySession, CoreError> {
        self.repository
            .finish_study(crate::StudySessionId::parse(id)?, seconds, notes)
    }

    pub fn discard_study(&self, id: &str) -> Result<(), CoreError> {
        self.repository
            .discard_study(crate::StudySessionId::parse(id)?)
    }

    // --- O painel

    /// Quantas sessoes de estudo o painel le.
    ///
    /// Trezentas sao ~dez por semana num semestre inteiro. Alem disso a sessao
    /// deixou de contar para "esta semana" e para "hoje", que e tudo o que o
    /// painel pergunta — carregar o historico completo a cada abertura seria o
    /// N+1 que o §32 do pedido proibe, so que em forma de tabela crescendo.
    const SESSOES_DO_PAINEL: usize = 300;

    /// O painel inteiro, ja composto.
    ///
    /// `now_local` vem da tela: o dia civil e do fuso de quem olha, e decidi-lo
    /// aqui em UTC jogaria toda madrugada para o dia seguinte.
    pub fn dashboard(
        &self,
        now_local: time::OffsetDateTime,
    ) -> Result<crate::AcademicDashboard, CoreError> {
        let semesters = self.repository.semesters(false)?;
        let subjects = self.repository.subjects(false)?;
        let assignments = self.repository.assignments(false)?;
        let exams = self.repository.exams(false)?;
        let sessions = self.repository.study_sessions(Self::SESSOES_DO_PAINEL)?;

        // A contagem de materiais vem AGREGADA do banco, e nao numa consulta por
        // disciplina: dez materias fariam dez consultas a cada refresh da Home.
        let contagens: std::collections::HashMap<_, _> =
            self.repository.material_counts()?.into_iter().collect();
        let materials = |id: crate::SubjectId| contagens.get(&id).copied().unwrap_or(0);

        Ok(crate::compose_dashboard(crate::DashboardInput {
            now_local,
            semesters: &semesters,
            subjects: &subjects,
            assignments: &assignments,
            exams: &exams,
            sessions: &sessions,
            materials: &materials,
        }))
    }

    /// Os compromissos academicos numa janela, para o Calendario.
    ///
    /// Nao passa pelo painel: ver `compose_compromissos`.
    pub fn compromissos_entre(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
        now_local: time::OffsetDateTime,
    ) -> Result<Vec<crate::Compromisso>, CoreError> {
        Ok(crate::compose_compromissos(
            &self.repository.subjects(false)?,
            &self.repository.assignments(false)?,
            &self.repository.exams(false)?,
            since,
            until,
            now_local,
        ))
    }

    /// O recorte de hoje, para o Start My Day e o End My Day.
    pub fn today(
        &self,
        now_local: time::OffsetDateTime,
    ) -> Result<crate::AcademicToday, CoreError> {
        let painel = self.dashboard(now_local)?;
        Ok(crate::compose_today(&painel, now_local))
    }
}

/// Os servicos que a camada de acao do agente usa.
///
/// # Por que este tipo existe
///
/// O executor do Hermes vivia dentro do shell do desktop e pegava os servicos
/// de `app.state::<AppState>()` — um tipo do Tauri. Isso amarrava 2.388 linhas
/// de logica de dominio a uma janela, e a superficie de bolso nao tinha como
/// reaproveita-las sem arrastar o Tauri junto.
///
/// Nada aqui e novo: sao os mesmos oito servicos que a interface ja usa. O que
/// muda e que agora eles cabem num parametro, e quem executa uma acao do agente
/// nao precisa saber se existe uma janela do outro lado.
///
/// **A invariante que isto protege continua sendo a de sempre:** a acao do
/// agente passa pelos MESMOS servicos que a acao do usuario. Nunca SQL proprio,
/// nunca um atalho (ADR-024, ADR-032).
#[derive(Clone)]
pub struct Servicos {
    pub captures: CaptureService,
    pub work: WorkService,
    pub memory: MemoryService,
    pub conversations: ConversationService,
    pub tracking: TrackingService,
    pub meetings: MeetingService,
    pub attention: AttentionService,
    pub daily: DailyService,
}
