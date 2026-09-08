//! Persistencia do Attention System.
//!
//! O banco e a fonte de verdade do agendador. Se o agendador morrer, os
//! Reminders continuam aqui e a reconciliacao da abertura os encontra — e e
//! isso que sustenta a promessa de que nenhum lembrete se perde em silencio.
//!
//! O `trigger` vai como JSON com discriminante. So `at` existe no P0, e guardar
//! JSON evita uma coluna nova por braco futuro. O preco e que o banco nao
//! valida a forma interna do trigger; quem valida e o dominio ao ler, e um
//! `kind` desconhecido vira erro de integridade em vez de comportamento
//! silencioso.

use mos_core::{
    AttentionRepository, Channel, ContentPrivacy, CoreError, DeliveryPolicy, ErrorCode,
    LifecycleState, NewNotification, NewReminder, Notification, NotificationId, NotificationStatus,
    Priority, Reminder, ReminderId, ReminderSource, ReminderStatus, ReminderTarget, Trigger,
    VisualLevel,
};
use rusqlite::{params, Row};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time},
    SqliteStorage,
};

const REMINDER_COLUMNS: &str = "id, title, body, target_type, target_id, trigger, priority, \
     status, source, snooze_allowed, privacy, next_due_at, snooze_count, delivered_count, \
     created_at, updated_at, completed_at, lifecycle_state, kind, waiting_for, persistent, \
     escalation_step, last_triggered_at, retry_at, recurrence";

const TRIGGER_COLUMNS: &str = "id, reminder_id, scheduled_at, kind, lead_minutes, status, \
     fired_at, lifecycle_state, created_at, updated_at";

const EVENT_COLUMNS: &str = "id, reminder_id, kind, at, detail";

const NOTIFICATION_COLUMNS: &str = "id, reminder_id, channel, dedupe_key, status, level, \
     created_at, delivered_at, resolved_at, failure";

fn read_reminder(row: &Row<'_>) -> rusqlite::Result<Result<Reminder, CoreError>> {
    let id: String = row.get(0)?;
    let target_type: Option<String> = row.get(3)?;
    let target_id: Option<String> = row.get(4)?;
    let trigger: String = row.get(5)?;
    let priority: String = row.get(6)?;
    let status: String = row.get(7)?;
    let source: String = row.get(8)?;
    let privacy: String = row.get(10)?;
    let next_due_at: Option<String> = row.get(11)?;
    let created_at: String = row.get(14)?;
    let updated_at: String = row.get(15)?;
    let completed_at: Option<String> = row.get(16)?;
    let lifecycle: String = row.get(17)?;
    let kind: String = row.get(18)?;
    let waiting_for: String = row.get(19)?;
    let persistent: i64 = row.get(20)?;
    let escalation_step: i64 = row.get(21)?;
    let last_triggered_at: Option<String> = row.get(22)?;
    let retry_at: Option<String> = row.get(23)?;
    let recurrence: Option<String> = row.get(24)?;

    let title: String = row.get(1)?;
    let body: String = row.get(2)?;
    let snooze_allowed: i64 = row.get(9)?;
    let snooze_count: i64 = row.get(12)?;
    let delivered_count: i64 = row.get(13)?;

    Ok((|| {
        let target = match (target_type.as_deref(), target_id.as_deref()) {
            (Some(kind), Some(value)) => Some(ReminderTarget::from_columns(kind, value)?),
            _ => None,
        };

        let trigger: Trigger = serde_json::from_str(&trigger).map_err(|_| {
            CoreError::new(
                ErrorCode::DataIntegrity,
                "Trigger de Reminder ilegivel.",
                false,
            )
        })?;

        Ok(Reminder {
            id: ReminderId::parse(&id)?,
            title,
            body,
            target,
            trigger,
            priority: Priority::parse(&priority)?,
            status: ReminderStatus::parse(&status)?,
            policy: DeliveryPolicy {
                snooze_allowed: snooze_allowed != 0,
                privacy: ContentPrivacy::parse(&privacy)?,
            },
            source: ReminderSource::parse(&source)?,
            next_due_at: next_due_at.as_deref().map(parse_time).transpose()?,
            snooze_count: snooze_count as u32,
            delivered_count: delivered_count as u32,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
            completed_at: completed_at.as_deref().map(parse_time).transpose()?,
            lifecycle_state: LifecycleState::parse(&lifecycle)?,
            kind: mos_core::ReminderKind::parse(&kind)?,
            waiting_for,
            persistent: persistent != 0,
            escalation_step: escalation_step.max(0) as u32,
            last_triggered_at: last_triggered_at.as_deref().map(parse_time).transpose()?,
            retry_at: retry_at.as_deref().map(parse_time).transpose()?,
            recurrence: decode_recurrence(recurrence.as_deref())?,
        })
    })())
}

/// A regra de repeticao, do JSON para o dominio.
///
/// Regra ilegivel vira ERRO e nao `None`: um lembrete que perdesse a repeticao
/// em silencio e exatamente a falha que este sistema inteiro existe para nao
/// ter. Melhor a leitura falhar alto do que a serie parar de existir sem aviso.
fn decode_recurrence(raw: Option<&str>) -> Result<Option<mos_core::Recurrence>, CoreError> {
    let Some(raw) = raw.map(str::trim).filter(|texto| !texto.is_empty()) else {
        return Ok(None);
    };
    let regra: mos_core::Recurrence = serde_json::from_str(raw).map_err(|_| {
        CoreError::new(
            ErrorCode::DataIntegrity,
            "Regra de repeticao ilegivel.",
            false,
        )
    })?;
    regra.validate()?;
    Ok(Some(regra))
}

fn encode_recurrence(
    recurrence: Option<&mos_core::Recurrence>,
) -> Result<Option<String>, CoreError> {
    match recurrence {
        None => Ok(None),
        Some(regra) => {
            regra.validate()?;
            serde_json::to_string(regra).map(Some).map_err(|_| {
                CoreError::new(
                    ErrorCode::DataIntegrity,
                    "Nao consegui serializar a repeticao.",
                    false,
                )
            })
        }
    }
}

