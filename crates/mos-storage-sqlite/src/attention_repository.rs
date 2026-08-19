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
    LifecycleState, NewNotification, NewReminder, Notification, NotificationId,
    NotificationStatus, Priority, Reminder, ReminderId, ReminderSource, ReminderStatus,
    ReminderTarget, Trigger, VisualLevel,
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
     created_at, updated_at, completed_at, lifecycle_state";

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
        let connection = self.connection.lock().map_err(map_lock_error)?;
        insert_reminder(&connection, &reminder)?;
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

    fn save_reminder(&self, reminder: &Reminder) -> Result<Reminder, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let (target_type, target_id) = match reminder.target {
            Some(target) => {
                let (kind, id) = target.as_columns();
                (Some(kind.to_owned()), Some(id))
            }
            None => (None, None),
        };

        let changed = connection
            .execute(
                "UPDATE reminders SET title = ?2, body = ?3, target_type = ?4, target_id = ?5, \
                 trigger_kind = ?6, trigger = ?7, priority = ?8, status = ?9, \
                 snooze_allowed = ?10, privacy = ?11, next_due_at = ?12, snooze_count = ?13, \
                 delivered_count = ?14, updated_at = ?15, completed_at = ?16 WHERE id = ?1",
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

        drop(connection);
        self.reminder(reminder.id)
    }

    fn set_reminder_lifecycle(
        &self,
        id: ReminderId,
        state: LifecycleState,
    ) -> Result<Reminder, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
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
}

impl SqliteStorage {
    fn query_reminders(&self, tail: &str) -> Result<Vec<Reminder>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!("SELECT {REMINDER_COLUMNS} FROM reminders {tail}"))
            .map_err(map_sql_error)?;
        let rows = statement.query_map([], read_reminder).map_err(map_sql_error)?;

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
             created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
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
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
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
            assert_eq!(storage.save_reminder(&subject).unwrap().policy.privacy, privacy);
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

        assert_eq!(ids, vec![soon.id, later.id], "concluido e arquivado ficam fora");
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
        let snoozed =
            mos_core::apply(&created, Transition::Snooze { until }, clock.now()).unwrap();
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
        assert_eq!(
            recorded.dedupe_key,
            format!("reminder-due:{}", reminder.id)
        );

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
        assert_eq!(saved.failure.as_deref(), Some("toast recusado pelo sistema"));

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
            .queue_delivery(created.id, Channel::InApp, "reminder-due", VisualLevel::Normal)
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
            .create_at("X", "", clock.now() + Duration::hours(1), None, ReminderSource::User)
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
            .create_at("Ligar", "", clock.now() + Duration::hours(1), None, ReminderSource::User)
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

        assert_eq!(service.reminder(created.id).unwrap().status, ReminderStatus::Missed);
        assert_eq!(service.needs_attention_count().unwrap(), 1);
    }

    /// Sem isto, "Task atrasada" a cada acordada do laco.
    #[test]
    fn a_second_delivery_of_the_same_subject_is_blocked_while_the_first_lives() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at("X", "", clock.now() + Duration::minutes(1), None, ReminderSource::User)
            .unwrap();
        clock.advance(Duration::minutes(1));
        service.reconcile().unwrap();

        assert!(service
            .queue_delivery(created.id, Channel::InApp, "reminder-due", VisualLevel::Normal)
            .unwrap()
            .is_some());
        assert!(
            service
                .queue_delivery(created.id, Channel::InApp, "reminder-due", VisualLevel::Normal)
                .unwrap()
                .is_none(),
            "a segunda com a mesma chave e recusada"
        );

        // Assunto diferente NAO e bloqueado: "venceu" e "foi perdido" sao
        // avisos diferentes sobre o mesmo Reminder.
        assert!(service
            .queue_delivery(created.id, Channel::InApp, "reminder-missed", VisualLevel::Normal)
            .unwrap()
            .is_some());
    }

    /// A invariante central do sistema, exercitada onde ela pode falhar.
    #[test]
    fn a_failed_delivery_leaves_the_reminder_needing_attention() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at("X", "", clock.now() + Duration::minutes(1), None, ReminderSource::User)
            .unwrap();
        clock.advance(Duration::minutes(1));
        service.reconcile().unwrap();

        let queued = service
            .queue_delivery(created.id, Channel::Windows, "reminder-due", VisualLevel::Normal)
            .unwrap()
            .unwrap();
        service.mark_failed(&queued, "toast recusado").unwrap();

        let after = service.reminder(created.id).unwrap();
        assert_eq!(after.status, ReminderStatus::Due, "falha nao resolve nada");
        assert!(after.status.needs_attention());
        assert_eq!(service.needs_attention_count().unwrap(), 1);

        // E como a entrega morreu, o dedupe libera a proxima tentativa.
        assert!(service
            .queue_delivery(created.id, Channel::Windows, "reminder-due", VisualLevel::Normal)
            .unwrap()
            .is_some());
    }

    #[test]
    fn snoozing_takes_it_out_of_attention_and_puts_it_back_later() {
        let (service, clock, _guard) = service();
        let created = service
            .create_at("X", "", clock.now() + Duration::minutes(1), None, ReminderSource::User)
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
            .create_at("X", "", clock.now() + Duration::hours(1), None, ReminderSource::User)
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
            .create_at("X", "", clock.now() + Duration::hours(1), None, ReminderSource::User)
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
            storage
                .create_reminder(new_reminder(&clock, 1))
                .unwrap()
                .id
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

        let missed =
            mos_core::apply(&waiting[0], Transition::Miss, clock.now()).unwrap();
        let saved = storage.save_reminder(&missed).unwrap();
        assert_eq!(saved.status, ReminderStatus::Missed);
        assert_eq!(
            saved.overdue_by(clock.now()),
            Some(Duration::hours(29)),
            "o atraso e contado do vencimento original"
        );
    }
}
