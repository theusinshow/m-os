//! Persistencia do que o sistema observa (ADR-032).
//!
//! Guarda evento e programa monitorado. Nao cria sessao, e isso e a regra:
//! observacao nao vira hora sozinha. A Linha do Tempo mostra o vao, e quem
//! decide se aquilo foi trabalho e a pessoa.

use mos_core::{
    ActivityEvent, ActivityEventId, ActivityKind, CoreError, ErrorCode, MonitoredApp,
    MonitoringRepository, NewActivityEvent,
};
use rusqlite::params;
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time},
    SqliteStorage,
};

impl MonitoringRepository for SqliteStorage {
    fn monitored_apps(&self) -> Result<Vec<MonitoredApp>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT id, display_name, process_name, enabled, remind_on_open, \
                 remind_on_close FROM monitored_apps ORDER BY display_name",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(MonitoredApp {
                    id: row.get(0)?,
                    display_name: row.get(1)?,
                    process_name: row.get(2)?,
                    enabled: row.get::<_, i64>(3)? != 0,
                    remind_on_open: row.get::<_, i64>(4)? != 0,
                    remind_on_close: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(map_sql_error)?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_sql_error)
    }

    fn save_monitored_app(&self, app: MonitoredApp) -> Result<MonitoredApp, CoreError> {
        let process = app.process_name.trim().to_lowercase();
        if process.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O nome do processo nao pode estar vazio.",
                false,
            ));
        }

        let connection = self.connection.lock().map_err(map_lock_error)?;
        let now = format_time(OffsetDateTime::now_utc())?;
        // Casa por `process_name` e nao por `id`: e o processo que o
        // monitoramento observa, e cadastrar `acad.exe` duas vezes com ids
        // diferentes geraria dois lembretes para a mesma abertura.
        connection
            .execute(
                "INSERT INTO monitored_apps (id, display_name, process_name, enabled, \
                 remind_on_open, remind_on_close, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7) \
                 ON CONFLICT (process_name) DO UPDATE SET \
                 display_name = excluded.display_name, enabled = excluded.enabled, \
                 remind_on_open = excluded.remind_on_open, \
                 remind_on_close = excluded.remind_on_close, updated_at = excluded.updated_at",
                params![
                    app.id,
                    app.display_name,
                    process,
                    i64::from(app.enabled),
                    i64::from(app.remind_on_open),
                    i64::from(app.remind_on_close),
                    now,
                ],
            )
            .map_err(map_sql_error)?;

        Ok(MonitoredApp {
            process_name: process,
            ..app
        })
    }

    fn delete_monitored_app(&self, id: &str) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute("DELETE FROM monitored_apps WHERE id = ?1", params![id])
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Programa monitorado nao encontrado.",
                false,
            ));
        }
        Ok(())
    }

    fn activity_events(
        &self,
        since: OffsetDateTime,
        until: OffsetDateTime,
    ) -> Result<Vec<ActivityEvent>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // Do mais antigo para o mais novo: a Linha do Tempo se le na ordem em
        // que o dia aconteceu, e nao ao contrario como um historico.
        let mut statement = connection
            .prepare(
                "SELECT id, event_type, process_name, detected_at, processed \
                 FROM activity_events WHERE detected_at >= ?1 AND detected_at <= ?2 \
                 ORDER BY detected_at",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![format_time(since)?, format_time(until)?], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut events = Vec::new();
        for row in rows {
            let (id, kind, process, detected, processed) = row.map_err(map_sql_error)?;
            events.push(ActivityEvent {
                id: ActivityEventId::parse(&id)?,
                kind: ActivityKind::parse(&kind)?,
                process_name: process.unwrap_or_default(),
                detected_at: parse_time(&detected)?,
                processed: processed != 0,
            });
        }
        Ok(events)
    }

    fn record_activity(&self, event: NewActivityEvent) -> Result<ActivityEvent, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let id = ActivityEventId::new();
        let detected = format_time(event.detected_at)?;
        connection
            .execute(
                "INSERT INTO activity_events (id, event_type, process_name, detected_at, \
                 processed, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)",
                params![
                    id.to_string(),
                    event.kind.as_str(),
                    (!event.process_name.is_empty()).then_some(&event.process_name),
                    detected,
                    format_time(OffsetDateTime::now_utc())?,
                ],
            )
            .map_err(map_sql_error)?;

        Ok(ActivityEvent {
            id,
            kind: event.kind,
            process_name: event.process_name,
            detected_at: event.detected_at,
            processed: false,
        })
    }

    fn mark_activity_processed(&self, id: ActivityEventId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE activity_events SET processed = 1 WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    fn monitoring_settings(&self) -> Result<mos_core::MonitoringSettings, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .query_row(
                "SELECT process_monitoring_enabled, process_check_interval_seconds, \
                 idle_detection_enabled, idle_threshold_minutes, remind_on_monitored_open, \
                 remind_on_monitored_close FROM tracking_settings WHERE id = 1",
                [],
                |row| {
                    Ok(mos_core::MonitoringSettings {
                        process_monitoring_enabled: row.get::<_, i64>(0)? != 0,
                        check_interval_seconds: row.get(1)?,
                        idle_detection_enabled: row.get::<_, i64>(2)? != 0,
                        idle_threshold_minutes: row.get(3)?,
                        remind_on_open: row.get::<_, i64>(4)? != 0,
                        remind_on_close: row.get::<_, i64>(5)? != 0,
                    })
                },
            )
            .map_err(map_sql_error)
    }

    fn set_monitoring_settings(
        &self,
        settings: mos_core::MonitoringSettings,
    ) -> Result<mos_core::MonitoringSettings, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // Intervalo minimo de um segundo: zero faria o laco girar sem pausa e
        // comer um nucleo inteiro para observar um AutoCAD que nao mudou.
        let interval = settings.check_interval_seconds.max(1);
        connection
            .execute(
                "UPDATE tracking_settings SET process_monitoring_enabled = ?1, \
                 process_check_interval_seconds = ?2, idle_detection_enabled = ?3, \
                 idle_threshold_minutes = ?4, remind_on_monitored_open = ?5, \
                 remind_on_monitored_close = ?6 WHERE id = 1",
                params![
                    i64::from(settings.process_monitoring_enabled),
                    interval,
                    i64::from(settings.idle_detection_enabled),
                    settings.idle_threshold_minutes.max(1),
                    i64::from(settings.remind_on_open),
                    i64::from(settings.remind_on_close),
                ],
            )
            .map_err(map_sql_error)?;
        Ok(mos_core::MonitoringSettings {
            check_interval_seconds: interval,
            ..settings
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage() -> (SqliteStorage, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (storage, directory)
    }

    /// A migration semeia cinco sugestoes. Elas existem para o usuario
    /// reconhecer o proprio ferramental, nao para afirmar que ele tem os cinco.
    #[test]
    fn the_suggested_programs_are_there_from_the_start() {
        let (storage, _guard) = storage();
        let apps = storage.monitored_apps().unwrap();

        assert_eq!(apps.len(), 5);
        assert!(apps.iter().any(|app| app.process_name == "acad.exe"));
        assert!(apps.iter().all(|app| app.enabled));
    }

    /// Cadastrar `acad.exe` de novo nao pode criar um segundo: o monitoramento
    /// casa por processo, e dois cadastros dariam dois lembretes para a mesma
    /// abertura.
    #[test]
    fn saving_the_same_process_updates_instead_of_duplicating() {
        let (storage, _guard) = storage();
        storage
            .save_monitored_app(MonitoredApp {
                id: "outro-id".into(),
                display_name: "AutoCAD 2027".into(),
                process_name: "ACAD.EXE".into(),
                enabled: false,
                remind_on_open: false,
                remind_on_close: true,
            })
            .unwrap();

        let apps = storage.monitored_apps().unwrap();
        assert_eq!(apps.len(), 5, "nao duplicou");
        let acad = apps
            .iter()
            .find(|app| app.process_name == "acad.exe")
            .unwrap();
        assert_eq!(acad.display_name, "AutoCAD 2027");
        assert!(!acad.enabled);
    }

    /// O nome do processo e normalizado para minusculas: o Windows nao
    /// diferencia, e `ACAD.EXE` cadastrado ao lado de `acad.exe` seria o mesmo
    /// programa monitorado duas vezes.
    #[test]
    fn the_process_name_is_normalised() {
        let (storage, _guard) = storage();
        let saved = storage
            .save_monitored_app(MonitoredApp {
                id: "novo".into(),
                display_name: "Meu CAD".into(),
                process_name: "  MeuCAD.EXE  ".into(),
                enabled: true,
                remind_on_open: true,
                remind_on_close: true,
            })
            .unwrap();
        assert_eq!(saved.process_name, "meucad.exe");
    }

    #[test]
    fn events_come_back_in_the_order_the_day_happened() {
        let (storage, _guard) = storage();
        let base = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();

        for (offset, kind) in [
            (60, ActivityKind::AppClosed),
            (0, ActivityKind::AppOpened),
            (30, ActivityKind::IdleStarted),
        ] {
            storage
                .record_activity(NewActivityEvent {
                    kind,
                    process_name: "acad.exe".into(),
                    detected_at: base + time::Duration::seconds(offset),
                })
                .unwrap();
        }

        let events = storage
            .activity_events(
                base - time::Duration::hours(1),
                base + time::Duration::hours(1),
            )
            .unwrap();
        let kinds: Vec<_> = events.iter().map(|event| event.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ActivityKind::AppOpened,
                ActivityKind::IdleStarted,
                ActivityKind::AppClosed
            ]
        );
    }

    /// A janela filtra de verdade: sem isso a Linha do Tempo de hoje mostraria
    /// o mes inteiro.
    #[test]
    fn events_outside_the_window_stay_out() {
        let (storage, _guard) = storage();
        let base = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        storage
            .record_activity(NewActivityEvent {
                kind: ActivityKind::AppOpened,
                process_name: "acad.exe".into(),
                detected_at: base,
            })
            .unwrap();

        let far = storage
            .activity_events(
                base + time::Duration::days(1),
                base + time::Duration::days(2),
            )
            .unwrap();
        assert!(far.is_empty());
    }

    #[test]
    fn a_processed_event_is_marked_so_the_period_is_not_offered_again() {
        let (storage, _guard) = storage();
        let base = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let event = storage
            .record_activity(NewActivityEvent {
                kind: ActivityKind::AppOpened,
                process_name: "acad.exe".into(),
                detected_at: base,
            })
            .unwrap();
        assert!(!event.processed);

        storage.mark_activity_processed(event.id).unwrap();
        let stored = storage
            .activity_events(
                base - time::Duration::hours(1),
                base + time::Duration::hours(1),
            )
            .unwrap();
        assert!(stored[0].processed);
    }
}
