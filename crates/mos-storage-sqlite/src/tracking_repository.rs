//! Persistencia do rastreio de tempo (ADR-032, etapa B).
//!
//! Le e escreve o tempo REAL. Nenhuma funcao daqui arredonda nem desconta
//! inatividade: isso e decisao de apresentacao, e um repositorio que a aplicasse
//! tornaria impossivel recuperar o que de fato aconteceu.

use mos_core::{
    ActivityType, CoreError, EntrySource, NewTimeEntry, ProjectId, ProjectTracking, Rounding,
    RoundingMode, TimeEntry, TimeEntryId, TimeTrackingRepository, TrackingSettings, TrackingStatus,
};
use rusqlite::{params, Row};

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time},
    SqliteStorage,
};

const ENTRY_COLUMNS: &str = "id, project_id, started_at, ended_at, duration_seconds, \
     idle_seconds, description, activity_type, billable, hourly_rate_snapshot_cents, \
     source, created_at, updated_at";

/// A linha crua, antes de virar dominio.
///
/// Existe como alias porque a tupla tem treze posicoes: sem nome, cada uso dela
/// numa assinatura viraria uma parede que ninguem le.
type RawEntry = (
    String,
    String,
    String,
    Option<String>,
    i64,
    i64,
    String,
    String,
    i64,
    i64,
    String,
    String,
    String,
);

fn read_entry(row: &Row<'_>) -> rusqlite::Result<RawEntry> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
    ))
}

fn build_entry(raw: RawEntry) -> Result<TimeEntry, CoreError> {
    let (
        id,
        project_id,
        started_at,
        ended_at,
        duration_seconds,
        idle_seconds,
        description,
        activity_type,
        billable,
        hourly_rate_snapshot_cents,
        source,
        created_at,
        updated_at,
    ) = raw;

    Ok(TimeEntry {
        id: TimeEntryId::parse(&id)?,
        project_id: ProjectId::parse(&project_id)?,
        started_at: parse_time(&started_at)?,
        ended_at: ended_at.as_deref().map(parse_time).transpose()?,
        duration_seconds,
        idle_seconds,
        description,
        activity_type: ActivityType::parse(&activity_type)?,
        billable: billable != 0,
        hourly_rate_snapshot_cents,
        source: EntrySource::parse(&source)?,
        created_at: parse_time(&created_at)?,
        updated_at: parse_time(&updated_at)?,
    })
}

impl TimeTrackingRepository for SqliteStorage {
    fn create_time_entry(&self, entry: NewTimeEntry) -> Result<TimeEntry, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let id = TimeEntryId::new();
        let now = format_time(time::OffsetDateTime::now_utc())?;
        let started = format_time(entry.started_at)?;
        let ended = entry.ended_at.map(format_time).transpose()?;

        connection
            .execute(
                "INSERT INTO time_entries (id, project_id, started_at, ended_at, \
                 duration_seconds, idle_seconds, description, activity_type, billable, \
                 hourly_rate_snapshot_cents, source, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
                params![
                    id.to_string(),
                    entry.project_id.to_string(),
                    started,
                    ended,
                    entry.duration_seconds.max(0),
                    entry.idle_seconds.max(0),
                    entry.description,
                    entry.activity_type.as_str(),
                    i64::from(entry.billable),
                    entry.hourly_rate_snapshot_cents,
                    entry.source.as_str(),
                    now,
                ],
            )
            .map_err(map_sql_error)?;

