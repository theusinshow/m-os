mod app_repository;
mod attention_repository;
mod backup;
mod conversation_repository;
mod daily_repository;
mod device_repository;
mod cronocad_import;
mod ingestion_repository;
mod meeting_repository;
mod monitoring_repository;
mod repository;
mod resource_repository;
mod sync_emit;
mod sync_repository;
mod tracking_repository;
mod voice_repository;
mod work_repository;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use mos_core::{CoreError, ErrorCode};
use rusqlite::{Connection, MAIN_DB};
use serde::Serialize;

pub use cronocad_import::ImportReport;

const SCHEMA_VERSION: u32 = 30;
const MIGRATION_001: &str = include_str!("../migrations/0001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/0002_work.sql");
const MIGRATION_003: &str = include_str!("../migrations/0003_apps.sql");
const MIGRATION_004: &str = include_str!("../migrations/0004_workspaces.sql");
const MIGRATION_005: &str = include_str!("../migrations/0005_app_sources.sql");
const MIGRATION_006: &str = include_str!("../migrations/0006_resources.sql");
const MIGRATION_007: &str = include_str!("../migrations/0007_v03_design.sql");
const MIGRATION_008: &str = include_str!("../migrations/0008_workspace_widgets.sql");
const MIGRATION_009: &str = include_str!("../migrations/0009_resource_workspaces.sql");
const MIGRATION_010: &str = include_str!("../migrations/0010_conversations.sql");
const MIGRATION_011: &str = include_str!("../migrations/0011_time_tracking.sql");
const MIGRATION_012: &str = include_str!("../migrations/0012_cronocad_import_mark.sql");
const MIGRATION_013: &str = include_str!("../migrations/0013_tracking_surface.sql");
const MIGRATION_014: &str = include_str!("../migrations/0014_project_budget.sql");
const MIGRATION_015: &str = include_str!("../migrations/0015_attention.sql");
const MIGRATION_016: &str = include_str!("../migrations/0016_widget_order.sql");
const MIGRATION_017: &str = include_str!("../migrations/0017_widget_layout.sql");
const MIGRATION_018: &str = include_str!("../migrations/0018_layout_sem_workspace.sql");
const MIGRATION_019: &str = include_str!("../migrations/0019_ocultos_sem_workspace.sql");
// A de Meeting nasceu 0017 na branch e virou 0020 no merge: master chegou ao
// 17 primeiro. Renumerar aqui e seguro porque a branch nunca rodou na maquina
// de ninguem — se tivesse rodado, um banco com `user_version = 17` de Meeting
// nao poderia ser distinguido de um com o 17 de widgets, e a saida seria uma
// migration de conserto em vez de uma renumeracao.
const MIGRATION_020: &str = include_str!("../migrations/0020_meetings.sql");
const MIGRATION_021: &str = include_str!("../migrations/0021_radial_pins.sql");
const MIGRATION_022: &str = include_str!("../migrations/0022_meeting_notes.sql");
const MIGRATION_023: &str = include_str!("../migrations/0023_universal_drop.sql");
const MIGRATION_024: &str = include_str!("../migrations/0024_meeting_detection.sql");
// A de voz nasceu 0022 na branch e virou 0025 no merge: a master chegou ao 24
// primeiro. Renumerar e seguro porque a branch nunca rodou na maquina de
// ninguem — e a mesma situacao que a 0020 registrou quando o Meeting Agent
// entrou. Um banco que ja tivesse visto o 22 antigo nao poderia ser
// distinguido do 22 de notas de reuniao, e a saida seria uma migration de
// conserto em vez de uma renumeracao.
const MIGRATION_025: &str = include_str!("../migrations/0025_voice.sql");
const MIGRATION_026: &str = include_str!("../migrations/0026_project_paid.sql");
const MIGRATION_027: &str = include_str!("../migrations/0027_sync_foundation.sql");
const MIGRATION_028: &str = include_str!("../migrations/0028_daily_session.sql");
const MIGRATION_029: &str = include_str!("../migrations/0029_weekly_review.sql");
const MIGRATION_030: &str = include_str!("../migrations/0030_orfas_de_meeting.sql");

pub struct SqliteStorage {
    connection: Mutex<Connection>,
    backup_lock: Mutex<()>,
    database_path: PathBuf,
    backup_directory: PathBuf,
    /// O relogio logico, quando a emissao de operacoes esta ligada.
    ///
    /// `None` significa sincronizacao desligada, e nao erro: o M/OS funciona
    /// inteiro sem ela, e e isso que permite ligar a emissao por entidade, uma
    /// de cada vez, sem parar o desktop. Ver `sync_emit.rs`.
    sync: Mutex<Option<mos_sync::HlcClock>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageHealth {
    pub database_path: String,
    pub schema_version: u32,
    pub journal_mode: String,
    pub synchronous: String,
    pub integrity: String,
}

impl SqliteStorage {
    pub fn open(
        database_path: impl AsRef<Path>,
        backup_directory: impl AsRef<Path>,
    ) -> Result<Self, CoreError> {
        let database_path = database_path.as_ref().to_path_buf();
        let backup_directory = backup_directory.as_ref().to_path_buf();
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(map_io_error)?;
        }
        fs::create_dir_all(&backup_directory).map_err(map_io_error)?;

        let connection = Connection::open(&database_path).map_err(map_sql_error)?;
        configure_connection(&connection)?;
        verify_integrity(&connection)?;
        migrate(&connection, &backup_directory)?;
        ensure_search_projection(&connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
            backup_lock: Mutex::new(()),
            database_path,
            backup_directory,
            sync: Mutex::new(None),
        })
    }

    pub fn health(&self) -> Result<StorageHealth, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(map_sql_error)?;
        let synchronous: i64 = connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(map_sql_error)?;
        let schema_version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(map_sql_error)?;
        let integrity: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(map_sql_error)?;

        Ok(StorageHealth {
            database_path: self.database_path.display().to_string(),
            schema_version,
            journal_mode,
            synchronous: synchronous_name(synchronous).into(),
            integrity,
        })
    }
}

fn configure_connection(connection: &Connection) -> Result<(), CoreError> {
    connection
        .busy_timeout(Duration::from_millis(1_000))
        .map_err(map_sql_error)?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             PRAGMA trusted_schema=OFF;",
        )
        .map_err(map_sql_error)?;

    let journal: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    let synchronous: i64 = connection
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(map_sql_error)?;

    if !journal.eq_ignore_ascii_case("wal") || synchronous != 2 || foreign_keys != 1 {
        return Err(CoreError::new(
            ErrorCode::StorageUnavailable,
            "O banco local nao abriu com as garantias de durabilidade exigidas.",
            false,
        ));
    }
    Ok(())
}