fn read_trigger(row: &Row<'_>) -> rusqlite::Result<Result<mos_core::ReminderTrigger, CoreError>> {
    let id: String = row.get(0)?;
    let reminder_id: String = row.get(1)?;
    let scheduled_at: String = row.get(2)?;
    let kind: String = row.get(3)?;
    let lead_minutes: Option<i64> = row.get(4)?;
    let status: String = row.get(5)?;
    let fired_at: Option<String> = row.get(6)?;
    let lifecycle: String = row.get(7)?;
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;

    Ok((|| {
        Ok(mos_core::ReminderTrigger {
            id: mos_core::ReminderTriggerId::parse(&id)?,
            reminder_id: ReminderId::parse(&reminder_id)?,
            scheduled_at: parse_time(&scheduled_at)?,
            kind: mos_core::StackTriggerKind::parse(&kind)?,
            lead_minutes: lead_minutes.and_then(|value| u32::try_from(value).ok()),
            status: mos_core::StackTriggerStatus::parse(&status)?,
            fired_at: fired_at.as_deref().map(parse_time).transpose()?,
            lifecycle_state: LifecycleState::parse(&lifecycle)?,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}

fn read_event(row: &Row<'_>) -> rusqlite::Result<Result<mos_core::ReminderEvent, CoreError>> {
    let id: String = row.get(0)?;
    let reminder_id: String = row.get(1)?;
    let kind: String = row.get(2)?;
    let at: String = row.get(3)?;
    let detail: Option<String> = row.get(4)?;

    Ok((|| {
        Ok(mos_core::ReminderEvent {
            id: mos_core::ReminderEventId::parse(&id)?,
            reminder_id: ReminderId::parse(&reminder_id)?,
            kind: mos_core::ReminderEventKind::parse(&kind)?,
            at: parse_time(&at)?,
            detail,
        })
    })())
}

fn read_notification(row: &Row<'_>) -> rusqlite::Result<Result<Notification, CoreError>> {
    let id: String = row.get(0)?;
    let reminder_id: String = row.get(1)?;
    let channel: String = row.get(2)?;
    let dedupe_key: String = row.get(3)?;
    let status: String = row.get(4)?;
    let level: String = row.get(5)?;
    let created_at: String = row.get(6)?;
    let delivered_at: Option<String> = row.get(7)?;
    let resolved_at: Option<String> = row.get(8)?;
    let failure: Option<String> = row.get(9)?;

    Ok((|| {
        Ok(Notification {
            id: NotificationId::parse(&id)?,
            reminder_id: ReminderId::parse(&reminder_id)?,
            channel: Channel::parse(&channel)?,
            dedupe_key,
            status: NotificationStatus::parse(&status)?,
            level: VisualLevel::parse(&level)?,
            created_at: parse_time(&created_at)?,
            delivered_at: delivered_at.as_deref().map(parse_time).transpose()?,
            resolved_at: resolved_at.as_deref().map(parse_time).transpose()?,
            failure,
        })
    })())
}

fn encode_trigger(trigger: &Trigger) -> Result<String, CoreError> {
    serde_json::to_string(trigger).map_err(|_| {
        CoreError::new(
            ErrorCode::DataIntegrity,
            "Nao consegui serializar o trigger.",
            false,
        )
    })
}

impl AttentionRepository for SqliteStorage {
    fn create_reminder(&self, reminder: NewReminder) -> Result<Reminder, CoreError> {
        let id = reminder.id;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        insert_reminder(&transaction, &reminder)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("reminder", id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: campos_do_lembrete_novo(&reminder)?,
            },
        )?;
        transaction.commit().map_err(map_sql_error)?;
        drop(connection);
        self.reminder(id)
    }

    fn reminder(&self, id: ReminderId) -> Result<Reminder, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let found = connection
            .query_row(
                &format!("SELECT {REMINDER_COLUMNS} FROM reminders WHERE id = ?1"),
                params![id.to_string()],
                read_reminder,
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    CoreError::new(ErrorCode::NotFound, "Lembrete nao encontrado.", false)
                }
                other => map_sql_error(other),
            })?;
        found
    }

    fn waiting_reminders(&self) -> Result<Vec<Reminder>, CoreError> {
        // A ordem e a do indice parcial `reminders_waiting`: quem vence antes
        // vem antes, e o agendador so precisa do primeiro.
        self.query_reminders(
            "WHERE status IN ('scheduled', 'snoozed') AND lifecycle_state = 'active' \
             ORDER BY next_due_at",
        )
    }

    fn open_reminders(&self) -> Result<Vec<Reminder>, CoreError> {
        self.query_reminders(
            "WHERE status NOT IN ('completed', 'cancelled', 'expired') \
             AND lifecycle_state = 'active' \
             ORDER BY next_due_at",
        )
    }

    fn resolved_reminders(&self, limit: usize) -> Result<Vec<Reminder>, CoreError> {
        // `updated_at` e nao `completed_at`: cancelar nao preenche o segundo, e
        // ordenar por ele poria os cancelados no fim da lista para sempre — como
        // se ninguem os tivesse tocado.
        self.query_reminders(&format!(
            "WHERE status IN ('completed', 'cancelled', 'expired')              AND lifecycle_state = 'active'              ORDER BY updated_at DESC LIMIT {limit}"
        ))
    }

    fn save_reminder(&self, reminder: &Reminder) -> Result<Reminder, CoreError> {
        let connection = self.escrita()?;
        let (target_type, target_id) = match reminder.target {
            Some(target) => {
                let (kind, id) = target.as_columns();
                (Some(kind.to_owned()), Some(id))
            }
            None => (None, None),
        };

        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE reminders SET title = ?2, body = ?3, target_type = ?4, target_id = ?5, \
                 trigger_kind = ?6, trigger = ?7, priority = ?8, status = ?9, \
                 snooze_allowed = ?10, privacy = ?11, next_due_at = ?12, snooze_count = ?13, \
                 delivered_count = ?14, updated_at = ?15, completed_at = ?16, kind = ?17, \
                 waiting_for = ?18, persistent = ?19, escalation_step = ?20, \
                 last_triggered_at = ?21, retry_at = ?22, recurrence = ?23 WHERE id = ?1",
                params![
                    reminder.id.to_string(),
                    reminder.title,
                    reminder.body,
                    target_type,
                    target_id,
                    reminder.trigger.kind_str(),
                    encode_trigger(&reminder.trigger)?,
                    reminder.priority.as_str(),
                    reminder.status.as_str(),
                    i64::from(reminder.policy.snooze_allowed),
                    reminder.policy.privacy.as_str(),
                    reminder.next_due_at.map(format_time).transpose()?,
                    i64::from(reminder.snooze_count),
                    i64::from(reminder.delivered_count),
                    format_time(reminder.updated_at)?,
                    reminder.completed_at.map(format_time).transpose()?,
                    reminder.kind.as_str(),
                    reminder.waiting_for,
                    i64::from(reminder.persistent),
                    i64::from(reminder.escalation_step),
                    reminder.last_triggered_at.map(format_time).transpose()?,
                    reminder.retry_at.map(format_time).transpose()?,
                    encode_recurrence(reminder.recurrence.as_ref())?,
                ],
            )
            .map_err(map_sql_error)?;

        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Lembrete nao encontrado.",
                false,
            ));
        }

        // A INTENCAO viaja; a ENTREGA fica.
        //
        // `delivered_count` conta quantas vezes ESTE dispositivo mostrou o
        // aviso, e isso e escrituracao local — o iPhone tocar nao significa que
        // o PC tocou. Sincronizar esse numero faria dois aparelhos disputarem um
        // contador que nem descreve a mesma coisa, e com merge por campo um
        // deles perderia a propria contagem.
        //
        // `snooze_count` viaja porque adiar e ACAO DA PESSOA: ela adiou o
        // lembrete, e nao o aparelho.
        self.emitir_update(
            &transaction,
            "reminder",
            reminder.id.as_uuid(),
            &campos_do_lembrete(reminder, target_type.as_deref(), target_id.as_deref())?,
        )?;
        transaction.commit().map_err(map_sql_error)?;

        drop(connection);
        self.reminder(reminder.id)
    }

    fn set_reminder_lifecycle(
        &self,
        id: ReminderId,
        state: LifecycleState,
    ) -> Result<Reminder, CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE reminders SET lifecycle_state = ?2, updated_at = ?3 WHERE id = ?1",
                params![
                    id.to_string(),
                    state.as_str(),
                    format_time(OffsetDateTime::now_utc())?,
                ],
            )
            .map_err(map_sql_error)?;

        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Lembrete nao encontrado.",
                false,
            ));
        }

        self.emitir_update(
            &transaction,
            "reminder",
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(state.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;

        drop(connection);
        self.reminder(id)
    }

    fn record_notification(
        &self,
        notification: NewNotification,
    ) -> Result<Notification, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "INSERT INTO attention_notifications \
                 (id, reminder_id, channel, dedupe_key, status, level, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    notification.id.to_string(),
                    notification.reminder_id.to_string(),
                    notification.channel.as_str(),
                    notification.dedupe_key,
                    NotificationStatus::Queued.as_str(),
                    notification.level.as_str(),
                    format_time(notification.created_at)?,
                ],
            )
            .map_err(map_sql_error)?;

        connection
            .query_row(
                &format!(
                    "SELECT {NOTIFICATION_COLUMNS} FROM attention_notifications WHERE id = ?1"
                ),
                params![notification.id.to_string()],
                read_notification,
            )
            .map_err(map_sql_error)?
    }

    fn save_notification(&self, notification: &Notification) -> Result<Notification, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE attention_notifications SET status = ?2, level = ?3, delivered_at = ?4, \
                 resolved_at = ?5, failure = ?6 WHERE id = ?1",
                params![
                    notification.id.to_string(),
                    notification.status.as_str(),
                    notification.level.as_str(),
                    notification.delivered_at.map(format_time).transpose()?,
                    notification.resolved_at.map(format_time).transpose()?,
                    notification.failure,
                ],
            )
            .map_err(map_sql_error)?;

        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Notificacao nao encontrada.",
                false,
            ));
        }

        connection
            .query_row(
                &format!(
                    "SELECT {NOTIFICATION_COLUMNS} FROM attention_notifications WHERE id = ?1"
                ),
                params![notification.id.to_string()],
                read_notification,
            )
            .map_err(map_sql_error)?
    }

    fn live_notification(&self, dedupe_key: &str) -> Result<Option<Notification>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {NOTIFICATION_COLUMNS} FROM attention_notifications \
                 WHERE dedupe_key = ?1 AND status IN ('queued', 'delivering', 'delivered') \
                 ORDER BY created_at DESC LIMIT 1"
            ))
            .map_err(map_sql_error)?;
        let mut rows = statement
            .query_map(params![dedupe_key], read_notification)
            .map_err(map_sql_error)?;

        match rows.next() {
            Some(row) => Ok(Some(row.map_err(map_sql_error)??)),
            None => Ok(None),
        }
    }

    fn notifications_for(&self, reminder: ReminderId) -> Result<Vec<Notification>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {NOTIFICATION_COLUMNS} FROM attention_notifications \
                 WHERE reminder_id = ?1 ORDER BY created_at"
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![reminder.to_string()], read_notification)
            .map_err(map_sql_error)?;

        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(map_sql_error)??);
        }
        Ok(found)
    }

    // ------------------------------------------------------------ re-alerta

    fn reminders_to_retry(&self, now: OffsetDateTime) -> Result<Vec<Reminder>, CoreError> {
        let limite = format_time(now)?;
        // O indice parcial `reminders_retry` responde esta consulta: so
        // lembrete com re-alerta marcado participa dele, e num banco com anos
        // de uso isso e um punhado de linhas em vez do acervo inteiro.
        self.query_reminders(&format!(
            "WHERE retry_at IS NOT NULL AND retry_at <= '{limite}' \
             AND lifecycle_state = 'active' \
             AND status NOT IN ('completed', 'cancelled', 'expired') \
             ORDER BY retry_at"
        ))
    }

    // ------------------------------------------------------ pilha de alertas

    fn create_trigger(
        &self,
        trigger: mos_core::NewReminderTrigger,
    ) -> Result<mos_core::ReminderTrigger, CoreError> {
        let id = trigger.id;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        insert_trigger(&transaction, &trigger)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("reminder_trigger", id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: campos_do_alerta_novo(&trigger)?,
            },
        )?;
        transaction.commit().map_err(map_sql_error)?;
        drop(connection);
        self.trigger(id)
    }

    fn triggers_for(
        &self,
        reminder: ReminderId,
    ) -> Result<Vec<mos_core::ReminderTrigger>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {TRIGGER_COLUMNS} FROM reminder_triggers \
                 WHERE reminder_id = ?1 AND lifecycle_state = 'active' \
                 ORDER BY scheduled_at"
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![reminder.to_string()], read_trigger)
            .map_err(map_sql_error)?;
        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(map_sql_error)??);
        }
        Ok(found)
    }

    fn set_trigger_status(
        &self,
        id: mos_core::ReminderTriggerId,
        status: mos_core::StackTriggerStatus,
        at: Option<OffsetDateTime>,
    ) -> Result<mos_core::ReminderTrigger, CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let agora = format_time(OffsetDateTime::now_utc())?;
        let changed = transaction
            .execute(
                "UPDATE reminder_triggers SET status = ?2, fired_at = ?3, updated_at = ?4 \
                 WHERE id = ?1",
                params![
                    id.to_string(),
                    status.as_str(),
                    at.map(format_time).transpose()?,
                    agora,
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Alerta nao encontrado.",
                false,
            ));
        }
        self.emitir_update(
            &transaction,
            "reminder_trigger",
            id.as_uuid(),
            &[
                ("status", serde_json::json!(status.as_str())),
                (
                    "firedAt",
                    serde_json::json!(at.map(format_time).transpose()?),
                ),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        drop(connection);
        self.trigger(id)
    }

    fn cancel_pending_triggers(&self, reminder: ReminderId) -> Result<usize, CoreError> {
        // Uma consulta para achar, uma para escrever, e as duas na MESMA
        // transacao: cancelar metade da pilha e pior que nao cancelar nenhuma,
        // porque a metade viva continuaria tocando por algo ja resolvido.
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        let pendentes: Vec<String> = {
            let mut statement = transaction
                .prepare(
                    "SELECT id FROM reminder_triggers WHERE reminder_id = ?1 \
                     AND status = 'pending' AND lifecycle_state = 'active'",
                )
                .map_err(map_sql_error)?;
            let rows = statement
                .query_map(params![reminder.to_string()], |row| row.get::<_, String>(0))
                .map_err(map_sql_error)?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(map_sql_error)?);
            }
            ids
        };

        if pendentes.is_empty() {
            return Ok(0);
        }

        let agora = format_time(OffsetDateTime::now_utc())?;
        transaction
            .execute(
                "UPDATE reminder_triggers SET status = 'cancelled', updated_at = ?2 \
                 WHERE reminder_id = ?1 AND status = 'pending' AND lifecycle_state = 'active'",
                params![reminder.to_string(), agora],
            )
            .map_err(map_sql_error)?;

        for id in &pendentes {
            let uuid = mos_core::ReminderTriggerId::parse(id)?.as_uuid();
            self.emitir_update(
                &transaction,
                "reminder_trigger",
                uuid,
                &[("status", serde_json::json!("cancelled"))],
            )?;
        }

        transaction.commit().map_err(map_sql_error)?;
        Ok(pendentes.len())
    }

    fn triggers_due(
        &self,
        now: OffsetDateTime,
    ) -> Result<Vec<mos_core::ReminderTrigger>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {TRIGGER_COLUMNS} FROM reminder_triggers \
                 WHERE status = 'pending' AND lifecycle_state = 'active' \
                 AND scheduled_at <= ?1 ORDER BY scheduled_at"
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![format_time(now)?], read_trigger)
            .map_err(map_sql_error)?;
        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(map_sql_error)??);
        }
        Ok(found)
    }

    // --------------------------------------------------------------- historico

    fn record_event(&self, event: mos_core::NewReminderEvent) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "INSERT INTO reminder_events (id, reminder_id, kind, at, detail, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event.id.to_string(),
                    event.reminder_id.to_string(),
                    event.kind.as_str(),
                    format_time(event.at)?,
                    event.detail,
                    format_time(OffsetDateTime::now_utc())?,
                ],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    fn events_for(
        &self,
        reminder: ReminderId,
        limit: usize,
    ) -> Result<Vec<mos_core::ReminderEvent>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {EVENT_COLUMNS} FROM reminder_events WHERE reminder_id = ?1 \
                 ORDER BY at DESC, rowid DESC LIMIT {limit}"
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![reminder.to_string()], read_event)
            .map_err(map_sql_error)?;
        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(map_sql_error)??);
        }
        Ok(found)
    }

    // ------------------------------------------------------------ configuracao

    fn attention_settings(&self) -> Result<mos_core::AttentionSettings, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .query_row(
                "SELECT quiet_enabled, quiet_start_minute, quiet_end_minute, \
                 quiet_allow_urgent, os_channel_enabled, local_offset_minutes \
                 FROM attention_settings WHERE id = 1",
                [],
                |row| {
                    let enabled: i64 = row.get(0)?;
                    let start: i64 = row.get(1)?;
                    let end: i64 = row.get(2)?;
                    let urgent: i64 = row.get(3)?;
                    let canal: i64 = row.get(4)?;
                    let fuso: i64 = row.get(5)?;
                    Ok(mos_core::AttentionSettings {
                        quiet: mos_core::QuietHours {
                            enabled: enabled != 0,
                            start_minute: start.clamp(0, 1439) as u16,
                            end_minute: end.clamp(0, 1439) as u16,
                            allow_urgent: urgent != 0,
                        },
                        os_channel_enabled: canal != 0,
                        local_offset_minutes: fuso.clamp(-840, 840) as i16,
                    })
                },
            )
            .map_err(map_sql_error)
    }

    fn save_attention_settings(
        &self,
        settings: mos_core::AttentionSettings,
    ) -> Result<mos_core::AttentionSettings, CoreError> {
        {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            connection
                .execute(
                    "UPDATE attention_settings SET quiet_enabled = ?1, quiet_start_minute = ?2, \
                     quiet_end_minute = ?3, quiet_allow_urgent = ?4, os_channel_enabled = ?5, \
                     local_offset_minutes = ?6 WHERE id = 1",
                    params![
                        i64::from(settings.quiet.enabled),
                        i64::from(settings.quiet.start_minute),
                        i64::from(settings.quiet.end_minute),
                        i64::from(settings.quiet.allow_urgent),
                        i64::from(settings.os_channel_enabled),
                        i64::from(settings.local_offset_minutes),
                    ],
                )
                .map_err(map_sql_error)?;
        }
        // Le de volta em vez de devolver o que recebeu: o CHECK do banco e quem
        // tem a ultima palavra sobre o que ficou gravado. Mesma regra do
        // `save_reminder`.
        self.attention_settings()
    }
}

