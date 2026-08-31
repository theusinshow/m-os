//! Persistencia do rastreio de tempo (ADR-032, etapa B).
//!
//! Le e escreve o tempo REAL. Nenhuma funcao daqui arredonda nem desconta
//! inatividade: isso e decisao de apresentacao, e um repositorio que a aplicasse
//! tornaria impossivel recuperar o que de fato aconteceu.

use mos_core::{
    ActiveTimer, ActivityType, Client, ClientId, ClientInput, CoreError, EntrySource, ErrorCode,
    NewTimeEntry, ProjectId, ProjectTracking, Rounding, RoundingMode, StartTimer, TimeEntry,
    TimeEntryEdit, TimeEntryId, TimeTrackingRepository, TimerStatus, TrackingSettings,
    TrackingStatus,
};
use rusqlite::{params, OptionalExtension, Row};

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
        // `escrita` e nao `connection.lock`: esta escrita emite, e emitir toca o
        // relogio. Pegar os dois cadeados a mao aqui e o abraco mortal que o
        // portao existe para tornar impossivel — ver `SqliteStorage::portao`.
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let id = TimeEntryId::new();
        let now = format_time(time::OffsetDateTime::now_utc())?;
        let started = format_time(entry.started_at)?;
        let ended = entry.ended_at.map(format_time).transpose()?;

        transaction
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

        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("time_entry", id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    (
                        "projectId".to_owned(),
                        serde_json::json!(entry.project_id.to_string()),
                    ),
                    ("startedAt".to_owned(), serde_json::json!(started)),
                    ("endedAt".to_owned(), serde_json::json!(ended)),
                    (
                        "durationSeconds".to_owned(),
                        serde_json::json!(entry.duration_seconds.max(0)),
                    ),
                    (
                        "idleSeconds".to_owned(),
                        serde_json::json!(entry.idle_seconds.max(0)),
                    ),
                    (
                        "description".to_owned(),
                        serde_json::json!(entry.description),
                    ),
                    (
                        "activityType".to_owned(),
                        serde_json::json!(entry.activity_type.as_str()),
                    ),
                    ("billable".to_owned(), serde_json::json!(entry.billable)),
                    (
                        "hourlyRateSnapshotCents".to_owned(),
                        serde_json::json!(entry.hourly_rate_snapshot_cents),
                    ),
                    ("source".to_owned(), serde_json::json!(entry.source.as_str())),
                    ("createdAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;

        let raw = transaction
            .query_row(
                &format!("SELECT {ENTRY_COLUMNS} FROM time_entries WHERE id = ?1"),
                params![id.to_string()],
                read_entry,
            )
            .map_err(map_sql_error)?;
        let construida = build_entry(raw)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(construida)
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

    fn trashed_time_entries(&self) -> Result<Vec<TimeEntry>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // Pela remocao e nao pelo inicio: quem abre a lixeira esta procurando o
        // que acabou de apagar, e nao a sessao mais antiga que ja apagou.
        let sql = format!(
            "SELECT {ENTRY_COLUMNS} FROM time_entries \
             WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC"
        );
        let mut statement = connection.prepare(&sql).map_err(map_sql_error)?;
        let rows = statement.query_map([], read_entry).map_err(map_sql_error)?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(build_entry(row.map_err(map_sql_error)?)?);
        }
        Ok(entries)
    }

    fn update_time_entry(
        &self,
        id: TimeEntryId,
        edit: TimeEntryEdit,
    ) -> Result<TimeEntry, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // `ended_at` deriva do inicio mais a duracao em vez de ser editado
        // separado: dois campos que precisam concordar acabam discordando, e o
        // que o usuario corrige e "quanto tempo durou", nao "quando terminou".
        let ended = edit.started_at + time::Duration::seconds(edit.duration_seconds.max(0));
        let changed = connection
            .execute(
                "UPDATE time_entries SET started_at = ?2, ended_at = ?3, \
                 duration_seconds = ?4, idle_seconds = ?5, description = ?6, \
                 activity_type = ?7, billable = ?8, updated_at = ?9 \
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![
                    id.to_string(),
                    format_time(edit.started_at)?,
                    format_time(ended)?,
                    edit.duration_seconds.max(0),
                    edit.idle_seconds.max(0),
                    edit.description,
                    edit.activity_type.as_str(),
                    i64::from(edit.billable),
                    format_time(time::OffsetDateTime::now_utc())?,
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Sessao nao encontrada.",
                false,
            ));
        }

        let raw = connection
            .query_row(
                &format!("SELECT {ENTRY_COLUMNS} FROM time_entries WHERE id = ?1"),
                params![id.to_string()],
                read_entry,
            )
            .map_err(map_sql_error)?;
        build_entry(raw)
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

    fn restore_time_entry(&self, id: TimeEntryId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE time_entries SET deleted_at = NULL, updated_at = ?2 \
                 WHERE id = ?1 AND deleted_at IS NOT NULL",
                params![
                    id.to_string(),
                    format_time(time::OffsetDateTime::now_utc())?
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Sessao nao encontrada na lixeira.",
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
                 tracking_status, client_id, budget_minutes, paid_at, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9) \
                 ON CONFLICT (project_id) DO UPDATE SET \
                 hourly_rate_cents = excluded.hourly_rate_cents, code = excluded.code, \
                 color = excluded.color, tracking_status = excluded.tracking_status, \
                 client_id = excluded.client_id, budget_minutes = excluded.budget_minutes, \
                 paid_at = excluded.paid_at, updated_at = excluded.updated_at",
                params![
                    tracking.project_id.to_string(),
                    tracking.hourly_rate_cents,
                    tracking.code,
                    tracking.color,
                    tracking.tracking_status.as_str(),
                    tracking.client_id.map(|id| id.to_string()),
                    tracking.budget_minutes,
                    tracking.paid_at,
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
                "SELECT project_id, hourly_rate_cents, code, color, tracking_status, client_id, \
                 budget_minutes, paid_at FROM project_tracking",
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
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut tracking = Vec::new();
        for row in rows {
            let (project_id, hourly_rate_cents, code, color, status, client, budget, paid_at) =
                row.map_err(map_sql_error)?;
            tracking.push(ProjectTracking {
                project_id: ProjectId::parse(&project_id)?,
                hourly_rate_cents,
                code: code.unwrap_or_default(),
                color: color.unwrap_or_default(),
                tracking_status: TrackingStatus::parse(&status)?,
                client_id: client.as_deref().map(ClientId::parse).transpose()?,
                budget_minutes: budget,
                paid_at,
            });
        }
        Ok(tracking)
    }

    fn active_timer(&self) -> Result<Option<ActiveTimer>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let found = connection
            .query_row(
                "SELECT project_id, started_at, last_resumed_at, accumulated_seconds, status, \
                 description, activity_type FROM active_timer WHERE singleton = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(map_sql_error)?;

        let Some((project, started, resumed, accumulated, status, description, activity)) = found
        else {
            return Ok(None);
        };

        Ok(Some(ActiveTimer {
            project_id: ProjectId::parse(&project)?,
            started_at: parse_time(&started)?,
            last_resumed_at: parse_time(&resumed)?,
            accumulated_seconds: accumulated,
            status: if status == "paused" {
                TimerStatus::Paused
            } else {
                TimerStatus::Running
            },
            description: description.unwrap_or_default(),
            activity_type: ActivityType::parse(&activity)?,
        }))
    }

    fn start_timer(&self, start: StartTimer) -> Result<ActiveTimer, CoreError> {
        // Recusa em vez de substituir: encerrar o anterior por conta
        // descartaria tempo que o usuario nao mandou descartar.
        if self.active_timer()?.is_some() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Ja existe um cronometro em curso. Encerre ou pause antes de comecar outro.",
                false,
            ));
        }

        let connection = self.connection.lock().map_err(map_lock_error)?;
        let now = time::OffsetDateTime::now_utc();
        let stamp = format_time(now)?;
        connection
            .execute(
                "INSERT INTO active_timer (id, singleton, project_id, started_at, \
                 last_resumed_at, accumulated_seconds, status, description, activity_type, \
                 created_at, updated_at) \
                 VALUES (?1, 1, ?2, ?3, ?3, 0, 'running', ?4, ?5, ?3, ?3)",
                params![
                    uuid::Uuid::now_v7().to_string(),
                    start.project_id.to_string(),
                    stamp,
                    start.description,
                    start.activity_type.as_str(),
                ],
            )
            .map_err(map_sql_error)?;

        Ok(ActiveTimer {
            project_id: start.project_id,
            started_at: now,
            last_resumed_at: now,
            accumulated_seconds: 0,
            status: TimerStatus::Running,
            description: start.description,
            activity_type: start.activity_type,
        })
    }

    fn set_timer_running(&self, running: bool) -> Result<ActiveTimer, CoreError> {
        let timer = self.active_timer()?.ok_or_else(|| {
            CoreError::new(ErrorCode::NotFound, "Nao ha cronometro em curso.", false)
        })?;
        let now = time::OffsetDateTime::now_utc();

        // Pausar consolida o que correu ate agora em `accumulated_seconds`;
        // retomar zera a marca de referencia. Sem consolidar, uma pausa perderia
        // o trecho entre o ultimo resume e ela.
        let (accumulated, resumed) = if running {
            (timer.accumulated_seconds, now)
        } else {
            (timer.elapsed(now), timer.last_resumed_at)
        };

        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE active_timer SET status = ?1, accumulated_seconds = ?2, \
                 last_resumed_at = ?3, updated_at = ?4 WHERE singleton = 1",
                params![
                    if running { "running" } else { "paused" },
                    accumulated,
                    format_time(resumed)?,
                    format_time(now)?,
                ],
            )
            .map_err(map_sql_error)?;

        Ok(ActiveTimer {
            accumulated_seconds: accumulated,
            last_resumed_at: resumed,
            status: if running {
                TimerStatus::Running
            } else {
                TimerStatus::Paused
            },
            ..timer
        })
    }

    fn stop_timer(&self) -> Result<TimeEntry, CoreError> {
        let timer = self.active_timer()?.ok_or_else(|| {
            CoreError::new(ErrorCode::NotFound, "Nao ha cronometro em curso.", false)
        })?;
        let now = time::OffsetDateTime::now_utc();
        let duration = timer.elapsed(now);

        let connection = self.connection.lock().map_err(map_lock_error)?;
        // Gravar a sessao e apagar o cronometro na MESMA transacao. Separados,
        // uma queda entre os dois deixaria a hora contada duas vezes ou nenhuma.
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        // A taxa vem do Project no momento do encerramento e vira snapshot da
        // sessao: reajustar depois nao reescreve o que ja foi trabalhado.
        let rate: i64 = transaction
            .query_row(
                "SELECT hourly_rate_cents FROM project_tracking WHERE project_id = ?1",
                params![timer.project_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .unwrap_or(0);

        let id = TimeEntryId::new();
        let stamp = format_time(now)?;
        transaction
            .execute(
                "INSERT INTO time_entries (id, project_id, started_at, ended_at, \
                 duration_seconds, idle_seconds, description, activity_type, billable, \
                 hourly_rate_snapshot_cents, source, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, 1, ?8, 'timer', ?4, ?4)",
                params![
                    id.to_string(),
                    timer.project_id.to_string(),
                    format_time(timer.started_at)?,
                    stamp,
                    duration,
                    timer.description,
                    timer.activity_type.as_str(),
                    rate,
                ],
            )
            .map_err(map_sql_error)?;
        transaction
            .execute("DELETE FROM active_timer WHERE singleton = 1", [])
            .map_err(map_sql_error)?;

        let raw = transaction
            .query_row(
                &format!("SELECT {ENTRY_COLUMNS} FROM time_entries WHERE id = ?1"),
                params![id.to_string()],
                read_entry,
            )
            .map_err(map_sql_error)?;
        transaction.commit().map_err(map_sql_error)?;
        build_entry(raw)
    }

    fn clients(&self, include_archived: bool) -> Result<Vec<Client>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT id, name, company_name, email, phone, notes, archived_at \
                 FROM clients WHERE (?1 OR archived_at IS NULL) ORDER BY name",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![include_archived], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut clients = Vec::new();
        for row in rows {
            let (id, name, company, email, phone, notes, archived) = row.map_err(map_sql_error)?;
            clients.push(Client {
                id: ClientId::parse(&id)?,
                name,
                company_name: company.unwrap_or_default(),
                email: email.unwrap_or_default(),
                phone: phone.unwrap_or_default(),
                notes: notes.unwrap_or_default(),
                archived: archived.is_some(),
            });
        }
        Ok(clients)
    }

    fn create_client(&self, input: ClientInput) -> Result<Client, CoreError> {
        let name = input.validated()?.to_owned();
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let id = ClientId::new();
        let now = format_time(time::OffsetDateTime::now_utc())?;
        connection
            .execute(
                "INSERT INTO clients (id, name, company_name, email, phone, notes, \
                 created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    id.to_string(),
                    name,
                    input.company_name,
                    input.email,
                    input.phone,
                    input.notes,
                    now,
                ],
            )
            .map_err(map_sql_error)?;

        Ok(Client {
            id,
            name,
            company_name: input.company_name,
            email: input.email,
            phone: input.phone,
            notes: input.notes,
            archived: false,
        })
    }

    fn update_client(&self, id: ClientId, input: ClientInput) -> Result<Client, CoreError> {
        let name = input.validated()?.to_owned();
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE clients SET name = ?2, company_name = ?3, email = ?4, phone = ?5, \
                 notes = ?6, updated_at = ?7 WHERE id = ?1",
                params![
                    id.to_string(),
                    name,
                    input.company_name,
                    input.email,
                    input.phone,
                    input.notes,
                    format_time(time::OffsetDateTime::now_utc())?,
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Cliente nao encontrado.",
                false,
            ));
        }
        drop(connection);
        self.clients(true)?
            .into_iter()
            .find(|client| client.id == id)
            .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Cliente nao encontrado.", false))
    }

    fn set_client_archived(&self, id: ClientId, archived: bool) -> Result<Client, CoreError> {
        {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            let stamp = if archived {
                Some(format_time(time::OffsetDateTime::now_utc())?)
            } else {
                None
            };
            let changed = connection
                .execute(
                    "UPDATE clients SET archived_at = ?2, updated_at = ?3 WHERE id = ?1",
                    params![
                        id.to_string(),
                        stamp,
                        format_time(time::OffsetDateTime::now_utc())?
                    ],
                )
                .map_err(map_sql_error)?;
            if changed == 0 {
                return Err(CoreError::new(
                    ErrorCode::NotFound,
                    "Cliente nao encontrado.",
                    false,
                ));
            }
        }
        self.clients(true)?
            .into_iter()
            .find(|client| client.id == id)
            .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Cliente nao encontrado.", false))
    }

    fn discard_timer(&self) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute("DELETE FROM active_timer WHERE singleton = 1", [])
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Nao ha cronometro em curso.",
                false,
            ));
        }
        Ok(())
    }

    fn tracking_settings(&self) -> Result<TrackingSettings, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // `idle_threshold_minutes` NAO entra: ele pertence a
        // `MonitoringSettings`. Duas structs escrevendo a mesma coluna fariam
        // salvar uma desfazer a outra, e o usuario veria a configuracao voltar
        // sozinha sem entender por que.
        let (enabled, interval, mode) = connection
            .query_row(
                "SELECT rounding_enabled, rounding_interval_minutes, rounding_mode \
                 FROM tracking_settings WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
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
                 rounding_interval_minutes = ?2, rounding_mode = ?3 WHERE id = 1",
                params![
                    i64::from(settings.rounding.enabled),
                    settings.rounding.interval_minutes,
                    settings.rounding.mode.as_str(),
                ],
            )
            .map_err(map_sql_error)?;
        Ok(settings)
    }

    fn issuer(&self) -> Result<mos_core::Issuer, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .query_row(
                "SELECT issuer_name, issuer_document, issuer_contact \
                 FROM tracking_settings WHERE id = 1",
                [],
                |row| {
                    Ok(mos_core::Issuer {
                        name: row.get(0)?,
                        document: row.get(1)?,
                        contact: row.get(2)?,
                    })
                },
            )
            .map_err(map_sql_error)
    }

    fn set_issuer(&self, issuer: mos_core::Issuer) -> Result<mos_core::Issuer, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE tracking_settings SET issuer_name = ?1, issuer_document = ?2, \
                 issuer_contact = ?3 WHERE id = 1",
                params![issuer.name, issuer.document, issuer.contact],
            )
            .map_err(map_sql_error)?;
        Ok(issuer)
    }
}