fn verify_integrity(connection: &Connection) -> Result<(), CoreError> {
    let result: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    if result != "ok" {
        return Err(CoreError::new(
            ErrorCode::DataIntegrity,
            format!("O banco local falhou na verificacao de integridade: {result}"),
            false,
        ));
    }
    Ok(())
}

fn migrate(connection: &Connection, backup_directory: &Path) -> Result<(), CoreError> {
    let current: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    if current > SCHEMA_VERSION {
        return Err(CoreError::new(
            ErrorCode::StorageUnavailable,
            "O banco local foi criado por uma versao mais nova do M/OS.",
            false,
        ));
    }
    if current > 0 && current < SCHEMA_VERSION {
        create_pre_migration_snapshot(connection, backup_directory, current)?;
    }
    /* O que ja estava quebrado ANTES de a migration encostar no banco.
       Sem esta medida a guarda do fim nao sabe de quem e a culpa, e foi assim
       que em 2026-08-22 o app recusou abrir acusando uma migration inocente:
       as 50 orfas eram de uma reuniao apagada por fora em 2026-08-21, e o
       snapshot `pre-migration-v26` prova que elas ja estavam la. */
    let orfas_antes = if current > 0 && current < SCHEMA_VERSION {
        contagem_de_orfas(connection)?
    } else {
        Vec::new()
    };
    if current == 0 {
        connection
            .execute_batch(MIGRATION_001)
            .map_err(map_sql_error)?;
    }
    if current <= 1 {
        connection
            .execute_batch(MIGRATION_002)
            .map_err(map_sql_error)?;
    }
    if current <= 2 {
        connection
            .execute_batch(MIGRATION_003)
            .map_err(map_sql_error)?;
    }
    if current <= 3 {
        connection
            .execute_batch(MIGRATION_004)
            .map_err(map_sql_error)?;
    }
    if current <= 4 {
        connection
            .execute_batch(MIGRATION_005)
            .map_err(map_sql_error)?;
    }
    if current <= 5 {
        connection
            .execute_batch(MIGRATION_006)
            .map_err(map_sql_error)?;
    }
    if current <= 6 {
        connection
            .execute_batch(MIGRATION_007)
            .map_err(map_sql_error)?;
    }
    if current <= 7 {
        connection
            .execute_batch(MIGRATION_008)
            .map_err(map_sql_error)?;
    }
    if current <= 8 {
        connection
            .execute_batch(MIGRATION_009)
            .map_err(map_sql_error)?;
    }
    if current <= 9 {
        connection
            .execute_batch(MIGRATION_010)
            .map_err(map_sql_error)?;
    }
    if current <= 10 {
        connection
            .execute_batch(MIGRATION_011)
            .map_err(map_sql_error)?;
    }
    if current <= 11 {
        connection
            .execute_batch(MIGRATION_012)
            .map_err(map_sql_error)?;
    }
    if current <= 12 {
        connection
            .execute_batch(MIGRATION_013)
            .map_err(map_sql_error)?;
    }
    if current <= 13 {
        connection
            .execute_batch(MIGRATION_014)
            .map_err(map_sql_error)?;
    }
    if current <= 14 {
        connection
            .execute_batch(MIGRATION_015)
            .map_err(map_sql_error)?;
    }
    if current <= 15 {
        connection
            .execute_batch(MIGRATION_016)
            .map_err(map_sql_error)?;
    }
    if current <= 16 {
        connection
            .execute_batch(MIGRATION_017)
            .map_err(map_sql_error)?;
    }
    if current <= 17 {
        connection
            .execute_batch(MIGRATION_018)
            .map_err(map_sql_error)?;
    }
    if current <= 18 {
        connection
            .execute_batch(MIGRATION_019)
            .map_err(map_sql_error)?;
    }
    if current <= 19 {
        connection
            .execute_batch(MIGRATION_020)
            .map_err(map_sql_error)?;
    }
    if current <= 20 {
        connection
            .execute_batch(MIGRATION_021)
            .map_err(map_sql_error)?;
    }
    if current <= 21 {
        connection
            .execute_batch(MIGRATION_022)
            .map_err(map_sql_error)?;
    }
    if current <= 22 {
        connection
            .execute_batch(MIGRATION_023)
            .map_err(map_sql_error)?;
    }
    if current <= 23 {
        connection
            .execute_batch(MIGRATION_024)
            .map_err(map_sql_error)?;
    }
    if current <= 24 {
        connection
            .execute_batch(MIGRATION_025)
            .map_err(map_sql_error)?;
    }
    if current <= 25 {
        connection
            .execute_batch(MIGRATION_026)
            .map_err(map_sql_error)?;
    }
    if current <= 26 {
        connection
            .execute_batch(MIGRATION_027)
            .map_err(map_sql_error)?;
    }
    if current <= 27 {
        connection
            .execute_batch(MIGRATION_028)
            .map_err(map_sql_error)?;
    }
    if current <= 28 {
        connection
            .execute_batch(MIGRATION_029)
            .map_err(map_sql_error)?;
    }
    if current <= 29 {
        connection
            .execute_batch(MIGRATION_030)
            .map_err(map_sql_error)?;
    }
    if current < SCHEMA_VERSION {
        verify_foreign_keys(connection, &orfas_antes)?;
    }
    Ok(())
}

/// Quantas referencias orfas existem, POR TABELA.
///
/// Por tabela e nao um total: uma migration que criasse uma orfa em `tasks`
/// enquanto outra limpasse cinquenta de `meeting_transcript_index` faria o total
/// CAIR, e a regressao passaria em silencio.
fn contagem_de_orfas(connection: &Connection) -> Result<Vec<(String, i64)>, CoreError> {
    let mut statement = connection
        .prepare(
            "SELECT \"table\", count(*) FROM pragma_foreign_key_check              GROUP BY \"table\" ORDER BY \"table\"",
        )
        .map_err(map_sql_error)?;
    let linhas = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
        .map_err(map_sql_error)?;
    linhas
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(map_sql_error)
}