/// Os lembretes vivos de um alvo, numa conexao ou transacao ja aberta.
///
/// Existe fora do trait porque a gaveta da Task monta Task, checklist,
/// referencias e lembretes numa consulta so — e pedir isto pelo `AttentionRepository`
/// tomaria a conexao uma segunda vez, no meio de quem ja a tem na mao.
///
/// O indice `reminders_target` (migration 0015) foi criado exatamente para esta
/// consulta: *"para achar os lembretes de uma Task ao abrir a Task"*.
pub(crate) fn query_reminders_for_target(
    connection: &rusqlite::Connection,
    target_type: &str,
    target_id: &str,
) -> Result<Vec<Reminder>, CoreError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {REMINDER_COLUMNS} FROM reminders
              WHERE target_type = ?1 AND target_id = ?2 AND lifecycle_state = 'active'
              ORDER BY next_due_at IS NULL, next_due_at ASC"
        ))
        .map_err(map_sql_error)?;
    let linhas = statement
        .query_map(params![target_type, target_id], read_reminder)
        .map_err(map_sql_error)?;
    let mut achados = Vec::new();
    for linha in linhas {
        achados.push(linha.map_err(map_sql_error)??);
    }
    Ok(achados)
}

impl SqliteStorage {
    fn query_reminders(&self, tail: &str) -> Result<Vec<Reminder>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!("SELECT {REMINDER_COLUMNS} FROM reminders {tail}"))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], read_reminder)
            .map_err(map_sql_error)?;

        let mut found = Vec::new();
        for row in rows {
            found.push(row.map_err(map_sql_error)??);
        }
        Ok(found)
    }
}