#[cfg(test)]
mod tests {
    use mos_core::{MonitoringRepository, NewProject, WorkRepository};
    use mos_sync::DeviceRepository;

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

    /// Os tipos que estao na fila de sincronizacao agora.
    ///
    /// Le o `sync_outbox` direto porque a pergunta do teste e "a escrita
    /// emitiu?", e nenhuma API de dominio responde isso — `quantidade_pendente`
    /// devolve um numero, e um numero nao diz QUAL tipo foi esquecido.
    fn kinds_na_fila(storage: &SqliteStorage) -> Vec<String> {
        let connection = storage.connection.lock().unwrap();
        let mut consulta = connection
            .prepare("SELECT DISTINCT entity_kind FROM sync_outbox ORDER BY entity_kind")
            .unwrap();
        let linhas = consulta
            .query_map([], |linha| linha.get::<_, String>(0))
            .unwrap();
        linhas.map(|linha| linha.unwrap()).collect()
    }

    /// Um storage com a emissao LIGADA.
    ///
    /// Sem `habilitar_sync` o `sync` fica em `None` e nenhuma escrita emite —
    /// um teste de emissao montado sem isto passaria a mentir, porque a fila
    /// vazia teria duas explicacoes.
    fn storage_que_emite() -> (SqliteStorage, tempfile::TempDir) {
        let (storage, guard) = temporary_storage();
        let dispositivo = storage
            .este_dispositivo("teste", "windows", "0.0.0")
            .unwrap();
        storage.habilitar_sync(dispositivo.id).unwrap();
        (storage, guard)
    }