/// A migration responde pelo que ELA deixou, e nao pelo que encontrou.
///
/// # Por que comparativa
///
/// A versao anterior contava as orfas depois de migrar e recusava abrir se
/// houvesse qualquer uma. Em 2026-08-22 isso trancou a porta na maquina do dono
/// por 50 linhas de indice de reuniao que estavam ali desde o dia 21 — deixadas
/// por uma limpeza feita FORA do app, onde `PRAGMA foreign_keys` vem desligado
/// por padrao. A mensagem dizia "A migration deixou 50 referencias orfas", e a
/// migration nao tinha deixado nenhuma.
///
/// Sujeira antiga nao e emergencia: lixo de indice nao corrompe leitura, e a
/// resposta certa e uma migration de conserto que a conheca — como a 0030 —, e
/// nao um app que nao abre. Regressao de migration continua sendo erro duro.
fn verify_foreign_keys(
    connection: &Connection,
    antes: &[(String, i64)],
) -> Result<(), CoreError> {
    let enabled: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(map_sql_error)?;
    if enabled != 1 {
        return Err(CoreError::new(
            ErrorCode::DataIntegrity,
            "Uma migration deixou as foreign keys desligadas.",
            false,
        ));
    }

    let depois = contagem_de_orfas(connection)?;
    let quantas_antes = |tabela: &str| {
        antes
            .iter()
            .find(|(nome, _)| nome == tabela)
            .map(|(_, quantas)| *quantas)
            .unwrap_or(0)
    };

    let novas: Vec<String> = depois
        .iter()
        .filter(|(tabela, quantas)| *quantas > quantas_antes(tabela))
        .map(|(tabela, quantas)| {
            format!("{tabela} ({} a mais)", quantas - quantas_antes(tabela))
        })
        .collect();

    if !novas.is_empty() {
        return Err(CoreError::new(
            ErrorCode::DataIntegrity,
            format!(
                "Esta migration deixou referencias orfas em: {}.",
                novas.join(", ")
            ),
            false,
        ));
    }
    Ok(())
}

fn create_pre_migration_snapshot(
    connection: &Connection,
    backup_directory: &Path,
    version: u32,
) -> Result<(), CoreError> {
    let destination = backup_directory.join(format!(
        "pre-migration-v{version}-{}.db",
        time::OffsetDateTime::now_utc().unix_timestamp()
    ));
    connection
        .backup(MAIN_DB, &destination, None)
        .map_err(map_sql_error)?;
    let snapshot = Connection::open(&destination).map_err(map_sql_error)?;
    verify_integrity(&snapshot)
}

fn ensure_search_projection(connection: &Connection) -> Result<(), CoreError> {
    for (source, projection) in [
        ("captures", "capture_search"),
        ("projects", "project_search"),
        ("tasks", "task_search"),
        ("apps", "app_search"),
        ("workspaces", "workspace_search"),
        ("resources", "resource_search"),
        ("ingestions", "ingestion_search"),
        ("message_parts", "message_search"),
    ] {
        let source_count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {source}"), [], |row| {
                row.get(0)
            })
            .map_err(map_sql_error)?;
        let search_count: i64 = connection
            .query_row(&format!("SELECT count(*) FROM {projection}"), [], |row| {
                row.get(0)
            })
            .map_err(map_sql_error)?;
        if source_count != search_count {
            connection
                .execute(
                    &format!("INSERT INTO {projection}({projection}) VALUES('rebuild')"),
                    [],
                )
                .map_err(map_sql_error)?;
        }
    }
    Ok(())
}

fn rebuild_search_projection(connection: &Connection) -> Result<usize, CoreError> {
    let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
    transaction
        .execute(
            "INSERT INTO capture_search(capture_search) VALUES('rebuild')",
            [],
        )
        .map_err(map_sql_error)?;
    let count = transaction
        .query_row("SELECT count(*) FROM capture_search", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(map_sql_error)? as usize;
    transaction.commit().map_err(map_sql_error)?;
    Ok(count)
}

fn synchronous_name(value: i64) -> &'static str {
    match value {
        0 => "OFF",
        1 => "NORMAL",
        2 => "FULL",
        3 => "EXTRA",
        _ => "UNKNOWN",
    }
}

fn map_sql_error(error: rusqlite::Error) -> CoreError {
    use rusqlite::{Error, ErrorCode as SqlErrorCode};

    let (code, retryable, message) = match &error {
        Error::SqliteFailure(details, _)
            if matches!(
                details.code,
                SqlErrorCode::DatabaseBusy | SqlErrorCode::DatabaseLocked
            ) =>
        {
            (
                ErrorCode::StorageBusy,
                true,
                "O banco local esta ocupado. Nada foi salvo; tente novamente.".to_owned(),
            )
        }
        Error::SqliteFailure(details, _) if details.code == SqlErrorCode::DiskFull => (
            ErrorCode::StorageUnavailable,
            false,
            "O disco esta sem espaco. Nada foi salvo.".to_owned(),
        ),
        Error::SqliteFailure(details, _) if details.code == SqlErrorCode::DatabaseCorrupt => (
            ErrorCode::DataIntegrity,
            false,
            "O banco local parece corrompido. Escritas foram bloqueadas.".to_owned(),
        ),
        _ => (
            ErrorCode::StorageUnavailable,
            false,
            format!("Falha no armazenamento local: {error}"),
        ),
    };
    CoreError::new(code, message, retryable)
}

fn map_io_error(error: std::io::Error) -> CoreError {
    CoreError::new(
        ErrorCode::Io,
        format!("Falha de arquivo local: {error}"),
        false,
    )
}