/// Insere um Reminder numa conexao ou transacao ja aberta.
///
/// `pub(crate)` pela mesma razao que `insert_task`: aceitar um item de reuniao
/// cria Task e Reminder juntos, e "juntos" precisa ser uma transacao so — senao
/// existe um instante em que a Task existe e o lembrete dela nao, e uma queda
/// ali deixaria o compromisso sem aviso.
/// Os campos de um lembrete que VIAJAM.
///
/// Fora daqui, de proposito: `delivered_count`. Ele conta quantas vezes ESTE
/// dispositivo mostrou o aviso, e isso e escrituracao local — o iPhone tocar
/// nao significa que o PC tocou.
fn campos_do_lembrete(
    reminder: &mos_core::Reminder,
    target_type: Option<&str>,
    target_id: Option<&str>,
) -> Result<Vec<(&'static str, serde_json::Value)>, CoreError> {
    Ok(vec![
        ("title", serde_json::json!(reminder.title)),
        ("body", serde_json::json!(reminder.body)),
        ("targetType", serde_json::json!(target_type)),
        ("targetId", serde_json::json!(target_id)),
        (
            "triggerKind",
            serde_json::json!(reminder.trigger.kind_str()),
        ),
        (
            "trigger",
            serde_json::json!(encode_trigger(&reminder.trigger)?),
        ),
        ("priority", serde_json::json!(reminder.priority.as_str())),
        ("status", serde_json::json!(reminder.status.as_str())),
        (
            "snoozeAllowed",
            serde_json::json!(reminder.policy.snooze_allowed),
        ),
        (
            "privacy",
            serde_json::json!(reminder.policy.privacy.as_str()),
        ),
        (
            "nextDueAt",
            serde_json::json!(reminder.next_due_at.map(format_time).transpose()?),
        ),
        ("snoozeCount", serde_json::json!(reminder.snooze_count)),
        (
            "completedAt",
            serde_json::json!(reminder.completed_at.map(format_time).transpose()?),
        ),
        // A partir daqui, o que a migration 0040 acrescentou — e so o que e
        // DECISAO DA PESSOA. `escalationStep`, `retryAt` e `lastTriggeredAt`
        // ficam de fora pela mesma razao de `deliveredCount`: eles descrevem a
        // insistencia DESTE aparelho, e dois agendadores disputando essas
        // colunas fariam um deles silenciar o outro.
        ("kind", serde_json::json!(reminder.kind.as_str())),
        ("waitingFor", serde_json::json!(reminder.waiting_for)),
        ("persistent", serde_json::json!(reminder.persistent)),
        (
            "recurrence",
            serde_json::json!(encode_recurrence(reminder.recurrence.as_ref())?),
        ),
    ])
}

/// O mesmo, para um lembrete que acabou de nascer.
fn campos_do_lembrete_novo(
    reminder: &NewReminder,
) -> Result<serde_json::Map<String, serde_json::Value>, CoreError> {
    let (target_type, target_id) = match reminder.target {
        Some(target) => {
            let (kind, id) = target.as_columns();
            (Some(kind.to_owned()), Some(id))
        }
        None => (None, None),
    };
    Ok([
        ("title".to_owned(), serde_json::json!(reminder.title)),
        ("body".to_owned(), serde_json::json!(reminder.body)),
        ("targetType".to_owned(), serde_json::json!(target_type)),
        ("targetId".to_owned(), serde_json::json!(target_id)),
        (
            "triggerKind".to_owned(),
            serde_json::json!(reminder.trigger.kind_str()),
        ),
        (
            "trigger".to_owned(),
            serde_json::json!(encode_trigger(&reminder.trigger)?),
        ),
        (
            "priority".to_owned(),
            serde_json::json!(reminder.priority.as_str()),
        ),
        (
            "snoozeAllowed".to_owned(),
            serde_json::json!(reminder.policy.snooze_allowed),
        ),
        (
            "privacy".to_owned(),
            serde_json::json!(reminder.policy.privacy.as_str()),
        ),
        // `nextDueAt` no CREATE, e nao so no primeiro update.
        //
        // Sem ele, um lembrete criado no PC chegava ao celular sem hora: o
        // `Mapa` tem a coluna, mas ninguem a preenchia ate a primeira
        // transicao. O celular entao mostrava o lembrete sem quando, e o laco
        // de avisos nunca o via vencer — o lembrete existia nos dois aparelhos
        // e so funcionava num.
        (
            "nextDueAt".to_owned(),
            serde_json::json!(reminder.next_due_at.map(format_time).transpose()?),
        ),
        (
            "status".to_owned(),
            serde_json::json!(ReminderStatus::Scheduled.as_str()),
        ),
        ("lifecycleState".to_owned(), serde_json::json!("active")),
        ("snoozeCount".to_owned(), serde_json::json!(0)),
        ("kind".to_owned(), serde_json::json!(reminder.kind.as_str())),
        (
            "waitingFor".to_owned(),
            serde_json::json!(reminder.waiting_for),
        ),
        (
            "persistent".to_owned(),
            serde_json::json!(reminder.persistent),
        ),
        (
            "recurrence".to_owned(),
            serde_json::json!(encode_recurrence(reminder.recurrence.as_ref())?),
        ),
    ]
    .into_iter()
    .collect())
}

