//! Importacao unica do banco do CronoCAD (ADR-032, etapa B).
//!
//! Le um `cronocad.sqlite` e traz projetos, sessoes e pendencias para o M/OS. E
//! um caminho de mao unica, feito para rodar uma vez: o CronoCAD continua
//! existindo com o proprio banco ate a etapa C, e nada aqui escreve nele.
//!
//! A fonte e aberta em `mode=ro`. Uma importacao que pudesse alterar a origem
//! transformaria "tentar de novo" em risco, e tentar de novo e exatamente o que
//! se faz quando algo dá errado no meio.

use std::path::Path;

use mos_core::{
    ActivityType, CoreError, EntrySource, ErrorCode, LifecycleState, NewProject, NewTask,
    NewTimeEntry, ProjectId, ProjectTracking, TaskState, TimeTrackingRepository, TrackingStatus,
    WorkRepository,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Serialize;

use crate::{map_sql_error, repository::parse_time, SqliteStorage};

/// O que a importacao trouxe. Devolvido para conferencia, e nao so para log: o
/// numero de horas e o que o usuario compara com a tela do CronoCAD para saber
/// se pode desinstalar.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub projects: usize,
    pub entries: usize,
    pub tasks: usize,
    /// Segundos brutos, somados como estavam na origem.
    pub tracked_seconds: i64,
    pub monitored_apps: usize,
    /// O historico observado pelo sistema. Sao centenas de linhas e nenhuma
    /// delas e digitavel de novo — e o que o computador viu acontecer.
    pub activity_events: usize,
    pub clients: usize,
}

/// Traducao do estado do CronoCAD para os dois eixos do M/OS.
///
/// `completed` e `archived` levam ao mesmo lifecycle porque o M/OS nao distingue
/// os dois — e e por isso que `tracking_status` guarda o original. Sem ele, uma
/// obra entregue e uma abandonada ficariam indistinguiveis.
fn map_status(raw: &str) -> (LifecycleState, TrackingStatus) {
    match raw {
        "completed" => (LifecycleState::Archived, TrackingStatus::Completed),
        "archived" => (LifecycleState::Archived, TrackingStatus::Archived),
        "paused" => (LifecycleState::Active, TrackingStatus::Paused),
        _ => (LifecycleState::Active, TrackingStatus::Active),
    }
}