fn map_lock_error<T>(error: std::sync::PoisonError<T>) -> CoreError {
    CoreError::new(
        ErrorCode::StorageUnavailable,
        format!("Acesso ao banco local foi interrompido: {error}"),
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{
        AppRepository, CaptureRepository, NewResource, ResourceRepository, SearchRequest,
    };

    /// Sobe um banco v16 POVOADO ate a v17.
    ///
    /// O teste que importa nao e "a migration roda": e "a migration roda sem
    /// perder o que ja estava la". Um banco de verdade na hora do upgrade tem
    /// Captures, Projects e Tasks, e a `0017` so acrescenta tabelas — mas e
    /// exatamente esse tipo de certeza que precisa ser exercitado antes de
    /// alcancar a maquina de alguem.
    #[test]
    fn upgrades_v16_preserving_existing_data() {
        use mos_core::{MeetingRepository, MeetingSource, NewMeeting, NewProject, WorkRepository};

        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();

        // Um banco parado na versao anterior, com dado dentro.
        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            MIGRATION_001,
            MIGRATION_002,
            MIGRATION_003,
            MIGRATION_004,
            MIGRATION_005,
            MIGRATION_006,
            MIGRATION_007,
            MIGRATION_008,
            MIGRATION_009,
            MIGRATION_010,
            MIGRATION_011,
            MIGRATION_012,
            MIGRATION_013,
            MIGRATION_014,
            MIGRATION_015,
            MIGRATION_016,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        connection
            .execute(
                "INSERT INTO captures (
                    id, content, source_kind, processing_state, lifecycle_state,
                    captured_at, created_at, updated_at
                 ) VALUES (
                    '0198a7d5-a64e-7000-8000-0000000000aa', 'Sobreviveu ao upgrade', 'home',
                    'inbox', 'active', '2026-08-18T00:00:00Z',
                    '2026-08-18T00:00:00Z', '2026-08-18T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 16, "o banco de partida precisa estar na v16");
        drop(connection);

        // A abertura migra.
        let storage = SqliteStorage::open(&database, &backups).unwrap();
        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        assert_eq!(storage.health().unwrap().integrity, "ok");

        // O que ja existia continua la.
        assert_eq!(CaptureRepository::recent(&storage, 10).unwrap().len(), 1);

        // E o snapshot pre-migration foi criado, como manda `ARCHITECTURE.md` §16.
        assert_eq!(
            fs::read_dir(&backups)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("pre-migration-v16-"))
                .count(),
            1
        );

        // E as tabelas novas funcionam, incluindo a FK para `projects`, que so
        // existe porque a migration anterior ja tinha rodado.
        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        let meeting = storage
            .create_meeting(NewMeeting::start(
                "Primeira reuniao depois do upgrade",
                MeetingSource::Manual,
                Some(project.id),
                time::macros::datetime!(2026-08-18 14:00:00 UTC),
            ))
            .unwrap();
        assert_eq!(storage.meeting(meeting.id).unwrap().project_id, Some(project.id));
    }


    /// Um banco v29 com o rastro que a limpeza por fora deixa: a reuniao apagada,
    /// e os dois indices do FTS intactos apontando para o vazio.
    ///
    /// # Por que este teste existe
    ///
    /// Foi o estado real da maquina do dono em 2026-08-22: 48 linhas em
    /// `meeting_transcript_index` e 2 em `meeting_search_index` apontando para
    /// `meetings` que nao existiam mais. Qualquer cliente SQLite — DB Browser,
    /// o `sqlite3`, um script — abre com `PRAGMA foreign_keys` DESLIGADO, que e
    /// o padrao do proprio SQLite; so o M/OS liga. Um `DELETE FROM meetings`
    /// dado ali fora nao cascateia, e o rastro fica.
    ///
    /// O sintoma so apareceu meses depois, na migration seguinte: o app recusou
    /// abrir com "A migration deixou 50 referencias orfas", acusando uma
    /// migration que nao tinha feito nada.
    fn banco_com_orfas_de_meeting(connection: &Connection) {
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 INSERT INTO meetings (
                     id, title, status, lifecycle_state, source, audio_dir,
                     started_at, created_at, updated_at
                 ) VALUES (
                     '01a0224b-3fab-7a43-8fba-70b2cafa5409', 'Apagada por fora',
                     'ready', 'active', 'manual', 'meetings/01a0224b',
                     '2026-08-21T03:09:11Z', '2026-08-21T03:09:11Z', '2026-08-21T03:09:11Z'
                 );
                 INSERT INTO meeting_search_index (rowid, meeting_id)
                 VALUES (1, '01a0224b-3fab-7a43-8fba-70b2cafa5409');
                 INSERT INTO meeting_transcript_index (rowid, meeting_id, segment_id)
                 VALUES (1, '01a0224b-3fab-7a43-8fba-70b2cafa5409',
                         '01a0224f-4744-7de2-a199-08b2e86c0c38');
                 INSERT INTO meeting_search (rowid, title, summary, insights)
                 VALUES (1, 'Apagada por fora', '', '');
                 INSERT INTO meeting_transcript_search (rowid, text)
                 VALUES (1, 'o trecho que sobrou');
                 DELETE FROM meetings;
                 PRAGMA foreign_keys = ON;",
            )
            .unwrap();
    }

    fn orfas(connection: &Connection) -> i64 {
        connection
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    /// A migration 0030 varre o rastro, e leva junto o lixo do FTS.
    ///
    /// Apagar so a linha do indice deixaria o texto no `meeting_search`: a busca
    /// devolveria o titulo de uma reuniao que nao existe, e o clique nao teria
    /// para onde ir. As duas metades saem juntas ou nenhuma sai.
    #[test]
    fn a_0030_limpa_o_rastro_de_meeting_apagada_por_fora() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();

        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        migrate(&connection, &backups).unwrap();

        // Volta para a v29 e planta o rastro: e o banco que chegou aqui.
        connection.execute_batch("PRAGMA user_version = 29;").unwrap();
        banco_com_orfas_de_meeting(&connection);
        assert_eq!(orfas(&connection), 2, "o rastro precisa existir antes");

        migrate(&connection, &backups).unwrap();

        assert_eq!(orfas(&connection), 0, "a 0030 varre o rastro");
        for tabela in [
            "meeting_search_index",
            "meeting_transcript_index",
            "meeting_search",
            "meeting_transcript_search",
        ] {
            let restante: i64 = connection
                .query_row(&format!("SELECT count(*) FROM {tabela}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(restante, 0, "{tabela} nao pode guardar o que perdeu o dono");
        }
    }

    /// Indice de reuniao que EXISTE nao pode ser varrido junto.
    ///
    /// Uma limpeza que leva o valido embora seria pior que a sujeira: a busca
    /// pararia de achar reuniao nenhuma, e sem erro nenhum.
    #[test]
    fn a_0030_nao_encosta_no_indice_de_reuniao_viva() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();

        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        migrate(&connection, &backups).unwrap();
        connection.execute_batch("PRAGMA user_version = 29;").unwrap();

        connection
            .execute_batch(
                "INSERT INTO meetings (
                     id, title, status, lifecycle_state, source, audio_dir,
                     started_at, created_at, updated_at
                 ) VALUES (
                     '01a0225f-0a7d-7671-8197-b5fa30968e71', 'Viva',
                     'ready', 'active', 'manual', 'meetings/01a0225f',
                     '2026-08-21T03:30:49Z', '2026-08-21T03:30:49Z', '2026-08-21T03:30:49Z'
                 );
                 INSERT INTO meeting_search_index (rowid, meeting_id)
                 VALUES (9, '01a0225f-0a7d-7671-8197-b5fa30968e71');
                 INSERT INTO meeting_search (rowid, title, summary, insights)
                 VALUES (9, 'Viva', '', '');",
            )
            .unwrap();

        migrate(&connection, &backups).unwrap();

        let sobrou: i64 = connection
            .query_row("SELECT count(*) FROM meeting_search_index", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(sobrou, 1, "a reuniao viva continua indexada");
        let texto: i64 = connection
            .query_row("SELECT count(*) FROM meeting_search", [], |row| row.get(0))
            .unwrap();
        assert_eq!(texto, 1, "e o texto dela tambem");
    }


    /// Sujeira que ja estava no banco NAO pode trancar a porta.
    ///
    /// Era o comportamento ate 2026-08-22: `verify_foreign_keys` contava as
    /// orfas depois de migrar, sem saber quantas havia antes, e qualquer rastro
    /// antigo virava uma recusa de abrir — acusando "a migration", que nao tinha
    /// feito nada. O app ficava inutilizavel por lixo de indice que nao afeta
    /// leitura nenhuma.
    ///
    /// A regra certa e comparativa: a migration responde pelo que ELA deixou.
    #[test]
    fn orfa_pre_existente_nao_impede_o_app_de_abrir() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();

        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        migrate(&connection, &backups).unwrap();

        // Uma Task apontando para uma Capture que nao existe. E uma orfa que a
        // 0030 NAO limpa — de proposito: o teste precisa de sujeira que
        // sobreviva a migracao para provar que ela nao tranca a porta.
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 INSERT INTO captures (
                     id, content, source_kind, processing_state, lifecycle_state,
                     captured_at, created_at, updated_at
                 ) VALUES (
                     '01a0224b-0000-7000-8000-00000000c001', 'a que sumiu',
                     'quick_capture', 'processed', 'active',
                     '2026-08-21T03:09:11Z', '2026-08-21T03:09:11Z', '2026-08-21T03:09:11Z'
                 );
                 INSERT INTO tasks (
                     id, title, description, source_capture_id, work_state,
                     lifecycle_state, created_at, updated_at
                 ) VALUES (
                     '01a0224b-0000-7000-8000-00000000d001', 'orfa de proveniencia', '',
                     '01a0224b-0000-7000-8000-00000000c001', 'backlog', 'active',
                     '2026-08-21T03:09:11Z', '2026-08-21T03:09:11Z'
                 );
                 DELETE FROM captures;
                 PRAGMA foreign_keys = ON;",
            )
            .unwrap();
        let antes = orfas(&connection);
        assert!(antes > 0, "o teste precisa de sujeira pre-existente");

        // Volta uma versao e migra de novo: e o caminho real de quem atualiza o
        // app com um banco ja sujo.
        connection.execute_batch("PRAGMA user_version = 29;").unwrap();
        migrate(&connection, &backups).expect("sujeira antiga nao tranca a porta");

        assert_eq!(
            orfas(&connection),
            antes,
            "e ela continua la, para a migration de conserto que souber trata-la"
        );
    }

    /// Orfa que a migration CRIOU continua sendo erro.
    ///
    /// E o caso que a guarda existe para pegar, e afrouxa-la seria trocar um
    /// falso positivo por um falso negativo.
    #[test]
    fn orfa_criada_pela_migration_continua_recusada() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();

        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        migrate(&connection, &backups).unwrap();

        let antes = contagem_de_orfas(&connection).unwrap();
        assert!(antes.is_empty(), "o banco novo nasce limpo");

        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 INSERT INTO meeting_search_index (rowid, meeting_id)
                 VALUES (77, '01a0224b-3fab-7a43-8fba-70b2cafa5409');
                 PRAGMA foreign_keys = ON;",
            )
            .unwrap();

        let erro = verify_foreign_keys(&connection, &antes).unwrap_err();
        assert_eq!(erro.code, ErrorCode::DataIntegrity);
        assert!(
            erro.message.contains("meeting_search_index"),
            "a mensagem precisa nomear a tabela, e nao so contar: {}",
            erro.message
        );
    }

    /// Sobe um banco v21 POVOADO ate a v22.
    ///
    /// Este e o teste que a migration de voz precisava ter antes de alcancar a
    /// maquina de alguem. Ele sobe um banco v21 povoado e o leva ate a versao
    /// atual, atravessando as DUAS recriacoes de `captures` que existem no
    /// caminho — a da 0023, para admitir `drop`, e a da 0025, para admitir
    /// `voice` —, ambas com a guarda de integridade referencial desligada.
    ///
    /// Tres coisas podiam quebrar em silencio, e as tres sao verificadas aqui:
    ///
    /// 1. **A busca apontar para a linha errada.** `capture_search` e uma FTS5
    ///    de conteudo externo indexada por `content_rowid`. Se o swap
    ///    renumerasse as linhas, procurar por uma palavra devolveria outra
    ///    Capture — sem erro nenhum, e sem sintoma ate alguem ler o resultado.
    /// 2. **A proveniencia se perder.** `tasks.source_capture_id` aponta para
    ///    `captures` com ON DELETE RESTRICT. Com as FKs desligadas, um erro no
    ///    caminho deixaria a Task apontando para o vazio em vez de falhar.
    /// 3. **A guarda nao voltar.** Sair da migration com `foreign_keys=OFF`
    ///    seria pior que o problema que ela resolve.
    #[test]
    fn upgrades_v21_through_every_captures_rebuild() {
        use mos_core::{CaptureSource, NewCapture, NewProject, NewTask, WorkRepository};
        use rusqlite::params_from_iter;

        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();

        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            MIGRATION_001, MIGRATION_002, MIGRATION_003, MIGRATION_004, MIGRATION_005,
            MIGRATION_006, MIGRATION_007, MIGRATION_008, MIGRATION_009, MIGRATION_010,
            MIGRATION_011, MIGRATION_012, MIGRATION_013, MIGRATION_014, MIGRATION_015,
            MIGRATION_016, MIGRATION_017, MIGRATION_018, MIGRATION_019, MIGRATION_020,
            MIGRATION_021,
        ] {
            connection.execute_batch(migration).unwrap();
        }

        // DUAS Captures, e a que interessa e a SEGUNDA: com uma so, qualquer
        // desalinhamento de rowid ainda devolveria a linha certa por acidente.
        for (id, content) in [
            ("0198a7d5-a64e-7000-8000-0000000000b1", "a primeira, sobre orcamento"),
            ("0198a7d5-a64e-7000-8000-0000000000b2", "a segunda, sobre memorial"),
        ] {
            connection
                .execute(
                    "INSERT INTO captures (
                        id, content, source_kind, processing_state, lifecycle_state,
                        captured_at, created_at, updated_at
                     ) VALUES (?1, ?2, 'quick_capture', 'inbox', 'active',
                        '2026-08-18T00:00:00Z', '2026-08-18T00:00:00Z', '2026-08-18T00:00:00Z')",
                    params_from_iter([id, content]),
                )
                .unwrap();
            let rowid = connection.last_insert_rowid();
            connection
                .execute(
                    "INSERT INTO capture_search (rowid, content)
                     SELECT rowid, content FROM captures WHERE rowid = ?1",
                    [rowid],
                )
                .unwrap();
        }

        // E uma Task derivada da segunda: e a FK que o DROP TABLE recusaria.
        connection
            .execute(
                "INSERT INTO tasks (
                    id, title, description, project_id, source_capture_id, work_state,
                    lifecycle_state, created_at, updated_at
                 ) VALUES (
                    '0198a7d5-a64e-7000-8000-0000000000c1', 'Revisar memorial', '', NULL,
                    '0198a7d5-a64e-7000-8000-0000000000b2', 'backlog', 'active',
                    '2026-08-18T00:00:00Z', '2026-08-18T00:00:00Z'
                 )",
                [],
            )
            .unwrap();

        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 21, "o banco de partida precisa estar na v21");
        drop(connection);

        let storage = SqliteStorage::open(&database, &backups).unwrap();
        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        assert_eq!(storage.health().unwrap().integrity, "ok");

        // 1. A busca continua apontando para a Capture certa.
        let achados = CaptureRepository::search(
            &storage,
            SearchRequest {
                query: "memorial".into(),
                include_archived: false,
                limit: 10,
            },
        )
        .unwrap();
        assert_eq!(achados.len(), 1);
        assert_eq!(achados[0].content, "a segunda, sobre memorial");

        // 2. A proveniencia sobreviveu ao swap.
        let tasks = WorkRepository::tasks(&storage, false).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks[0].source_capture_id.map(|id| id.to_string()),
            Some("0198a7d5-a64e-7000-8000-0000000000b2".to_owned())
        );

        // 3. A guarda voltou, e nao sobrou referencia orfa.
        {
            let connection = storage.connection.lock().unwrap();
            let foreign_keys: i64 = connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                .unwrap();
            assert_eq!(foreign_keys, 1, "as FKs precisam voltar ligadas");
            let mut check = connection.prepare("PRAGMA foreign_key_check").unwrap();
            let orfas = check
                .query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .count();
            assert_eq!(orfas, 0);
        }

        // E a origem nova passa a ser aceita — o ponto de tudo isto.
        let falada = CaptureRepository::create(
            &storage,
            NewCapture::create("comprar cafe", CaptureSource::Voice).unwrap(),
        )
        .unwrap();
        assert_eq!(falada.source, CaptureSource::Voice);

        // E a Task com Reminder atomica funciona sobre o schema novo.
        let projeto = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        let (task, reminder) = storage
            .create_task_from_capture_with_reminder(
                falada.id,
                NewTask::create("Comprar cafe", "", Some(projeto.id)).unwrap(),
                None,
            )
            .unwrap();
        assert_eq!(task.source_capture_id, Some(falada.id));
        assert!(reminder.is_none());
    }

    #[test]
    fn opens_with_required_pragmas_and_schema() {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        let health = storage.health().unwrap();

        assert_eq!(health.journal_mode.to_lowercase(), "wal");
        assert_eq!(health.synchronous, "FULL");
        assert_eq!(health.schema_version, SCHEMA_VERSION);
        assert_eq!(health.integrity, "ok");
    }

    #[test]
    fn upgrades_v1_after_creating_a_consistent_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();
        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(MIGRATION_001).unwrap();
        connection
            .execute(
                "INSERT INTO captures (
                    id, content, source_kind, processing_state, lifecycle_state,
                    captured_at, created_at, updated_at
                 ) VALUES (
                    '0198a7d5-a64e-7000-8000-000000000001', 'Preservada', 'home',
                    'inbox', 'active', '2026-08-13T00:00:00Z',
                    '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        drop(connection);

        let storage = SqliteStorage::open(&database, &backups).unwrap();
        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        assert_eq!(CaptureRepository::recent(&storage, 10).unwrap().len(), 1);
        assert_eq!(
            fs::read_dir(&backups)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("pre-migration-v1-"))
                .count(),
            1
        );
    }

    /// Sobe um v6 povoado ate v7 e confere as tres mudancas do design v0.3.
    ///
    /// O ponto delicado e a busca: tasks e resources sao recriadas, e as tabelas
    /// FTS de conteudo externo indexam por rowid. Depois de um swap as contagens
    /// continuam batendo enquanto os rowids mudam, entao ensure_search_projection()
    /// nao detectaria a divergencia. Se a migration esquecer o rebuild explicito,
    /// a busca passa a devolver resultado errado em silencio — e este teste falha.
    #[test]
    fn upgrades_populated_v6_to_the_v03_design_schema() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();
        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            MIGRATION_001,
            MIGRATION_002,
            MIGRATION_003,
            MIGRATION_004,
            MIGRATION_005,
            MIGRATION_006,
        ] {
            connection.execute_batch(migration).unwrap();
        }

        connection
            .execute(
                "INSERT INTO projects (id, name, description, lifecycle_state, created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000010', 'Minarum', 'Escadas',
                         'active', '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO tasks (id, title, description, work_state, lifecycle_state,
                                    created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000011', 'Refatorar navbar',
                         'com urgencia', 'doing', 'active',
                         '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO resources (id, kind, title, url, note, lifecycle_state,
                                        created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000012', 'link', 'Motion',
                         'https://motion.dev', 'animacao declarativa', 'active',
                         '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO apps (id, name, description, launch_kind, launch_target,
                                   lifecycle_state, created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000013', 'ChronoCAD', 'Cronometro',
                         'url', 'https://chronocad.local', 'active',
                         '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .unwrap();
        drop(connection);

        let storage = SqliteStorage::open(&database, &backups).unwrap();
        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        assert_eq!(storage.health().unwrap().integrity, "ok");

        let connection = Connection::open(&database).unwrap();

        // 1. Estado existente preservado, e os tres estados novos sao aceitos.
        let state: String = connection
            .query_row(
                "SELECT work_state FROM tasks WHERE id = '0198a7d5-a64e-7000-8000-000000000011'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(state, "doing", "estado existente nao pode ser reescrito");

        for new_state in ["inbox", "planned", "review"] {
            connection
                .execute(
                    "UPDATE tasks SET work_state = ?1
                     WHERE id = '0198a7d5-a64e-7000-8000-000000000011'",
                    [new_state],
                )
                .unwrap_or_else(|error| panic!("estado {new_state} deveria ser aceito: {error}"));
        }

        // 2. link virou site, e a busca continua encontrando o recurso.
        let kind: String = connection
            .query_row("SELECT kind FROM resources", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kind, "site");

        let found = storage
            .search_resources(SearchRequest {
                query: "animacao".into(),
                include_archived: false,
                limit: 10,
            })
            .unwrap();
        assert_eq!(found.len(), 1, "o indice FTS de resources ficou obsoleto");
        assert_eq!(found[0].title, "Motion");

        // 3. Capacidades e repositorio.
        let can_open: i64 = connection
            .query_row("SELECT can_open FROM apps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            can_open, 1,
            "app com launch_target ja abre, e deve declarar isso"
        );

        let can_write: i64 = connection
            .query_row("SELECT can_write FROM apps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(can_write, 0);

        let repository: String = connection
            .query_row("SELECT repository FROM projects", [], |row| row.get(0))
            .unwrap();
        assert_eq!(repository, "");
    }

    /// A CHECK de url no banco espelha a validacao do dominio: uma Note nao tem
    /// url, e um Site sem http(s) e recusado nos dois lugares.
    #[test]
    fn resource_url_rules_follow_the_kind() {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        let connection = Connection::open(directory.path().join("mos.db")).unwrap();

        connection
            .execute(
                "INSERT INTO resources (id, kind, title, url, note, lifecycle_state,
                                        created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000020', 'note', 'Ideia solta', '',
                         'o motivo', 'active', '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .expect("note sem url deve ser aceita");

        connection
            .execute(
                "INSERT INTO resources (id, kind, title, url, note, lifecycle_state,
                                        created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000021', 'site', 'Sem esquema',
                         'motion.dev', '', 'active',
                         '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z')",
                [],
            )
            .expect_err("site sem http(s) deve ser recusado pelo banco");

        drop(storage);
    }

    #[test]
    fn upgrades_populated_v4_through_app_sources_and_resources() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();
        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        connection.execute_batch(MIGRATION_001).unwrap();
        connection.execute_batch(MIGRATION_002).unwrap();
        connection.execute_batch(MIGRATION_003).unwrap();
        connection.execute_batch(MIGRATION_004).unwrap();
        connection
            .execute(
                "INSERT INTO apps (
                    id, name, description, launch_kind, launch_target,
                    lifecycle_state, created_at, updated_at
                 ) VALUES (
                    '0198a7d5-a64e-7000-8000-000000000004', 'Motion',
                    'Biblioteca de animacao', 'url', 'https://motion.dev',
                    'active', '2026-08-13T00:00:00Z', '2026-08-13T00:00:00Z'
                 )",
                [],
            )
            .unwrap();
        drop(connection);

        let storage = SqliteStorage::open(&database, &backups).unwrap();

        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        let apps = storage.apps(false).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Motion");
        assert_eq!(apps[0].source_url, None);
        assert_eq!(
            storage
                .search_apps(SearchRequest {
                    query: "animacao".into(),
                    include_archived: false,
                    limit: 10,
                })
                .unwrap()
                .len(),
            1
        );

        let resource = storage
            .create_resource(
                NewResource::create_link(
                    "SQLite FTS5",
                    "https://sqlite.org/fts5.html",
                    "Busca local depois da migration",
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(
            storage
                .search_resources(SearchRequest {
                    query: "migration".into(),
                    include_archived: false,
                    limit: 10,
                })
                .unwrap()[0]
                .id,
            resource.id
        );
        assert_eq!(
            fs::read_dir(&backups)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("pre-migration-v4-"))
                .count(),
            1
        );
    }

    /// Sobe um v22 POVOADO ate a v23 — o teste mais perigoso desta feature.
    ///
    /// A 0023 recria `captures` e `resources`, e as duas TEM FILHAS: uma Task e
    /// um Resource derivados apontam para a Capture, e um vinculo de Workspace
    /// aponta para o Resource. Com `foreign_keys` ligado, o DROP da tabela
    /// antiga dispararia RESTRICT (na Task) e CASCADE (no vinculo) — o primeiro
    /// quebraria a migration, e o segundo apagaria contexto em silencio, que e
    /// pior.
    ///
    /// O teste prende as duas pontas: nada se perde, e nada fica orfao.
    #[test]
    fn upgrades_populated_v22_without_breaking_provenance() {
        use mos_core::{
            CaptureSource, IngestionRepository, LifecycleState, NewCapture, NewIngestion,
            NewProject, ResourceKind, WorkRepository,
        };

        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("mos.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();
        let connection = Connection::open(&database).unwrap();
        configure_connection(&connection).unwrap();
        for migration in [
            MIGRATION_001,
            MIGRATION_002,
            MIGRATION_003,
            MIGRATION_004,
            MIGRATION_005,
            MIGRATION_006,
            MIGRATION_007,
            MIGRATION_008,
            MIGRATION_009,
            MIGRATION_010,
            MIGRATION_011,
            MIGRATION_012,
            MIGRATION_013,
            MIGRATION_014,
            MIGRATION_015,
            MIGRATION_016,
            MIGRATION_017,
            MIGRATION_018,
            MIGRATION_019,
            MIGRATION_020,
            MIGRATION_021,
            MIGRATION_022,
        ] {
            connection.execute_batch(migration).unwrap();
        }
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 22, "o banco de partida precisa estar na v22");

        connection
            .execute_batch(
                "INSERT INTO captures (id, content, source_kind, processing_state,
                                       lifecycle_state, captured_at, created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000001', 'Virou Task', 'home',
                         'processed', 'active', '2026-08-19T00:00:00Z',
                         '2026-08-19T00:00:00Z', '2026-08-19T00:00:00Z'),
                        ('0198a7d5-a64e-7000-8000-000000000002', 'Virou Resource',
                         'quick_capture', 'processed', 'active', '2026-08-19T00:00:00Z',
                         '2026-08-19T00:00:00Z', '2026-08-19T00:00:00Z');

                 INSERT INTO tasks (id, title, description, source_capture_id, work_state,
                                    lifecycle_state, created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000011', 'Refatorar navbar', '',
                         '0198a7d5-a64e-7000-8000-000000000001', 'doing', 'active',
                         '2026-08-19T00:00:00Z', '2026-08-19T00:00:00Z');

                 INSERT INTO resources (id, kind, title, url, note, source_capture_id,
                                        lifecycle_state, created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000012', 'site', 'Motion',
                         'https://motion.dev', 'animacao declarativa',
                         '0198a7d5-a64e-7000-8000-000000000002', 'active',
                         '2026-08-19T00:00:00Z', '2026-08-19T00:00:00Z');

                 INSERT INTO workspaces (id, name, description, lifecycle_state,
                                         created_at, updated_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000013', 'Web Design', '', 'active',
                         '2026-08-19T00:00:00Z', '2026-08-19T00:00:00Z');

                 INSERT INTO resource_workspaces (resource_id, workspace_id, created_at)
                 VALUES ('0198a7d5-a64e-7000-8000-000000000012',
                         '0198a7d5-a64e-7000-8000-000000000013', '2026-08-19T00:00:00Z');",
            )
            .unwrap();
        drop(connection);

        let storage = SqliteStorage::open(&database, &backups).unwrap();
        assert_eq!(storage.health().unwrap().schema_version, SCHEMA_VERSION);
        assert_eq!(storage.health().unwrap().integrity, "ok");

        // 1. Nada se perdeu, e a proveniencia continua ligada dos dois lados.
        let connection = Connection::open(&database).unwrap();
        let orphans: i64 = connection
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(orphans, 0, "a migration deixou referencia orfa");
        let capture_of_task: String = connection
            .query_row(
                "SELECT source_capture_id FROM tasks
                 WHERE id = '0198a7d5-a64e-7000-8000-000000000011'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(capture_of_task, "0198a7d5-a64e-7000-8000-000000000001");
        let links: i64 = connection
            .query_row("SELECT count(*) FROM resource_workspaces", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(links, 1, "o CASCADE apagou um vinculo que era para ficar");

        // 2. A busca continua achando o que ja estava indexado. Depois do swap
        //    os rowids mudam, e sem o rebuild explicito a FTS devolveria o item
        //    errado — ou nenhum — sem que a contagem denunciasse.
        assert_eq!(
            CaptureRepository::search(
                &storage,
                SearchRequest {
                    query: "Resource".into(),
                    include_archived: false,
                    limit: 10,
                },
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            ResourceRepository::search_resources(
                &storage,
                SearchRequest {
                    query: "animacao".into(),
                    include_archived: false,
                    limit: 10,
                },
            )
            .unwrap()
            .len(),
            1
        );

        // 3. E o que a migration veio trazer funciona: origem 'drop', tipo
        //    'file' e a relacao com Project.
        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        let ingestion = storage
            .begin_ingestion(
                NewIngestion::file("memorial.pdf", "application/pdf", 12, Default::default())
                    .unwrap(),
                NewCapture::create("Arquivo recebido: memorial.pdf", CaptureSource::Drop).unwrap(),
            )
            .unwrap();
        let resource = storage
            .create_resource(
                NewResource::create(ResourceKind::File, "memorial.pdf", "", "", None).unwrap(),
            )
            .unwrap();
        storage
            .set_resource_project(resource.id, project.id, true)
            .unwrap();
        assert_eq!(storage.resource_projects().unwrap().len(), 1);
        assert_eq!(
            storage.get_ingestion(ingestion.id).unwrap().state,
            mos_core::IngestionState::Receiving
        );
        assert_eq!(
            storage
                .set_resource_lifecycle(resource.id, LifecycleState::Archived)
                .unwrap()
                .lifecycle_state,
            LifecycleState::Archived
        );
    }

    /// Ensaia a migration contra o banco REAL, numa cópia.
    ///
    /// Mesmo padrão do ensaio de importação do CronoCAD: uma migration testada
    /// só contra bancos sintéticos foi testada contra o que o autor imaginou. O
    /// banco de verdade tem reuniões, conversas do Hermes, horas rastreadas e
    /// Captures que já derivaram Task — e é nele que uma migration ruim custa
    /// caro.
    ///
    /// Roda numa CÓPIA e nunca no original:
    ///
    /// ```text
    /// MOS_DB=%APPDATA%/com.codedbym.mos/m-os.db \
    ///   cargo test -p mos-storage-sqlite ensaio -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "depende de um m-os.db real"]
    fn ensaio_de_migration_contra_o_banco_real() {
        use mos_core::WorkRepository;

        let Ok(origem) = std::env::var("MOS_DB") else {
            panic!("defina MOS_DB com o caminho do m-os.db");
        };
        let origem = std::path::PathBuf::from(origem);
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("m-os.db");
        let backups = directory.path().join("backups");
        fs::create_dir_all(&backups).unwrap();

        // A cópia sai pela API de backup do SQLite, e não por `fs::copy`.
        //
        // O `ARCHITECTURE.md` §16 já diz por quê: copiar o arquivo principal
        // enquanto o WAL está ativo produz um banco sem os últimos commits, e
        // copiar o `-shm` junto produz coisa pior — um índice de WAL que não
        // corresponde ao arquivo, e o SQLite passa a ler versões erradas de
        // página em silêncio.
        //
        // A origem é aberta SOMENTE LEITURA. Este ensaio nunca escreve no banco
        // de verdade, nem para migrar.
        {
            let fonte = Connection::open_with_flags(
                &origem,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            fonte.backup(MAIN_DB, &database, None).unwrap();
        }

        let antes = Connection::open(&database).unwrap();
        let versao: u32 = antes
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let captures: i64 = antes
            .query_row("SELECT count(*) FROM captures", [], |row| row.get(0))
            .unwrap();
        let resources: i64 = antes
            .query_row("SELECT count(*) FROM resources", [], |row| row.get(0))
            .unwrap();
        let tasks: i64 = antes
            .query_row("SELECT count(*) FROM tasks", [], |row| row.get(0))
            .unwrap();
        let vinculos: i64 = antes
            .query_row("SELECT count(*) FROM resource_workspaces", [], |row| {
                row.get(0)
            })
            .unwrap();
        drop(antes);
        println!(
            "antes: v{versao} · {captures} captures · {tasks} tasks · {resources} resources · {vinculos} vinculos"
        );

        let storage = SqliteStorage::open(&database, &backups).unwrap();
        let health = storage.health().unwrap();
        assert_eq!(health.schema_version, SCHEMA_VERSION);
        assert_eq!(health.integrity, "ok");

        let depois = Connection::open(&database).unwrap();
        let orfaos: i64 = depois
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(orfaos, 0, "a migration deixou referencia orfa");
        for (tabela, esperado) in [
            ("captures", captures),
            ("tasks", tasks),
            ("resources", resources),
            ("resource_workspaces", vinculos),
        ] {
            let agora: i64 = depois
                .query_row(&format!("SELECT count(*) FROM {tabela}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(agora, esperado, "{tabela} perdeu linha na migration");
        }
        // A busca continua respondendo depois do swap de rowid.
        let indexadas: i64 = depois
            .query_row("SELECT count(*) FROM capture_search", [], |row| row.get(0))
            .unwrap();
        assert_eq!(indexadas, captures, "o indice de Captures nao foi refeito");
        println!(
            "depois: v{} · {} projects legiveis · 0 orfaos",
            health.schema_version,
            storage.projects(true).unwrap().len()
        );
    }
}