    #[test]
    fn registrar_horas_emite_operacao() {
        let (storage, _guard) = storage_que_emite();
        let id = project(&storage);

        storage.create_time_entry(entry(id, 3_600, 3_000)).unwrap();

        let kinds = kinds_na_fila(&storage);
        assert!(
            kinds.iter().any(|kind| kind == "time_entry"),
            "registrar horas nao emitiu operacao nenhuma; a fila tem {kinds:?}"
        );
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

    fn edit_of(duration: i64) -> TimeEntryEdit {
        TimeEntryEdit {
            started_at: time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
            duration_seconds: duration,
            idle_seconds: 0,
            description: "corrigido".into(),
            activity_type: ActivityType::Revision,
            billable: false,
        }
    }

    /// Esquecer de encerrar o cronometro e o erro mais comum de quem rastreia
    /// tempo — corrigir a duracao precisa ser trivial.
    #[test]
    fn editing_an_entry_fixes_the_duration_and_derives_the_end() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        let created = storage.create_time_entry(entry(id, 36_000, 3_000)).unwrap();

        let fixed = storage
            .update_time_entry(created.id, edit_of(5_400))
            .unwrap();

        assert_eq!(fixed.duration_seconds, 5_400);
        assert_eq!(fixed.activity_type, ActivityType::Revision);
        assert!(!fixed.billable);
        // O fim deriva do inicio mais a duracao: dois campos que precisam
        // concordar acabam discordando.
        assert_eq!(
            fixed.ended_at.unwrap().unix_timestamp() - fixed.started_at.unix_timestamp(),
            5_400
        );
    }