        let raw = connection
            .query_row(
                &format!("SELECT {ENTRY_COLUMNS} FROM time_entries WHERE id = ?1"),
                params![id.to_string()],
                read_entry,
            )
            .map_err(map_sql_error)?;
        build_entry(raw)
    }

    fn time_entries(&self, project_id: Option<ProjectId>) -> Result<Vec<TimeEntry>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // Mais recente primeiro: a pergunta que se faz de um historico de horas
        // e quase sempre "o que eu fiz ultimamente".
        let sql = format!(
            "SELECT {ENTRY_COLUMNS} FROM time_entries \
             WHERE deleted_at IS NULL AND (?1 IS NULL OR project_id = ?1) \
             ORDER BY started_at DESC"
        );
        let mut statement = connection.prepare(&sql).map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![project_id.map(|id| id.to_string())], read_entry)
            .map_err(map_sql_error)?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(build_entry(row.map_err(map_sql_error)?)?);
        }
        Ok(entries)
    }

    fn trash_time_entry(&self, id: TimeEntryId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let now = format_time(time::OffsetDateTime::now_utc())?;
        let changed = connection
            .execute(
                "UPDATE time_entries SET deleted_at = ?2, updated_at = ?2 \
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![id.to_string(), now],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                mos_core::ErrorCode::NotFound,
                "Sessao nao encontrada.",
                false,
            ));
        }
        Ok(())
    }

    fn set_project_tracking(
        &self,
        tracking: ProjectTracking,
    ) -> Result<ProjectTracking, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let now = format_time(time::OffsetDateTime::now_utc())?;
        // `created_at` sobrevive ao upsert: quando esta linha nasceu e um fato, e
        // reescreve-lo a cada edicao de valor/hora apagaria desde quando aquele
        // Project e cobrado.
        connection
            .execute(
                "INSERT INTO project_tracking (project_id, hourly_rate_cents, code, color, \
                 tracking_status, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) \
                 ON CONFLICT (project_id) DO UPDATE SET \
                 hourly_rate_cents = excluded.hourly_rate_cents, code = excluded.code, \
                 color = excluded.color, tracking_status = excluded.tracking_status, \
                 updated_at = excluded.updated_at",
                params![
                    tracking.project_id.to_string(),
                    tracking.hourly_rate_cents,
                    tracking.code,
                    tracking.color,
                    tracking.tracking_status.as_str(),
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        Ok(tracking)
    }

    fn project_tracking(&self) -> Result<Vec<ProjectTracking>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT project_id, hourly_rate_cents, code, color, tracking_status \
                 FROM project_tracking",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut tracking = Vec::new();
        for row in rows {
            let (project_id, hourly_rate_cents, code, color, status) =
                row.map_err(map_sql_error)?;
            tracking.push(ProjectTracking {
                project_id: ProjectId::parse(&project_id)?,
                hourly_rate_cents,
                code: code.unwrap_or_default(),
                color: color.unwrap_or_default(),
                tracking_status: TrackingStatus::parse(&status)?,
            });
        }
        Ok(tracking)
    }

    fn tracking_settings(&self) -> Result<TrackingSettings, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let (enabled, interval, mode, idle) = connection
            .query_row(
                "SELECT rounding_enabled, rounding_interval_minutes, rounding_mode, \
                 idle_threshold_minutes FROM tracking_settings WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .map_err(map_sql_error)?;

        Ok(TrackingSettings {
            rounding: Rounding {
                enabled: enabled != 0,
                interval_minutes: interval,
                mode: RoundingMode::parse(&mode)?,
            },
            idle_threshold_minutes: idle,
        })
    }

    fn set_tracking_settings(
        &self,
        settings: TrackingSettings,
    ) -> Result<TrackingSettings, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE tracking_settings SET rounding_enabled = ?1, \
                 rounding_interval_minutes = ?2, rounding_mode = ?3, \
                 idle_threshold_minutes = ?4 WHERE id = 1",
                params![
                    i64::from(settings.rounding.enabled),
                    settings.rounding.interval_minutes,
                    settings.rounding.mode.as_str(),
                    settings.idle_threshold_minutes,
                ],
            )
            .map_err(map_sql_error)?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use mos_core::{NewProject, WorkRepository};

    use super::*;

    fn temporary_storage() -> (SqliteStorage, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (storage, directory)
    }

    fn project(storage: &SqliteStorage) -> ProjectId {
        storage
            .create_project(NewProject::create("Rancho Queimado", "", "").unwrap())
            .unwrap()
            .id
    }

    fn entry(project_id: ProjectId, duration: i64, rate: i64) -> NewTimeEntry {
        NewTimeEntry {
            project_id,
            started_at: time::OffsetDateTime::now_utc(),
            ended_at: None,
            duration_seconds: duration,
            idle_seconds: 0,
            description: String::new(),
            activity_type: ActivityType::Drawing,
            billable: true,
            hourly_rate_snapshot_cents: rate,
            source: EntrySource::Timer,
        }
    }

    #[test]
    fn a_time_entry_survives_the_round_trip() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);

        let created = storage.create_time_entry(entry(id, 3_600, 3_000)).unwrap();
        assert_eq!(created.duration_seconds, 3_600);
        assert_eq!(created.activity_type, ActivityType::Drawing);

        let listed = storage.time_entries(Some(id)).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);
        assert_eq!(listed[0].hourly_rate_snapshot_cents, 3_000);
    }

    /// Hora de trabalho e registro de cobranca: sai da vista sem sair do banco.
    #[test]
    fn trashing_an_entry_hides_it_without_erasing_it() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        let created = storage.create_time_entry(entry(id, 600, 3_000)).unwrap();

        storage.trash_time_entry(created.id).unwrap();
        assert!(storage.time_entries(None).unwrap().is_empty());

        let connection = storage.connection.lock().unwrap();
        let remaining: i64 = connection
            .query_row("SELECT count(*) FROM time_entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining, 1);
    }

    #[test]
    fn trashing_the_same_entry_twice_reports_not_found() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        let created = storage.create_time_entry(entry(id, 600, 3_000)).unwrap();

        storage.trash_time_entry(created.id).unwrap();
        assert!(storage.trash_time_entry(created.id).is_err());
    }

    /// O upsert preserva `created_at`: reescreve-lo a cada edicao de valor/hora
    /// apagaria desde quando o Project e cobrado.
    #[test]
    fn updating_tracking_keeps_the_original_created_at() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);

        let first = ProjectTracking {
            project_id: id,
            hourly_rate_cents: 3_000,
            code: "043".into(),
            color: String::new(),
            tracking_status: TrackingStatus::Active,
        };
        storage.set_project_tracking(first.clone()).unwrap();
        let born: String = {
            let connection = storage.connection.lock().unwrap();
            connection
                .query_row("SELECT created_at FROM project_tracking", [], |row| {
                    row.get(0)
                })
                .unwrap()
        };

        storage
            .set_project_tracking(ProjectTracking {
                hourly_rate_cents: 5_000,
                tracking_status: TrackingStatus::Completed,
                ..first
            })
            .unwrap();

        let listed = storage.project_tracking().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].hourly_rate_cents, 5_000);
        assert_eq!(listed[0].tracking_status, TrackingStatus::Completed);
        assert_eq!(listed[0].code, "043");

        let connection = storage.connection.lock().unwrap();
        let still: String = connection
            .query_row("SELECT created_at FROM project_tracking", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(still, born);
    }

    #[test]
    fn tracking_settings_start_with_rounding_off() {
        let (storage, _guard) = temporary_storage();
        let settings = storage.tracking_settings().unwrap();

        assert!(!settings.rounding.enabled);
        assert_eq!(settings.rounding.interval_minutes, 15);
        assert_eq!(settings.rounding.mode, RoundingMode::Nearest);
        assert_eq!(settings.idle_threshold_minutes, 10);
    }

    #[test]
    fn tracking_settings_round_trip() {
        let (storage, _guard) = temporary_storage();
        storage
            .set_tracking_settings(TrackingSettings {
                rounding: Rounding {
                    enabled: true,
                    interval_minutes: 30,
                    mode: RoundingMode::Up,
                },
                idle_threshold_minutes: 5,
            })
            .unwrap();

        let settings = storage.tracking_settings().unwrap();
        assert!(settings.rounding.enabled);
        assert_eq!(settings.rounding.interval_minutes, 30);
        assert_eq!(settings.rounding.mode, RoundingMode::Up);
        assert_eq!(settings.idle_threshold_minutes, 5);
    }
}
