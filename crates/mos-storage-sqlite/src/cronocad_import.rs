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
use rusqlite::{Connection, OpenFlags};
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
    /// Recusa se ja houver sessao gravada, e a recusa e a protecao: sem ela,
    /// rodar duas vezes dobraria as horas de todo projeto, e o erro so
    /// apareceria na hora de faturar.
    pub fn import_cronocad(&self, source: &Path) -> Result<ImportReport, CoreError> {
        if !self.time_entries(None)?.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Ja existe tempo registrado no M/OS. A importacao do CronoCAD roda uma vez, \
                 em banco sem sessao — importar de novo dobraria as horas.",
                false,
            ));
        }

        let origin = Connection::open_with_flags(
            source,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(map_sql_error)?;

        let mut report = ImportReport::default();

        let mut projects = origin
            .prepare(
                "SELECT id, name, description, hourly_rate_cents, status, code, color, notes \
                 FROM projects ORDER BY name",
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
                ))
            })
            .map_err(map_sql_error)?;

        for row in rows {
            let (origin_id, name, description, rate, status, code, color, notes) =
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
            })?;

            if lifecycle != LifecycleState::Active {
                self.set_project_lifecycle(project.id, lifecycle)?;
            }

            report.tracked_seconds +=
                self.copy_entries(&origin, &origin_id, project.id, &mut report)?;
            report.tasks += self.copy_todos(&origin, &origin_id, project.id)?;
            report.projects += 1;
        }

        Ok(report)
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
    use mos_core::{Rounding, RoundingMode, TrackingSettings};

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
                code TEXT, color TEXT, notes TEXT);
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
                ('p1','Rancho Queimado','obra',3000,'active','043',NULL,'lembrar do corrimao'),
                ('p2','Juliano - POA',NULL,5000,'completed',NULL,NULL,NULL);

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
                ('t2','p1','Enviar PDF',1,'2026-07-12T05:00:00Z');",
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

    /// Sem esta recusa, rodar duas vezes dobraria as horas de todo projeto — e o
    /// erro so apareceria na hora de faturar.
    #[test]
    fn importing_twice_is_refused() {
        let (storage, directory) = m_os();
        let source = cronocad(directory.path());
        storage.import_cronocad(&source).unwrap();

        let error = storage.import_cronocad(&source).unwrap_err();
        assert!(error.message.contains("uma vez"));
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
            "projetos={} sessoes={} tasks={} horas={:.1}",
            report.projects,
            report.entries,
            report.tasks,
            report.tracked_seconds as f64 / 3600.0
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
                idle_threshold_minutes: 10,
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