pub(crate) fn insert_reminder(
    connection: &rusqlite::Connection,
    reminder: &NewReminder,
) -> Result<(), CoreError> {
    let (target_type, target_id) = match reminder.target {
        Some(target) => {
            let (kind, id) = target.as_columns();
            (Some(kind.to_owned()), Some(id))
        }
        None => (None, None),
    };
    connection
        .execute(
            "INSERT INTO reminders (id, title, body, target_type, target_id, trigger_kind, \
             trigger, priority, status, source, snooze_allowed, privacy, next_due_at, \
             created_at, updated_at, kind, waiting_for, persistent, recurrence) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14, \
             ?15, ?16, ?17, ?18)",
            params![
                reminder.id.to_string(),
                reminder.title,
                reminder.body,
                target_type,
                target_id,
                reminder.trigger.kind_str(),
                encode_trigger(&reminder.trigger)?,
                reminder.priority.as_str(),
                ReminderStatus::Scheduled.as_str(),
                reminder.source.as_str(),
                i64::from(reminder.policy.snooze_allowed),
                reminder.policy.privacy.as_str(),
                reminder.next_due_at.map(format_time).transpose()?,
                format_time(reminder.created_at)?,
                reminder.kind.as_str(),
                reminder.waiting_for,
                i64::from(reminder.persistent),
                encode_recurrence(reminder.recurrence.as_ref())?,
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

/// Insere um alerta de pilha numa conexao ou transacao ja aberta.
pub(crate) fn insert_trigger(
    connection: &rusqlite::Connection,
    trigger: &mos_core::NewReminderTrigger,
) -> Result<(), CoreError> {
    connection
        .execute(
            "INSERT INTO reminder_triggers (id, reminder_id, scheduled_at, kind, lead_minutes, \
             status, lifecycle_state, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 'active', ?6, ?6)",
            params![
                trigger.id.to_string(),
                trigger.reminder_id.to_string(),
                format_time(trigger.scheduled_at)?,
                trigger.kind.as_str(),
                trigger.lead_minutes.map(i64::from),
                format_time(trigger.created_at)?,
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

/// Os campos de um alerta que VIAJAM.
///
/// A pilha inteira viaja porque ela e a INTENCAO: escolher quatro alertas para
/// uma entrega e uma decisao da pessoa, e ela vale nos dois aparelhos. O que nao
/// viaja e a ENTREGA — `attention_notifications` continua local, pela mesma
/// razao de sempre.
fn campos_do_alerta_novo(
    trigger: &mos_core::NewReminderTrigger,
) -> Result<serde_json::Map<String, serde_json::Value>, CoreError> {
    Ok([
        (
            "reminderId".to_owned(),
            serde_json::json!(trigger.reminder_id.to_string()),
        ),
        (
            "scheduledAt".to_owned(),
            serde_json::json!(format_time(trigger.scheduled_at)?),
        ),
        ("kind".to_owned(), serde_json::json!(trigger.kind.as_str())),
        (
            "leadMinutes".to_owned(),
            serde_json::json!(trigger.lead_minutes),
        ),
        ("status".to_owned(), serde_json::json!("pending")),
        ("firedAt".to_owned(), serde_json::Value::Null),
        ("lifecycleState".to_owned(), serde_json::json!("active")),
        (
            "createdAt".to_owned(),
            serde_json::json!(format_time(trigger.created_at)?),
        ),
    ]
    .into_iter()
    .collect())
}

impl SqliteStorage {
    fn trigger(
        &self,
        id: mos_core::ReminderTriggerId,
    ) -> Result<mos_core::ReminderTrigger, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .query_row(
                &format!("SELECT {TRIGGER_COLUMNS} FROM reminder_triggers WHERE id = ?1"),
                params![id.to_string()],
                read_trigger,
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    CoreError::new(ErrorCode::NotFound, "Alerta nao encontrado.", false)
                }
                other => map_sql_error(other),
            })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{Clock, FixedClock, Priority, Transition};
    use time::Duration;

    fn storage() -> (SqliteStorage, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (storage, directory)
    }

    fn clock() -> FixedClock {
        FixedClock::at(OffsetDateTime::UNIX_EPOCH + Duration::days(20_000))
    }

    fn new_reminder(clock: &FixedClock, hours_ahead: i64) -> NewReminder {
        NewReminder::at(
            "Enviar proposta",
            "para o cliente novo",
            clock.now() + Duration::hours(hours_ahead),
            clock,
        )
        .unwrap()
    }

    #[test]
    fn a_reminder_survives_the_round_trip() {
        let (storage, _guard) = storage();
        let clock = clock();
        let created = storage.create_reminder(new_reminder(&clock, 3)).unwrap();

        assert_eq!(created.title, "Enviar proposta");
        assert_eq!(created.body, "para o cliente novo");
        assert_eq!(created.status, ReminderStatus::Scheduled);
        assert_eq!(created.next_due_at, Some(clock.now() + Duration::hours(3)));
        assert_eq!(created.trigger.kind_str(), "at");
        assert_eq!(created.lifecycle_state, LifecycleState::Active);

        let read = storage.reminder(created.id).unwrap();
        assert_eq!(read.id, created.id);
        assert_eq!(read.trigger, created.trigger);
    }

    #[test]
    fn a_missing_reminder_is_not_found_and_not_a_crash() {
        let (storage, _guard) = storage();
        let error = storage.reminder(ReminderId::new()).unwrap_err();
        assert_eq!(error.code, ErrorCode::NotFound);
    }

    /// O alvo atravessa como par (tipo, id). Um round-trip por braco porque
    /// errar o mapeamento de um so nao quebraria os outros.
    #[test]
    fn every_target_kind_survives_the_database() {
        let (storage, _guard) = storage();
        let clock = clock();
        let targets = [
            ReminderTarget::Task(mos_core::TaskId::new()),
            ReminderTarget::Project(mos_core::ProjectId::new()),
            ReminderTarget::Capture(mos_core::CaptureId::new()),
            ReminderTarget::Resource(mos_core::ResourceId::new()),
            ReminderTarget::Conversation(mos_core::ConversationId::new()),
            ReminderTarget::App(mos_core::AppId::new()),
        ];

        for target in targets {
            let created = storage
                .create_reminder(new_reminder(&clock, 3).with_target(target))
                .unwrap();
            assert_eq!(storage.reminder(created.id).unwrap().target, Some(target));
        }
    }

    /// A migration declara os valores validos em `CHECK` e o dominio os declara
    /// em `as_str`. Sao dois lugares, e dois lugares divergem. Este teste grava
    /// cada valor do dominio e deixa o banco recusar se discordar.
    #[test]
    fn the_database_accepts_every_value_the_domain_knows() {
        let (storage, _guard) = storage();
        let clock = clock();

        for priority in [
            Priority::Low,
            Priority::Normal,
            Priority::High,
            Priority::Urgent,
        ] {
            let created = storage
                .create_reminder(new_reminder(&clock, 3).with_priority(priority))
                .unwrap();
            assert_eq!(created.priority, priority);
        }

        for source in [
            ReminderSource::User,
            ReminderSource::Hermes,
            ReminderSource::Capture,
            ReminderSource::System,
        ] {
            let created = storage
                .create_reminder(new_reminder(&clock, 3).from_source(source))
                .unwrap();
            assert_eq!(created.source, source);
        }

        let base = storage.create_reminder(new_reminder(&clock, 3)).unwrap();

        for privacy in [
            ContentPrivacy::ShowContent,
            ContentPrivacy::TitleOnly,
            ContentPrivacy::Hidden,
        ] {
            let mut subject = base.clone();
            subject.policy.privacy = privacy;
            assert_eq!(
                storage.save_reminder(&subject).unwrap().policy.privacy,
                privacy
            );
        }

        for status in [
            ReminderStatus::Scheduled,
            ReminderStatus::Due,
            ReminderStatus::Delivered,
            ReminderStatus::Acknowledged,
            ReminderStatus::Snoozed,
            ReminderStatus::Missed,
            ReminderStatus::Expired,
            ReminderStatus::Cancelled,
        ] {
            let mut subject = base.clone();
            subject.status = status;
            assert_eq!(
                storage.save_reminder(&subject).unwrap().status,
                status,
                "o banco recusou {}",
                status.as_str()
            );
        }

        for state in [
            LifecycleState::Active,
            LifecycleState::Archived,
            LifecycleState::Trashed,
        ] {
            assert_eq!(
                storage
                    .set_reminder_lifecycle(base.id, state)
                    .unwrap()
                    .lifecycle_state,
                state
            );
        }
    }

    /// `completed_at` e exclusivo de `completed`, e o banco impoe. Sem isso,
    /// um bug de transicao gravaria um carimbo de conclusao num lembrete vivo,
    /// e nada acusaria.
    #[test]
    fn the_database_refuses_a_completion_stamp_without_completion() {
        let (storage, _guard) = storage();
        let clock = clock();
        let created = storage.create_reminder(new_reminder(&clock, 3)).unwrap();

        let mut wrong = created.clone();
        wrong.completed_at = Some(clock.now());
        assert!(
            storage.save_reminder(&wrong).is_err(),
            "carimbo sem conclusao passou"
        );

        let right = mos_core::apply(&created, Transition::Complete, clock.now()).unwrap();
        let saved = storage.save_reminder(&right).unwrap();
        assert_eq!(saved.status, ReminderStatus::Completed);
        assert!(saved.completed_at.is_some());
    }

    #[test]
    fn waiting_only_returns_what_the_scheduler_cares_about() {
        let (storage, _guard) = storage();
        let clock = clock();

        let soon = storage.create_reminder(new_reminder(&clock, 1)).unwrap();
        let later = storage.create_reminder(new_reminder(&clock, 5)).unwrap();
        let done = storage.create_reminder(new_reminder(&clock, 2)).unwrap();
        storage
            .save_reminder(&mos_core::apply(&done, Transition::Complete, clock.now()).unwrap())
            .unwrap();
        let archived = storage.create_reminder(new_reminder(&clock, 3)).unwrap();
        storage
            .set_reminder_lifecycle(archived.id, LifecycleState::Archived)
            .unwrap();

        let waiting = storage.waiting_reminders().unwrap();
        let ids: Vec<_> = waiting.iter().map(|reminder| reminder.id).collect();

        assert_eq!(
            ids,
            vec![soon.id, later.id],
            "concluido e arquivado ficam fora"
        );
        assert_eq!(
            mos_core::next_wake(&waiting),
            Some(clock.now() + Duration::hours(1))
        );
    }

    #[test]
    fn open_keeps_what_the_surface_shows_and_drops_what_ended() {
        let (storage, _guard) = storage();
        let clock = clock();

        let waiting = storage.create_reminder(new_reminder(&clock, 1)).unwrap();
        let missed = storage.create_reminder(new_reminder(&clock, 2)).unwrap();
        storage
            .save_reminder(&mos_core::apply(&missed, Transition::Miss, clock.now()).unwrap())
            .unwrap();
        let cancelled = storage.create_reminder(new_reminder(&clock, 3)).unwrap();
        storage
            .save_reminder(&mos_core::apply(&cancelled, Transition::Cancel, clock.now()).unwrap())
            .unwrap();

        let open = storage.open_reminders().unwrap();
        let ids: Vec<_> = open.iter().map(|reminder| reminder.id).collect();

        assert!(ids.contains(&waiting.id));
        assert!(ids.contains(&missed.id), "perdido continua na superficie");
        assert!(!ids.contains(&cancelled.id));
    }

    /// A transicao inteira, ida e volta pelo banco: o que foi gravado e o que
    /// se le depois, e nao o que ficou na memoria de quem gravou.
    #[test]
    fn a_snooze_survives_the_database() {
        let (storage, _guard) = storage();
        let clock = clock();
        let created = storage.create_reminder(new_reminder(&clock, 1)).unwrap();

        let until = clock.now() + Duration::hours(4);
        let snoozed = mos_core::apply(&created, Transition::Snooze { until }, clock.now()).unwrap();
        storage.save_reminder(&snoozed).unwrap();

        let read = storage.reminder(created.id).unwrap();
        assert_eq!(read.status, ReminderStatus::Snoozed);
        assert_eq!(read.next_due_at, Some(until));
        assert_eq!(read.snooze_count, 1);
    }

    // ---------------------------------------------------------- notificações

    #[test]
    fn a_notification_records_and_reads_back() {
        let (storage, _guard) = storage();
        let clock = clock();
        let reminder = storage.create_reminder(new_reminder(&clock, 1)).unwrap();

        let recorded = storage
            .record_notification(NewNotification::queued(
                reminder.id,
                Channel::InApp,
                "reminder-due",
                VisualLevel::Normal,
                clock.now(),
            ))
            .unwrap();

        assert_eq!(recorded.status, NotificationStatus::Queued);
        assert_eq!(recorded.channel, Channel::InApp);
        assert_eq!(recorded.dedupe_key, format!("reminder-due:{}", reminder.id));

        let all = storage.notifications_for(reminder.id).unwrap();
        assert_eq!(all.len(), 1);
    }

    /// Sem isto, "Task atrasada" quatro vezes seguidas.
    #[test]
    fn a_live_notification_is_found_by_its_dedupe_key() {
        let (storage, _guard) = storage();
        let clock = clock();
        let reminder = storage.create_reminder(new_reminder(&clock, 1)).unwrap();
        let key = NewNotification::dedupe_key("reminder-due", reminder.id);

        assert!(storage.live_notification(&key).unwrap().is_none());

        let recorded = storage
            .record_notification(NewNotification::queued(
                reminder.id,
                Channel::InApp,
                "reminder-due",
                VisualLevel::Normal,
                clock.now(),
            ))
            .unwrap();

        assert_eq!(
            storage.live_notification(&key).unwrap().map(|n| n.id),
            Some(recorded.id)
        );
    }

    /// Depois de vista, a proxima entrega e um lembrete novo e legitimo. Se
    /// `Seen` continuasse bloqueando, um Reminder adiado silenciaria para
    /// sempre depois da primeira vez.
    #[test]
    fn a_seen_notification_stops_blocking_duplicates() {
        let (storage, _guard) = storage();
        let clock = clock();
        let reminder = storage.create_reminder(new_reminder(&clock, 1)).unwrap();
        let key = NewNotification::dedupe_key("reminder-due", reminder.id);

        let mut recorded = storage
            .record_notification(NewNotification::queued(
                reminder.id,
                Channel::InApp,
                "reminder-due",
                VisualLevel::Normal,
                clock.now(),
            ))
            .unwrap();

        recorded.status = NotificationStatus::Seen;
        recorded.resolved_at = Some(clock.now());
        storage.save_notification(&recorded).unwrap();

        assert!(storage.live_notification(&key).unwrap().is_none());
    }

    /// Falha de entrega guarda o motivo — "nao apareceu as 15h" precisa ter
    /// resposta — e nao resolve o Reminder.
    #[test]
    fn a_failed_delivery_keeps_its_reason_and_leaves_the_reminder_alive() {
        let (storage, _guard) = storage();
        let clock = clock();
        let reminder = storage.create_reminder(new_reminder(&clock, 1)).unwrap();

        let mut recorded = storage
            .record_notification(NewNotification::queued(
                reminder.id,
                Channel::Windows,
                "reminder-due",
                VisualLevel::Normal,
                clock.now(),
            ))
            .unwrap();
        recorded.status = NotificationStatus::Failed;
        recorded.failure = Some("toast recusado pelo sistema".into());
        let saved = storage.save_notification(&recorded).unwrap();

        assert_eq!(saved.status, NotificationStatus::Failed);
        assert_eq!(
            saved.failure.as_deref(),
            Some("toast recusado pelo sistema")
        );

        let still = storage.reminder(reminder.id).unwrap();
        assert_eq!(
            still.status,
            ReminderStatus::Scheduled,
            "falha de canal nao pode resolver a intencao"
        );
    }

    #[test]
    fn every_channel_and_level_survives_the_database() {
        let (storage, _guard) = storage();
        let clock = clock();
        let reminder = storage.create_reminder(new_reminder(&clock, 1)).unwrap();

        for channel in [Channel::InApp, Channel::Windows, Channel::Tray] {
            let recorded = storage
                .record_notification(NewNotification::queued(
                    reminder.id,
                    channel,
                    "reminder-due",
                    VisualLevel::Normal,
                    clock.now(),
                ))
                .unwrap();
            assert_eq!(recorded.channel, channel);
        }

        let base = storage
            .record_notification(NewNotification::queued(
                reminder.id,
                Channel::InApp,
                "x",
                VisualLevel::Normal,
                clock.now(),
            ))
            .unwrap();

        for level in [
            VisualLevel::Quiet,
            VisualLevel::Normal,
            VisualLevel::Important,
            VisualLevel::Critical,
        ] {
            let mut subject = base.clone();
            subject.level = level;
            assert_eq!(storage.save_notification(&subject).unwrap().level, level);
        }

        for status in [
            NotificationStatus::Queued,
            NotificationStatus::Delivering,
            NotificationStatus::Delivered,
            NotificationStatus::Seen,
            NotificationStatus::Acted,
            NotificationStatus::Dismissed,
            NotificationStatus::Failed,
        ] {
            let mut subject = base.clone();
            subject.status = status;
            assert_eq!(
                storage.save_notification(&subject).unwrap().status,
                status,
                "o banco recusou {}",
                status.as_str()
            );
        }
    }

    // ------------------------------------------------- o servico de ponta a ponta
    //
    // Estes exercitam `AttentionService` contra o banco de verdade com relogio
    // falso. O agendador do desktop e uma casca fina em volta disto — dorme,
    // acorda e chama — e o binario de teste do `mos-desktop` nao sobe nesta
    // maquina, entao e aqui que a logica precisa ficar coberta.

    use mos_core::{AttentionService, ReconcileReason};
    use std::sync::Arc;

    fn service() -> (AttentionService, FixedClock, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let storage = Arc::new(
            SqliteStorage::open(
                directory.path().join("mos.db"),
                directory.path().join("backups"),
            )
            .unwrap(),
        );
        let clock = clock();
        let service = AttentionService::new(storage, Arc::new(clock.clone()));
        (service, clock, directory)
    }

    /// O ciclo inteiro: criar, esperar, vencer, entregar, concluir.
    #[test]
    fn the_whole_life_of_a_reminder() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at(
                "Enviar proposta",
                "",
                clock.now() + Duration::hours(2),
                None,
                ReminderSource::User,
            )
            .unwrap();

        // Antes da hora, nada acontece.
        assert!(service.reconcile().unwrap().is_empty());
        assert_eq!(service.needs_attention_count().unwrap(), 0);
        assert_eq!(
            service.next_wake().unwrap(),
            Some(clock.now() + Duration::hours(2))
        );

        clock.advance(Duration::hours(2));

        let rang = service.reconcile().unwrap();
        assert_eq!(rang.len(), 1);
        assert_eq!(rang[0].1, ReconcileReason::DueNow);
        assert_eq!(rang[0].0.status, ReminderStatus::Due);
        assert_eq!(service.needs_attention_count().unwrap(), 1);

        let queued = service
            .queue_delivery(
                created.id,
                Channel::InApp,
                "reminder-due",
                VisualLevel::Normal,
            )
            .unwrap()
            .expect("primeira entrega e criada");
        service.mark_delivered(&queued).unwrap();

        let after = service.reminder(created.id).unwrap();
        assert_eq!(after.status, ReminderStatus::Delivered);
        assert_eq!(after.delivered_count, 1);

        service
            .transition(created.id, Transition::Complete)
            .unwrap();
        assert_eq!(service.needs_attention_count().unwrap(), 0);
        assert!(service.next_wake().unwrap().is_none());
    }

    /// Chamada a cada acordada do laco. Se nao fosse idempotente, um tick
    /// duplicado entregaria duas vezes.
    #[test]
    fn reconciling_twice_changes_nothing_the_second_time() {
        let (service, clock, _guard) = service();
        service
            .create_at(
                "X",
                "",
                clock.now() + Duration::hours(1),
                None,
                ReminderSource::User,
            )
            .unwrap();

        clock.advance(Duration::hours(1));

        assert_eq!(service.reconcile().unwrap().len(), 1);
        assert!(
            service.reconcile().unwrap().is_empty(),
            "a segunda passada nao acha nada, porque a primeira tirou da espera"
        );
    }

    /// O caso do PC que dormiu, atravessando servico e banco.
    #[test]
    fn what_expired_while_away_comes_back_as_missed_with_its_original_delay() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at(
                "Ligar",
                "",
                clock.now() + Duration::hours(1),
                None,
                ReminderSource::User,
            )
            .unwrap();

        clock.advance(Duration::hours(9));

        let found = service.reconcile().unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, ReconcileReason::MissedWhileAway);
        assert_eq!(found[0].0.status, ReminderStatus::Missed);
        assert_eq!(
            found[0].0.overdue_by(clock.now()),
            Some(Duration::hours(8)),
            "o atraso conta do vencimento original, e nao de agora"
        );

        assert_eq!(
            service.reminder(created.id).unwrap().status,
            ReminderStatus::Missed
        );
        assert_eq!(service.needs_attention_count().unwrap(), 1);
    }

    /// Sem isto, "Task atrasada" a cada acordada do laco.
    #[test]
    fn a_second_delivery_of_the_same_subject_is_blocked_while_the_first_lives() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at(
                "X",
                "",
                clock.now() + Duration::minutes(1),
                None,
                ReminderSource::User,
            )
            .unwrap();
        clock.advance(Duration::minutes(1));
        service.reconcile().unwrap();

        assert!(service
            .queue_delivery(
                created.id,
                Channel::InApp,
                "reminder-due",
                VisualLevel::Normal
            )
            .unwrap()
            .is_some());
        assert!(
            service
                .queue_delivery(
                    created.id,
                    Channel::InApp,
                    "reminder-due",
                    VisualLevel::Normal
                )
                .unwrap()
                .is_none(),
            "a segunda com a mesma chave e recusada"
        );

        // Assunto diferente NAO e bloqueado: "venceu" e "foi perdido" sao
        // avisos diferentes sobre o mesmo Reminder.
        assert!(service
            .queue_delivery(
                created.id,
                Channel::InApp,
                "reminder-missed",
                VisualLevel::Normal
            )
            .unwrap()
            .is_some());
    }

    /// A invariante central do sistema, exercitada onde ela pode falhar.
    #[test]
    fn a_failed_delivery_leaves_the_reminder_needing_attention() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at(
                "X",
                "",
                clock.now() + Duration::minutes(1),
                None,
                ReminderSource::User,
            )
            .unwrap();
        clock.advance(Duration::minutes(1));
        service.reconcile().unwrap();

        let queued = service
            .queue_delivery(
                created.id,
                Channel::Windows,
                "reminder-due",
                VisualLevel::Normal,
            )
            .unwrap()
            .unwrap();
        service.mark_failed(&queued, "toast recusado").unwrap();

        let after = service.reminder(created.id).unwrap();
        assert_eq!(after.status, ReminderStatus::Due, "falha nao resolve nada");
        assert!(after.status.needs_attention());
        assert_eq!(service.needs_attention_count().unwrap(), 1);

        // E como a entrega morreu, o dedupe libera a proxima tentativa.
        assert!(service
            .queue_delivery(
                created.id,
                Channel::Windows,
                "reminder-due",
                VisualLevel::Normal
            )
            .unwrap()
            .is_some());
    }

    #[test]
    fn snoozing_takes_it_out_of_attention_and_puts_it_back_later() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at(
                "X",
                "",
                clock.now() + Duration::minutes(1),
                None,
                ReminderSource::User,
            )
            .unwrap();
        clock.advance(Duration::minutes(1));
        service.reconcile().unwrap();
        assert_eq!(service.needs_attention_count().unwrap(), 1);

        let until = clock.now() + Duration::hours(3);
        service
            .transition(created.id, Transition::Snooze { until })
            .unwrap();

        assert_eq!(
            service.needs_attention_count().unwrap(),
            0,
            "adiado nao cobra atencao: a pessoa ja decidiu quando quer ver"
        );
        assert_eq!(service.next_wake().unwrap(), Some(until));

        clock.advance(Duration::hours(3));
        let rang = service.reconcile().unwrap();
        assert_eq!(rang.len(), 1);
        assert_eq!(rang[0].0.status, ReminderStatus::Due);
    }

    /// Arquivar tira das superficies sem apagar — ADR-035.
    #[test]
    fn archiving_removes_it_from_the_scheduler_without_destroying_it() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at(
                "X",
                "",
                clock.now() + Duration::hours(1),
                None,
                ReminderSource::User,
            )
            .unwrap();

        service
            .set_lifecycle(created.id, LifecycleState::Archived)
            .unwrap();

        assert!(service.next_wake().unwrap().is_none());
        assert!(service.open().unwrap().is_empty());
        assert_eq!(
            service.reminder(created.id).unwrap().title,
            "X",
            "arquivado continua consultavel"
        );
    }

    /// O servico le do banco antes de decidir, e nao do que a interface tinha.
    #[test]
    fn a_transition_decides_on_the_stored_state_not_the_callers_copy() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at(
                "X",
                "",
                clock.now() + Duration::hours(1),
                None,
                ReminderSource::User,
            )
            .unwrap();

        service.transition(created.id, Transition::Cancel).unwrap();

        // Quem ainda segura a copia antiga tenta concluir; o servico recusa
        // porque o banco diz `cancelled`.
        let error = service
            .transition(created.id, Transition::Complete)
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidTransition);
    }

    /// A promessa central, exercitada contra o banco: fechar e reabrir nao
    /// perde nada, e o que venceu enquanto ninguem olhava volta como perdido
    /// com o instante original.
    #[test]
    fn reminders_survive_a_restart_and_the_overdue_ones_come_back_as_missed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        let clock = clock();

        let id = {
            let storage = SqliteStorage::open(path.clone(), backups.clone()).unwrap();
            storage.create_reminder(new_reminder(&clock, 1)).unwrap().id
        };

        // A maquina ficou fora do ar por trinta horas.
        clock.advance(Duration::hours(30));

        let storage = SqliteStorage::open(path, backups).unwrap();
        let waiting = storage.waiting_reminders().unwrap();
        assert_eq!(waiting.len(), 1, "o lembrete sobreviveu ao restart");

        let found = mos_core::reconcile(&waiting, clock.now());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, id);
        assert_eq!(found[0].reason, mos_core::ReconcileReason::MissedWhileAway);

        let missed = mos_core::apply(&waiting[0], Transition::Miss, clock.now()).unwrap();
        let saved = storage.save_reminder(&missed).unwrap();
        assert_eq!(saved.status, ReminderStatus::Missed);
        assert_eq!(
            saved.overdue_by(clock.now()),
            Some(Duration::hours(29)),
            "o atraso e contado do vencimento original"
        );
    }

    // ---------------------------------------------------- migration 0040

    /// Tudo que a 0040 acrescentou sobrevive à ida e à volta pelo banco.
    ///
    /// Um campo que se perde no round-trip é um lembrete que muda de
    /// comportamento sozinho depois do primeiro restart — que é a falha mais
    /// difícil de notar de todas.
    #[test]
    fn the_new_fields_survive_the_round_trip() {
        let (storage, _guard) = storage();
        let clock = clock();
        let regra = mos_core::Recurrence {
            rule: mos_core::RecurrenceRule::Weekly { days: vec![0, 3] },
            anchor: mos_core::RecurrenceAnchor::Completion,
            hour: 9,
            minute: 30,
            offset_minutes: -180,
        };
        let draft = new_reminder(&clock, 3)
            .persisting()
            .following_up("Victor")
            .repeating(regra.clone())
            .unwrap();

        let criado = storage.create_reminder(draft).unwrap();
        let lido = storage.reminder(criado.id).unwrap();

        assert!(lido.persistent);
        assert_eq!(lido.kind, mos_core::ReminderKind::FollowUp);
        assert_eq!(lido.waiting_for, "Victor");
        assert_eq!(lido.recurrence, Some(regra));
        assert_eq!(lido.escalation_step, 0);
        assert!(lido.retry_at.is_none());
    }

    /// O estado da insistência sobrevive ao restart. É o que impede um lembrete
    /// persistente de voltar ao degrau zero toda vez que o app abre — e insistir
    /// para sempre.
    #[test]
    fn the_escalation_state_survives_a_save() {
        let (storage, _guard) = storage();
        let clock = clock();
        let criado = storage
            .create_reminder(new_reminder(&clock, 1).persisting())
            .unwrap();

        let vencido =
            mos_core::apply(&criado, Transition::Ring, clock.now() + Duration::hours(1)).unwrap();
        let gravado = storage.save_reminder(&vencido).unwrap();
        assert!(gravado.retry_at.is_some());
        assert_eq!(
            gravado.last_triggered_at,
            Some(clock.now() + Duration::hours(1))
        );

        let insistiu = mos_core::apply(
            &gravado,
            Transition::Escalate,
            clock.now() + Duration::hours(2),
        )
        .unwrap();
        let relido = storage.save_reminder(&insistiu).unwrap();
        assert_eq!(relido.escalation_step, 1);
    }

    /// A consulta do re-alerta acha só quem já pode insistir.
    #[test]
    fn only_armed_reminders_show_up_for_retry() {
        let (storage, _guard) = storage();
        let clock = clock();
        let agora = clock.now();

        let insistente = storage
            .create_reminder(new_reminder(&clock, 1).persisting())
            .unwrap();
        let vencido = mos_core::apply(&insistente, Transition::Ring, agora).unwrap();
        storage.save_reminder(&vencido).unwrap();

        let comum = storage.create_reminder(new_reminder(&clock, 1)).unwrap();
        storage
            .save_reminder(&mos_core::apply(&comum, Transition::Ring, agora).unwrap())
            .unwrap();

        // Antes da hora do re-alerta: ninguém.
        assert!(storage.reminders_to_retry(agora).unwrap().is_empty());
        // Depois: só o persistente.
        let achados = storage
            .reminders_to_retry(agora + Duration::minutes(31))
            .unwrap();
        assert_eq!(achados.len(), 1);
        assert_eq!(achados[0].id, insistente.id);
    }

    /// Um lembrete sem data existe no banco e não aparece para o agendador.
    #[test]
    fn a_someday_reminder_is_stored_without_a_due_date() {
        let (storage, _guard) = storage();
        let clock = clock();
        let criado = storage
            .create_reminder(NewReminder::someday("Comprar cabo HDMI", "", &clock).unwrap())
            .unwrap();
        assert!(criado.next_due_at.is_none());
        assert_eq!(criado.trigger.kind_str(), "someday");
        // Continua na lista aberta — é lá que ele vive.
        assert!(storage
            .open_reminders()
            .unwrap()
            .iter()
            .any(|item| item.id == criado.id));
    }

    // ------------------------------------------------------ pilha de alertas

    /// Quatro alertas, UM lembrete. É o §16 e o §53 do pedido, no banco.
    #[test]
    fn a_stack_of_four_alerts_belongs_to_one_reminder() {
        let (storage, _guard) = storage();
        let clock = clock();
        let prazo = clock.now() + Duration::days(3);
        let entrega = storage
            .create_reminder(NewReminder::at("Entregar atividade", "", prazo, &clock).unwrap())
            .unwrap();

        storage
            .create_trigger(mos_core::NewReminderTrigger::at_due(
                entrega.id,
                prazo,
                clock.now(),
            ))
            .unwrap();
        for minutos in [24 * 60, 4 * 60, 60] {
            let alerta =
                mos_core::NewReminderTrigger::lead(entrega.id, prazo, minutos, clock.now())
                    .unwrap();
            storage.create_trigger(alerta).unwrap();
        }

        let pilha = storage.triggers_for(entrega.id).unwrap();
        assert_eq!(pilha.len(), 4);
        assert!(pilha.iter().all(|item| item.reminder_id == entrega.id));
        // Em ordem de relógio: o de um dia antes vem primeiro.
        assert_eq!(pilha[0].scheduled_at, prazo - Duration::days(1));
        assert_eq!(pilha[3].scheduled_at, prazo);

        // E continua sendo UM lembrete na lista.
        let abertos = storage.open_reminders().unwrap();
        assert_eq!(abertos.len(), 1);
    }

    #[test]
    fn firing_a_trigger_takes_it_out_of_the_pending_set() {
        let (storage, _guard) = storage();
        let clock = clock();
        let prazo = clock.now() + Duration::days(1);
        let lembrete = storage
            .create_reminder(NewReminder::at("Entrega", "", prazo, &clock).unwrap())
            .unwrap();
        let alerta = storage
            .create_trigger(mos_core::NewReminderTrigger::at_due(
                lembrete.id,
                prazo,
                clock.now(),
            ))
            .unwrap();

        assert_eq!(storage.triggers_due(prazo).unwrap().len(), 1);
        storage
            .set_trigger_status(alerta.id, mos_core::StackTriggerStatus::Fired, Some(prazo))
            .unwrap();
        assert!(storage.triggers_due(prazo).unwrap().is_empty());
        assert_eq!(
            storage.triggers_for(lembrete.id).unwrap()[0].status,
            mos_core::StackTriggerStatus::Fired
        );
    }

    /// Concluir mata a pilha pendente: quatro alertas para algo já resolvido são
    /// quatro interrupções que não significam nada.
    #[test]
    fn cancelling_the_stack_takes_every_pending_alert_at_once() {
        let (storage, _guard) = storage();
        let clock = clock();
        let prazo = clock.now() + Duration::days(3);
        let lembrete = storage
            .create_reminder(NewReminder::at("Entrega", "", prazo, &clock).unwrap())
            .unwrap();
        for minutos in [24 * 60, 60] {
            storage
                .create_trigger(
                    mos_core::NewReminderTrigger::lead(lembrete.id, prazo, minutos, clock.now())
                        .unwrap(),
                )
                .unwrap();
        }
        assert_eq!(storage.cancel_pending_triggers(lembrete.id).unwrap(), 2);
        assert!(storage
            .triggers_for(lembrete.id)
            .unwrap()
            .iter()
            .all(|item| item.status == mos_core::StackTriggerStatus::Cancelled));
        // Idempotente: a segunda vez não tem o que cancelar.
        assert_eq!(storage.cancel_pending_triggers(lembrete.id).unwrap(), 0);
    }

    // ------------------------------------------------------------ histórico

    #[test]
    fn the_history_reads_newest_first() {
        let (storage, _guard) = storage();
        let clock = clock();
        let lembrete = storage.create_reminder(new_reminder(&clock, 1)).unwrap();

        for (kind, quando) in [
            (mos_core::ReminderEventKind::Created, clock.now()),
            (
                mos_core::ReminderEventKind::Triggered,
                clock.now() + Duration::hours(1),
            ),
            (
                mos_core::ReminderEventKind::Snoozed,
                clock.now() + Duration::hours(2),
            ),
        ] {
            storage
                .record_event(mos_core::NewReminderEvent::new(lembrete.id, kind, quando))
                .unwrap();
        }

        let historico = storage.events_for(lembrete.id, 10).unwrap();
        assert_eq!(historico.len(), 3);
        assert_eq!(historico[0].kind, mos_core::ReminderEventKind::Snoozed);
        assert_eq!(historico[2].kind, mos_core::ReminderEventKind::Created);
    }

    // --------------------------------------------------------- configuração

    #[test]
    fn quiet_hours_start_on_and_survive_a_save() {
        let (storage, _guard) = storage();
        let padrao = storage.attention_settings().unwrap();
        assert!(padrao.quiet.enabled, "o silencio vem ligado");
        assert!(!padrao.quiet.allow_urgent, "e furar o silencio e opt-in");
        assert!(padrao.os_channel_enabled, "o canal do sistema vem ligado");

        let mudado = mos_core::AttentionSettings {
            quiet: mos_core::QuietHours {
                enabled: true,
                start_minute: 23 * 60,
                end_minute: 7 * 60,
                allow_urgent: true,
            },
            os_channel_enabled: false,
            local_offset_minutes: -180,
        };
        storage.save_attention_settings(mudado).unwrap();
        assert_eq!(storage.attention_settings().unwrap(), mudado);
    }
}