impl SqliteStorage {
    /// Importa um banco do CronoCAD. Roda uma vez.
    ///
    /// A recusa olha para a MARCA de importacao, e nao para a existencia de
    /// sessao. A primeira versao recusava quando `time_entries` tinha qualquer
    /// linha, e isso quebrou no primeiro uso real: quem experimentou o
    /// cronometro antes de importar criou uma sessao e ficou impedido de
    /// importar para sempre. "Ja importei" e "tem dado" nao sao a mesma
    /// pergunta.
    pub fn import_cronocad(&self, source: &Path) -> Result<ImportReport, CoreError> {
        if self.cronocad_imported_at()?.is_some() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O CronoCAD ja foi importado neste M/OS. Importar de novo dobraria as horas \
                 de todo projeto, e o erro so apareceria na hora de faturar.",
                false,
            ));
        }

        let origin = Connection::open_with_flags(
            source,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(map_sql_error)?;

        // Clientes ANTES dos projetos: o projeto aponta para o cliente, e a
        // chave estrangeira recusaria um vinculo para uma linha que ainda nao
        // existe. Os ids da origem sao preservados, entao o vinculo atravessa
        // sem mapeamento.
        let mut report = ImportReport {
            clients: self.copy_clients(&origin)?,
            ..Default::default()
        };

        let mut projects = origin
            .prepare(
                "SELECT id, name, description, hourly_rate_cents, status, code, color, notes, \
                 client_id, budget_minutes FROM projects ORDER BY name",
            )
            .map_err(map_sql_error)?;
        let rows = projects
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, i64>(9)?,
                ))
            })
            .map_err(map_sql_error)?;

        for row in rows {
            let (origin_id, name, description, rate, status, code, color, notes, client, budget) =
                row.map_err(map_sql_error)?;

            // As anotacoes livres do projeto viram descricao do Project: e o
            // campo do M/OS com o mesmo papel, e perde-las seria perder contexto
            // que o usuario escreveu a mao.
            let description = [description.unwrap_or_default(), notes.unwrap_or_default()]
                .iter()
                .filter(|part| !part.trim().is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join("\n\n");

            let (lifecycle, tracking_status) = map_status(&status);
            let project = self.create_project(NewProject::create(&name, &description, "")?)?;

            self.set_project_tracking(ProjectTracking {
                project_id: project.id,
                hourly_rate_cents: rate,
                code: code.unwrap_or_default(),
                color: color.unwrap_or_default(),
                tracking_status,
                client_id: client
                    .as_deref()
                    .map(mos_core::ClientId::parse)
                    .transpose()?,
                budget_minutes: budget,
            })?;

            if lifecycle != LifecycleState::Active {
                self.set_project_lifecycle(project.id, lifecycle)?;
            }

            report.tracked_seconds +=
                self.copy_entries(&origin, &origin_id, project.id, &mut report)?;
            report.tasks += self.copy_todos(&origin, &origin_id, project.id)?;
            report.projects += 1;
        }

        // Preferencias ANTES da marca, e nunca depois das sessoes por acaso: o
        // arredondamento muda o numero cobravel, e importar as horas sem trazer
        // a configuracao faria o M/OS mostrar um valor diferente do que o
        // CronoCAD mostrava — divergencia silenciosa num numero que vira fatura.
        self.copy_preferences(&origin)?;
        report.monitored_apps = self.copy_monitored_apps(&origin)?;
        report.activity_events = self.copy_activity_events(&origin)?;

        self.mark_cronocad_imported()?;
        Ok(report)
    }

    /// Traz o arredondamento, a inatividade e o monitoramento.
    fn copy_preferences(&self, origin: &Connection) -> Result<(), CoreError> {
        let found = origin
            .query_row(
                "SELECT rounding_enabled, rounding_interval_minutes, rounding_mode, \
                 idle_threshold_minutes, idle_detection_enabled, process_monitoring_enabled, \
                 process_check_interval_seconds, remind_when_monitored_app_opens, \
                 remind_when_monitored_app_closes, issuer_name, issuer_document, \
                 issuer_contact FROM settings WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                    ))
                },
            )
            .optional()
            .map_err(map_sql_error)?;

        let Some((
            enabled,
            interval,
            mode,
            idle,
            idle_on,
            monitoring,
            check,
            on_open,
            on_close,
            issuer_name,
            issuer_document,
            issuer_contact,
        )) = found
        else {
            return Ok(());
        };

        let connection = self.connection.lock().map_err(crate::map_lock_error)?;
        connection
            .execute(
                "UPDATE tracking_settings SET rounding_enabled = ?1, \
                 rounding_interval_minutes = ?2, rounding_mode = ?3, \
                 idle_threshold_minutes = ?4, idle_detection_enabled = ?5, \
                 process_monitoring_enabled = ?6, process_check_interval_seconds = ?7, \
                 remind_on_monitored_open = ?8, remind_on_monitored_close = ?9, \
                 issuer_name = ?10, issuer_document = ?11, issuer_contact = ?12 WHERE id = 1",
                rusqlite::params![
                    enabled,
                    interval,
                    mode,
                    idle,
                    idle_on,
                    monitoring,
                    check,
                    on_open,
                    on_close,
                    issuer_name.unwrap_or_default(),
                    issuer_document.unwrap_or_default(),
                    issuer_contact.unwrap_or_default(),
                ],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Os programas monitorados, incluindo os que o usuario acrescentou.
    ///
    /// `ON CONFLICT` porque a migration ja semeou os cinco sugeridos: o que vem
    /// do CronoCAD e a verdade, e sobrescreve a semente em vez de duplicar.
    fn copy_monitored_apps(&self, origin: &Connection) -> Result<usize, CoreError> {
        let mut statement = origin
            .prepare(
                "SELECT id, display_name, process_name, enabled, remind_on_open, \
                 remind_on_close, created_at, updated_at FROM monitored_apps",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })
            .map_err(map_sql_error)?;

        let connection = self.connection.lock().map_err(crate::map_lock_error)?;
        let mut count = 0;
        for row in rows {
            let (id, name, process, enabled, on_open, on_close, created, updated) =
                row.map_err(map_sql_error)?;
            connection
                .execute(
                    "INSERT INTO monitored_apps (id, display_name, process_name, enabled, \
                     remind_on_open, remind_on_close, created_at, updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                     ON CONFLICT (process_name) DO UPDATE SET \
                     display_name = excluded.display_name, enabled = excluded.enabled, \
                     remind_on_open = excluded.remind_on_open, \
                     remind_on_close = excluded.remind_on_close, \
                     updated_at = excluded.updated_at",
                    rusqlite::params![
                        id, name, process, enabled, on_open, on_close, created, updated
                    ],
                )
                .map_err(map_sql_error)?;
            count += 1;
        }
        Ok(count)
    }

    /// O historico observado pelo sistema.
    ///
    /// Sao centenas de linhas e nenhuma delas e digitavel de novo: e o que o
    /// computador viu acontecer. Sem elas a Linha do Tempo Detectada nasce cega
    /// para tudo que ja passou.
    fn copy_activity_events(&self, origin: &Connection) -> Result<usize, CoreError> {
        let mut statement = origin
            .prepare(
                "SELECT id, event_type, process_name, detected_at, metadata_json, \
                 processed, created_at FROM activity_events ORDER BY detected_at",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(map_sql_error)?;

        let connection = self.connection.lock().map_err(crate::map_lock_error)?;
        let mut count = 0;
        for row in rows {
            let (id, kind, process, detected, metadata, processed, created) =
                row.map_err(map_sql_error)?;
            connection
                .execute(
                    "INSERT OR IGNORE INTO activity_events (id, event_type, process_name, \
                     detected_at, metadata_json, processed, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![id, kind, process, detected, metadata, processed, created],
                )
                .map_err(map_sql_error)?;
            count += 1;
        }
        Ok(count)
    }

    fn copy_clients(&self, origin: &Connection) -> Result<usize, CoreError> {
        let mut statement = origin
            .prepare(
                "SELECT id, name, company_name, email, phone, notes, created_at, \
                 updated_at, archived_at FROM clients",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            })
            .map_err(map_sql_error)?;

        let connection = self.connection.lock().map_err(crate::map_lock_error)?;
        let mut count = 0;
        for row in rows {
            let (id, name, company, email, phone, notes, created, updated, archived) =
                row.map_err(map_sql_error)?;
            connection
                .execute(
                    "INSERT OR IGNORE INTO clients (id, name, company_name, email, phone, \
                     notes, created_at, updated_at, archived_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        id, name, company, email, phone, notes, created, updated, archived
                    ],
                )
                .map_err(map_sql_error)?;
            count += 1;
        }
        Ok(count)
    }

    /// Quando o CronoCAD foi importado, se foi.
    pub fn cronocad_imported_at(&self) -> Result<Option<String>, CoreError> {
        let connection = self.connection.lock().map_err(crate::map_lock_error)?;
        connection
            .query_row(
                "SELECT cronocad_imported_at FROM tracking_settings WHERE id = 1",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(map_sql_error)
    }

    fn mark_cronocad_imported(&self) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(crate::map_lock_error)?;
        connection
            .execute(
                "UPDATE tracking_settings SET cronocad_imported_at = ?1 WHERE id = 1",
                [crate::repository::format_time(
                    time::OffsetDateTime::now_utc(),
                )?],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    fn copy_entries(
        &self,
        origin: &Connection,
        origin_id: &str,
        project_id: ProjectId,
        report: &mut ImportReport,
    ) -> Result<i64, CoreError> {
        let mut statement = origin
            .prepare(
                "SELECT started_at, ended_at, duration_seconds, idle_seconds, description, \
                 activity_type, billable, hourly_rate_snapshot_cents, source \
                 FROM time_entries WHERE project_id = ?1 AND deleted_at IS NULL \
                 ORDER BY started_at",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([origin_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut seconds = 0;
        for row in rows {
            let (started, ended, duration, idle, description, activity, billable, rate, source) =
                row.map_err(map_sql_error)?;

            self.create_time_entry(NewTimeEntry {
                project_id,
                started_at: parse_time(&started)?,
                ended_at: ended.as_deref().map(parse_time).transpose()?,
                duration_seconds: duration,
                idle_seconds: idle,
                description: description.unwrap_or_default(),
                activity_type: ActivityType::parse(&activity).unwrap_or_default(),
                billable: billable != 0,
                // A taxa vem da SESSAO, e nao do projeto: e o snapshot da epoca,
                // e reescreve-lo com o valor atual seria refaturar o passado.
                hourly_rate_snapshot_cents: rate,
                source: EntrySource::parse(&source).unwrap_or_default(),
            })?;

            seconds += duration.max(0);
            report.entries += 1;
        }
        Ok(seconds)
    }

    fn copy_todos(
        &self,
        origin: &Connection,
        origin_id: &str,
        project_id: ProjectId,
    ) -> Result<usize, CoreError> {
        let mut statement = origin
            .prepare(
                "SELECT text, done FROM project_todos WHERE project_id = ?1 ORDER BY created_at",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([origin_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(map_sql_error)?;

        let mut count = 0;
        for row in rows {
            let (text, done) = row.map_err(map_sql_error)?;
            // Pendencia do CronoCAD vira Task do M/OS em vez de uma segunda
            // lista de afazeres competindo com a que ja existe.
            let task = self.create_task(NewTask::create(&text, "", Some(project_id))?)?;
            if done != 0 {
                self.set_task_state(task.id, TaskState::Done)?;
            }
            count += 1;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use mos_core::{MonitoringRepository, Rounding, RoundingMode, TrackingSettings};

    use super::*;

    fn m_os() -> (SqliteStorage, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (storage, directory)
    }

    /// Reproduz o formato real do CronoCAD, incluindo o que a importacao precisa
    /// ignorar: sessao apagada e projeto concluido.
    fn cronocad(directory: &Path) -> std::path::PathBuf {
        let path = directory.join("cronocad.sqlite");
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE projects (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT,
                hourly_rate_cents INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL,
                code TEXT, color TEXT, notes TEXT, client_id TEXT,
                budget_minutes INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE time_entries (
                id TEXT PRIMARY KEY, project_id TEXT NOT NULL, started_at TEXT NOT NULL,
                ended_at TEXT, duration_seconds INTEGER NOT NULL, idle_seconds INTEGER NOT NULL,
                description TEXT, activity_type TEXT NOT NULL, billable INTEGER NOT NULL,
                hourly_rate_snapshot_cents INTEGER NOT NULL, source TEXT NOT NULL,
                deleted_at TEXT);
             CREATE TABLE project_todos (
                id TEXT PRIMARY KEY, project_id TEXT NOT NULL, text TEXT NOT NULL,
                done INTEGER NOT NULL, created_at TEXT NOT NULL);

             INSERT INTO projects VALUES
                ('p1','Rancho Queimado','obra',3000,'active','043',NULL,'lembrar do corrimao',NULL,2400),
                ('p2','Juliano - POA',NULL,5000,'completed',NULL,NULL,NULL,
                 '018f0000-0000-7000-8000-000000000001',0);

             INSERT INTO time_entries VALUES
                ('e1','p1','2026-07-12T05:27:34Z','2026-07-12T07:27:34Z',7200,600,'detalhe',
                 'drawing',1,3000,'timer',NULL),
                ('e2','p1','2026-07-13T05:00:00Z',NULL,3600,0,NULL,'revision',1,9000,
                 'reconstructed',NULL),
                ('e3','p1','2026-07-14T05:00:00Z',NULL,1800,0,NULL,'meeting',0,3000,'manual',
                 NULL),
                ('e4','p2','2026-08-11T01:33:44Z',NULL,3600,0,NULL,'drawing',1,5000,'timer',
                 NULL),
                ('e5','p1','2026-07-15T05:00:00Z',NULL,99999,0,NULL,'other',1,3000,'timer',
                 '2026-07-16T00:00:00Z');

             INSERT INTO project_todos VALUES
                ('t1','p1','Conferir cota do patamar',0,'2026-07-12T05:00:00Z'),
                ('t2','p1','Enviar PDF',1,'2026-07-12T05:00:00Z');

             CREATE TABLE settings (
                id INTEGER PRIMARY KEY, idle_detection_enabled INTEGER,
                idle_threshold_minutes INTEGER, process_monitoring_enabled INTEGER,
                process_check_interval_seconds INTEGER,
                remind_when_monitored_app_opens INTEGER,
                remind_when_monitored_app_closes INTEGER, rounding_enabled INTEGER,
                rounding_interval_minutes INTEGER, rounding_mode TEXT,
                issuer_name TEXT, issuer_document TEXT, issuer_contact TEXT);
             INSERT INTO settings VALUES (1, 1, 7, 1, 9, 0, 1, 1, 30, 'up',
                'Matheus Mendes', '000.000.000-00', 'contato@exemplo.com');

             CREATE TABLE monitored_apps (
                id TEXT PRIMARY KEY, display_name TEXT, process_name TEXT UNIQUE,
                enabled INTEGER, remind_on_open INTEGER, remind_on_close INTEGER,
                created_at TEXT, updated_at TEXT);
             INSERT INTO monitored_apps VALUES
                ('app-acad','AutoCAD','acad.exe',0,1,1,'1970-01-01T00:00:00Z','1970-01-01T00:00:00Z'),
                ('app-meu','Ferramenta minha','minha.exe',1,1,0,'1970-01-01T00:00:00Z','1970-01-01T00:00:00Z');

             CREATE TABLE activity_events (
                id TEXT PRIMARY KEY, event_type TEXT, process_name TEXT,
                detected_at TEXT, metadata_json TEXT, processed INTEGER, created_at TEXT);
             INSERT INTO activity_events VALUES
                ('v1','app_opened','acad.exe','2026-07-12T05:00:00Z',NULL,0,'2026-07-12T05:00:00Z'),
                ('v2','idle_started',NULL,'2026-07-12T06:00:00Z',NULL,1,'2026-07-12T06:00:00Z');

             CREATE TABLE clients (
                id TEXT PRIMARY KEY, name TEXT, company_name TEXT, email TEXT,
                phone TEXT, notes TEXT, created_at TEXT, updated_at TEXT, archived_at TEXT);
             INSERT INTO clients VALUES
                ('018f0000-0000-7000-8000-000000000001','Juliano','JS Engenharia',NULL,NULL,NULL,
                 '2026-07-01T00:00:00Z','2026-07-01T00:00:00Z',NULL);",
        )
        .unwrap();
        path
    }

    #[test]
    fn the_import_brings_projects_sessions_and_todos() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());

        let report = storage.import_cronocad(&source).unwrap();

        assert_eq!(report.projects, 2);
        // A sessao apagada na origem nao entra: quatro das cinco.
        assert_eq!(report.entries, 4);
        assert_eq!(report.tasks, 2);
        assert_eq!(report.tracked_seconds, 7_200 + 3_600 + 1_800 + 3_600);
    }

    /// O arredondamento MUDA o numero cobravel. Importar as horas sem trazer a
    /// configuracao faria o M/OS mostrar um valor diferente do que o CronoCAD
    /// mostrava — divergencia silenciosa num numero que vira fatura.
    #[test]
    fn the_rounding_configuration_comes_across() {
        let (storage, directory) = m_os();
        storage
            .import_cronocad(&cronocad(directory.path()))
            .unwrap();

        let settings = storage.tracking_settings().unwrap();
        assert!(settings.rounding.enabled);
        assert_eq!(settings.rounding.interval_minutes, 30);
        assert_eq!(settings.rounding.mode, mos_core::RoundingMode::Up);

        // A observacao veio junto, e vive noutra struct: o limiar decide quando
        // o sistema considera que voce parou, e o arredondamento decide quanto
        // disso e cobrado. Sao duas perguntas.
        let watching = storage.monitoring_settings().unwrap();
        assert_eq!(watching.idle_threshold_minutes, 7);
        assert_eq!(watching.check_interval_seconds, 9);
        assert!(watching.idle_detection_enabled);
        assert!(!watching.remind_on_open, "o usuario tinha desligado");
        assert!(watching.remind_on_close);
    }

    /// O historico observado nao e digitavel de novo: e o que o computador viu.
    #[test]
    fn the_observed_history_and_the_monitored_apps_come_across() {
        let (storage, directory) = m_os();
        let report = storage
            .import_cronocad(&cronocad(directory.path()))
            .unwrap();

        assert_eq!(report.activity_events, 2);
        assert_eq!(report.clients, 1);
        // Dois na origem, mas a migration ja semeou cinco: o do usuario entra e
        // o AutoCAD sobrescreve a semente em vez de duplicar.
        assert_eq!(report.monitored_apps, 2);

        let connection = storage.connection.lock().unwrap();
        let apps: i64 = connection
            .query_row("SELECT count(*) FROM monitored_apps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            apps, 6,
            "a ferramenta do usuario entrou sem duplicar o AutoCAD"
        );
        let acad_off: i64 = connection
            .query_row(
                "SELECT enabled FROM monitored_apps WHERE process_name = 'acad.exe'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(acad_off, 0, "a escolha do usuario venceu a semente");
    }

    /// A taxa e da SESSAO, e nao do projeto. `e2` foi trabalhada a 90,00/h
    /// enquanto o projeto vale 30,00/h — reescrever com o valor atual seria
    /// refaturar o passado.
    #[test]
    fn each_session_keeps_the_rate_it_was_worked_at() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());
        storage.import_cronocad(&source).unwrap();

        let rates: Vec<i64> = storage
            .time_entries(None)
            .unwrap()
            .iter()
            .map(|entry| entry.hourly_rate_snapshot_cents)
            .collect();
        assert!(rates.contains(&9_000), "o snapshot de 90,00/h se perdeu");
        assert!(rates.contains(&3_000));
        assert!(rates.contains(&5_000));
    }

    /// O M/OS nao tem "concluido", entao o projeto entregue vira arquivado — e o
    /// `tracking_status` guarda que ele TERMINOU, em vez de ter caido em desuso.
    #[test]
    fn a_completed_project_is_archived_but_remembers_it_finished() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());
        storage.import_cronocad(&source).unwrap();

        let finished = storage
            .projects(true)
            .unwrap()
            .into_iter()
            .find(|project| project.name == "Juliano - POA")
            .expect("projeto concluido nao foi importado");
        assert_eq!(finished.lifecycle_state, LifecycleState::Archived);

        let tracking = storage
            .project_tracking()
            .unwrap()
            .into_iter()
            .find(|entry| entry.project_id == finished.id)
            .unwrap();
        assert_eq!(tracking.tracking_status, TrackingStatus::Completed);
        assert_eq!(tracking.hourly_rate_cents, 5_000);
        // O vinculo com o cliente atravessa: os ids da origem sao preservados, e
        // por isso os clientes precisam entrar ANTES dos projetos.
        assert!(
            tracking.client_id.is_some(),
            "o cliente do projeto se perdeu"
        );
    }

    #[test]
    fn project_notes_survive_as_the_description() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());
        storage.import_cronocad(&source).unwrap();

        let project = storage
            .projects(false)
            .unwrap()
            .into_iter()
            .find(|project| project.name == "Rancho Queimado")
            .unwrap();
        assert!(project.description.contains("obra"));
        assert!(project.description.contains("corrimao"));
    }

    /// O emissor sai no cabecalho da fatura. Se ele nao atravessa, a primeira
    /// fatura emitida no M/OS sai sem dizer quem deve receber.
    #[test]
    fn the_invoice_issuer_survives() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());
        storage.import_cronocad(&source).unwrap();

        let issuer = storage.issuer().unwrap();
        assert_eq!(issuer.name, "Matheus Mendes");
        assert_eq!(issuer.document, "000.000.000-00");
        assert_eq!(issuer.contact, "contato@exemplo.com");
    }

    /// A meta de horas nao tinha coluna ate a 0014, e a ausencia nao doeu porque
    /// o banco real tinha todas em zero. Um banco com meta configurada teria
    /// perdido a meta em silencio — a leitura passaria, o numero sumiria.
    #[test]
    fn the_hour_goal_survives() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());
        storage.import_cronocad(&source).unwrap();

        let project = storage
            .projects(false)
            .unwrap()
            .into_iter()
            .find(|project| project.name == "Rancho Queimado")
            .unwrap();
        let tracking = storage
            .project_tracking()
            .unwrap()
            .into_iter()
            .find(|entry| entry.project_id == project.id)
            .unwrap();
        assert_eq!(tracking.budget_minutes, 2_400, "a meta de 40h se perdeu");
    }

    /// Sem esta recusa, rodar duas vezes dobraria as horas de todo projeto — e o
    /// erro so apareceria na hora de faturar.
    #[test]
    fn importing_twice_is_refused() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());
        storage.import_cronocad(&source).unwrap();

        let error = storage.import_cronocad(&source).unwrap_err();
        assert!(error.message.contains("ja foi importado"));
        assert!(storage.cronocad_imported_at().unwrap().is_some());
    }

    /// O caso que quebrou no primeiro uso real.
    ///
    /// A trava antiga recusava quando `time_entries` tinha qualquer linha, entao
    /// quem experimentou o cronometro antes de importar ficava impedido de
    /// importar PARA SEMPRE. "Ja importei" e "tem dado" nao sao a mesma
    /// pergunta, e este teste prende a diferenca.
    #[test]
    fn trying_the_timer_first_does_not_block_the_import() {
        let (storage, directory) = m_os();
        let existing = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        storage
            .start_timer(mos_core::StartTimer {
                project_id: existing.id,
                description: String::new(),
                activity_type: ActivityType::Other,
            })
            .unwrap();
        storage.stop_timer().unwrap();
        assert_eq!(storage.time_entries(None).unwrap().len(), 1);

        let report = storage
            .import_cronocad(&cronocad(directory.path()))
            .unwrap();
        assert_eq!(report.projects, 2);
        // A sessao do teste do cronometro continua la, somada as importadas.
        assert_eq!(storage.time_entries(None).unwrap().len(), 5);
    }

    /// Ensaio contra um banco REAL, sob demanda.
    ///
    /// Ignorado por padrao porque depende de um arquivo que so existe na maquina
    /// de quem usa o CronoCAD. Serve para conferir a importacao antes de rodar
    /// pra valer, e escreve num banco temporario — a origem e aberta somente
    /// leitura e nada toca o M/OS instalado.
    ///
    ///     $env:CRONOCAD_DB="C:\...\cronocad.sqlite"
    ///     cargo test -p mos-storage-sqlite -- --ignored --nocapture rehearsal
    #[test]
    #[ignore = "depende de um cronocad.sqlite real"]
    fn rehearsal_against_a_real_database() {
        let Ok(source) = std::env::var("CRONOCAD_DB") else {
            panic!("defina CRONOCAD_DB com o caminho do cronocad.sqlite");
        };
        let (storage, _directory) = m_os();

        let report = storage.import_cronocad(Path::new(&source)).unwrap();
        println!(
            "projetos={} sessoes={} tasks={} horas={:.1} programas={} eventos={} clientes={}",
            report.projects,
            report.entries,
            report.tasks,
            report.tracked_seconds as f64 / 3600.0,
            report.monitored_apps,
            report.activity_events,
            report.clients
        );
        let settings = storage.tracking_settings().unwrap();
        let watching = storage.monitoring_settings().unwrap();
        println!(
            "  arredondamento: ativo={} intervalo={}min modo={:?} inatividade={}min",
            settings.rounding.enabled,
            settings.rounding.interval_minutes,
            settings.rounding.mode,
            watching.idle_threshold_minutes
        );

        for tracking in storage.project_tracking().unwrap() {
            let project = storage
                .projects(true)
                .unwrap()
                .into_iter()
                .find(|candidate| candidate.id == tracking.project_id)
                .unwrap();
            let seconds: i64 = storage
                .time_entries(Some(project.id))
                .unwrap()
                .iter()
                .map(|entry| entry.duration_seconds)
                .sum();
            println!(
                "  {:<28} {:>6.1}h  taxa={} status={:?} lifecycle={:?}",
                project.name,
                seconds as f64 / 3600.0,
                tracking.hourly_rate_cents,
                tracking.tracking_status,
                project.lifecycle_state
            );
        }
        assert!(report.projects > 0, "nada foi importado");
    }

    /// O total importado precisa bater com o que o CronoCAD mostrava. Com
    /// arredondamento desligado, a sessao nao faturavel soma horas e nao soma
    /// valor, e a inatividade sai do faturavel sem sair do bruto.
    #[test]
    fn the_totals_match_what_cronocad_showed() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());
        storage.import_cronocad(&source).unwrap();

        let service = mos_core::TrackingService::new(std::sync::Arc::new(
            SqliteStorage::open(
                directory.path().join("mos.db"),
                directory.path().join("backups"),
            )
            .unwrap(),
        ));
        service
            .set_settings(TrackingSettings {
                rounding: Rounding {
                    enabled: false,
                    interval_minutes: 15,
                    mode: RoundingMode::Nearest,
                },
            })
            .unwrap();

        let totals = service.totals_by_project().unwrap();
        let rancho = totals
            .values()
            .find(|total| total.gross_seconds == 7_200 + 3_600 + 1_800)
            .expect("os totais do Rancho Queimado nao bateram");

        assert_eq!(rancho.idle_seconds, 600);
        // Faturavel: (7200-600) + 3600 = 10 200. A de 1800 nao e faturavel.
        assert_eq!(rancho.billable_seconds, 10_200);
        // 6600s a 30,00/h = 55,00; 3600s a 90,00/h = 90,00.
        assert_eq!(rancho.amount_cents, 5_500 + 9_000);
    }
}