    /// A taxa e o registro do que valia quando o trabalho aconteceu. Uma
    /// correcao de duracao nao pode reprecificar o passado.
    #[test]
    fn editing_never_touches_the_rate_snapshot() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        let created = storage.create_time_entry(entry(id, 3_600, 9_000)).unwrap();

        let fixed = storage
            .update_time_entry(created.id, edit_of(1_800))
            .unwrap();
        assert_eq!(fixed.hourly_rate_snapshot_cents, 9_000);
    }

    #[test]
    fn editing_a_trashed_entry_reports_not_found() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        let created = storage.create_time_entry(entry(id, 600, 3_000)).unwrap();
        storage.trash_time_entry(created.id).unwrap();

        assert!(storage
            .update_time_entry(created.id, edit_of(1_200))
            .is_err());
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
            client_id: None,
            budget_minutes: 0,
            paid_at: None,
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

    fn start(project_id: ProjectId) -> StartTimer {
        StartTimer {
            project_id,
            description: "detalhe do patamar".into(),
            activity_type: ActivityType::Detailing,
        }
    }

    #[test]
    fn a_started_timer_is_running_from_zero() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);

        let timer = storage.start_timer(start(id)).unwrap();
        assert_eq!(timer.status, TimerStatus::Running);
        assert_eq!(timer.accumulated_seconds, 0);

        let found = storage.active_timer().unwrap().unwrap();
        assert_eq!(found.project_id, id);
        assert_eq!(found.activity_type, ActivityType::Detailing);
        assert_eq!(found.description, "detalhe do patamar");
    }

    /// Recusar em vez de substituir: encerrar o anterior por conta descartaria
    /// tempo que o usuario nao mandou descartar.
    #[test]
    fn a_second_timer_is_refused_instead_of_replacing_the_first() {
        let (storage, _guard) = temporary_storage();
        let first = project(&storage);
        storage.start_timer(start(first)).unwrap();

        let error = storage.start_timer(start(first)).unwrap_err();
        assert!(error.message.contains("cronometro em curso"));
        assert!(storage.active_timer().unwrap().is_some());
    }

    #[test]
    fn pausing_and_resuming_persist_the_state() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        storage.start_timer(start(id)).unwrap();

        let paused = storage.set_timer_running(false).unwrap();
        assert_eq!(paused.status, TimerStatus::Paused);
        assert_eq!(
            storage.active_timer().unwrap().unwrap().status,
            TimerStatus::Paused
        );

        let resumed = storage.set_timer_running(true).unwrap();
        assert_eq!(resumed.status, TimerStatus::Running);
        assert_eq!(
            storage.active_timer().unwrap().unwrap().status,
            TimerStatus::Running
        );
    }

    /// Encerrar grava a sessao E limpa o cronometro. Nada e descartado em
    /// silencio, e a taxa vem do Project no momento do encerramento.
    #[test]
    fn stopping_writes_the_session_and_clears_the_timer() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        storage
            .set_project_tracking(ProjectTracking {
                project_id: id,
                hourly_rate_cents: 3_000,
                code: String::new(),
                color: String::new(),
                tracking_status: TrackingStatus::Active,
                client_id: None,
                budget_minutes: 0,
                paid_at: None,
            })
            .unwrap();
        storage.start_timer(start(id)).unwrap();

        let entry = storage.stop_timer().unwrap();
        assert_eq!(entry.project_id, id);
        assert_eq!(entry.source, EntrySource::Timer);
        assert_eq!(entry.activity_type, ActivityType::Detailing);
        assert_eq!(entry.hourly_rate_snapshot_cents, 3_000);
        assert!(entry.ended_at.is_some());

        assert!(storage.active_timer().unwrap().is_none());
        assert_eq!(storage.time_entries(Some(id)).unwrap().len(), 1);
    }

    #[test]
    fn stopping_without_a_timer_reports_not_found() {
        let (storage, _guard) = temporary_storage();
        assert!(storage.stop_timer().is_err());
        assert!(storage.set_timer_running(false).is_err());
        assert!(storage.discard_timer().is_err());
    }

    /// Descartar e a UNICA operacao que joga tempo fora. Existe para quem
    /// iniciou no Project errado — e por isso nao grava sessao nenhuma.
    #[test]
    fn discarding_throws_the_timer_away_without_recording() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        storage.start_timer(start(id)).unwrap();

        storage.discard_timer().unwrap();

        assert!(storage.active_timer().unwrap().is_none());
        assert!(
            storage.time_entries(None).unwrap().is_empty(),
            "descartar nao pode gravar sessao"
        );
    }

    /// Sem tracking cadastrado a taxa e zero, e nao um erro: o Project pode ser
    /// pessoal e nao ter valor/hora nenhum.
    #[test]
    fn a_project_without_a_rate_records_zero() {
        let (storage, _guard) = temporary_storage();
        let id = project(&storage);
        storage.start_timer(start(id)).unwrap();

        assert_eq!(storage.stop_timer().unwrap().hourly_rate_snapshot_cents, 0);
    }

    #[test]
    fn tracking_settings_start_with_rounding_off() {
        let (storage, _guard) = temporary_storage();
        let settings = storage.tracking_settings().unwrap();

        assert!(!settings.rounding.enabled);
        assert_eq!(settings.rounding.interval_minutes, 15);
        assert_eq!(settings.rounding.mode, RoundingMode::Nearest);
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
            })
            .unwrap();

        let settings = storage.tracking_settings().unwrap();
        assert!(settings.rounding.enabled);
        assert_eq!(settings.rounding.interval_minutes, 30);
        assert_eq!(settings.rounding.mode, RoundingMode::Up);
    }

    /// O limiar de inatividade e do monitoramento, e a mesma coluna sustenta os
    /// dois tipos. Sem esta separacao, salvar o arredondamento zerava a
    /// configuracao de observacao — e o usuario veria a preferencia voltar
    /// sozinha sem entender por que.
    #[test]
    fn saving_the_rounding_leaves_the_observation_alone() {
        let (storage, _guard) = temporary_storage();
        storage
            .set_monitoring_settings(mos_core::MonitoringSettings {
                process_monitoring_enabled: false,
                check_interval_seconds: 20,
                idle_detection_enabled: true,
                idle_threshold_minutes: 7,
                remind_on_open: false,
                remind_on_close: true,
                meeting_detection_enabled: true,
            })
            .unwrap();

        storage
            .set_tracking_settings(TrackingSettings {
                rounding: Rounding {
                    enabled: true,
                    interval_minutes: 30,
                    mode: RoundingMode::Up,
                },
            })
            .unwrap();

        let watching = storage.monitoring_settings().unwrap();
        assert_eq!(watching.idle_threshold_minutes, 7);
        assert_eq!(watching.check_interval_seconds, 20);
        assert!(!watching.process_monitoring_enabled);
        assert!(!watching.remind_on_open);
    }

    /// Intervalo zero faria o laco girar sem pausa e comer um nucleo inteiro.
    #[test]
    fn the_check_interval_never_reaches_zero() {
        let (storage, _guard) = temporary_storage();
        let saved = storage
            .set_monitoring_settings(mos_core::MonitoringSettings {
                process_monitoring_enabled: true,
                check_interval_seconds: 0,
                idle_detection_enabled: true,
                idle_threshold_minutes: 0,
                remind_on_open: true,
                remind_on_close: true,
                meeting_detection_enabled: true,
            })
            .unwrap();

        assert_eq!(saved.check_interval_seconds, 1);
        assert_eq!(
            storage
                .monitoring_settings()
                .unwrap()
                .idle_threshold_minutes,
            1
        );
    }
}
