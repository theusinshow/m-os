use mos_core::{
    Capture, CaptureId, CaptureRepository, CaptureSource, CoreError, ErrorCode, LifecycleState,
    NewCapture, ProcessingState, SearchRequest,
};
use rusqlite::{params, OptionalExtension, Row};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{map_lock_error, map_sql_error, rebuild_search_projection, SqliteStorage};

pub(crate) struct RawCapture {
    id: String,
    content: String,
    source: String,
    processing_state: String,
    lifecycle_state: String,
    captured_at: String,
    updated_at: String,
}

impl RawCapture {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            content: row.get(1)?,
            source: row.get(2)?,
            processing_state: row.get(3)?,
            lifecycle_state: row.get(4)?,
            captured_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }

    pub(crate) fn into_capture(self) -> Result<Capture, CoreError> {
        Ok(Capture {
            id: CaptureId::parse(&self.id)?,
            content: self.content,
            source: CaptureSource::parse(&self.source)?,
            processing_state: ProcessingState::parse(&self.processing_state)?,
            lifecycle_state: LifecycleState::parse(&self.lifecycle_state)?,
            captured_at: parse_time(&self.captured_at)?,
            updated_at: parse_time(&self.updated_at)?,
        })
    }
}

pub(crate) const CAPTURE_COLUMNS: &str =
    "id, content, source_kind, processing_state, lifecycle_state, captured_at, updated_at";

impl CaptureRepository for SqliteStorage {
    fn create(&self, capture: NewCapture) -> Result<Capture, CoreError> {
        let now = format_time(capture.captured_at)?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        insert_capture(&transaction, &capture, &now)?;
        // A operacao entra na MESMA transacao. Gravar a Capture e falhar aqui
        // deixaria uma Capture que nunca sai deste dispositivo, e ninguem
        // ficaria sabendo. Ver `sync_emit.rs`.
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("capture", capture.id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    ("content".to_owned(), serde_json::json!(capture.content)),
                    (
                        "source".to_owned(),
                        serde_json::json!(capture.source.as_str()),
                    ),
                    (
                        "processingState".to_owned(),
                        serde_json::json!(ProcessingState::Inbox.as_str()),
                    ),
                    ("capturedAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;
        transaction.commit().map_err(map_sql_error)?;
        query_capture(&connection, capture.id)
    }

    fn get(&self, id: CaptureId) -> Result<Capture, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_capture(&connection, id)
    }

    fn recent(&self, limit: usize) -> Result<Vec<Capture>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_list(
            &connection,
            &format!(
                "SELECT {CAPTURE_COLUMNS} FROM captures
                 WHERE lifecycle_state = 'active'
                 ORDER BY captured_at DESC LIMIT ?1"
            ),
            [limit as i64],
        )
    }

    fn inbox(&self, limit: usize) -> Result<Vec<Capture>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_list(
            &connection,
            &format!(
                "SELECT {CAPTURE_COLUMNS} FROM captures
                 WHERE processing_state = 'inbox' AND lifecycle_state = 'active'
                 ORDER BY captured_at DESC LIMIT ?1"
            ),
            [limit as i64],
        )
    }

    fn captures_between(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
    ) -> Result<Vec<Capture>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // Crescente, ao contrario das outras leituras: o Calendario le o dia na
        // ordem em que ele aconteceu, e nao "o que houve por ultimo".
        let mut statement = connection
            .prepare(&format!(
                "SELECT {CAPTURE_COLUMNS} FROM captures
                 WHERE captured_at >= ?1 AND captured_at <= ?2
                 ORDER BY captured_at ASC"
            ))
            .map_err(map_sql_error)?;
        // O resultado e ligado antes de devolver, como nos vizinhos: devolver a
        // expressao direto mantem `statement` emprestado ate depois do retorno.
        let captures = collect_rows(
            statement
                .query_map(
                    params![format_time(since)?, format_time(until)?],
                    RawCapture::from_row,
                )
                .map_err(map_sql_error)?,
        )?;
        Ok(captures)
    }

    fn by_lifecycle(
        &self,
        lifecycle: LifecycleState,
        limit: usize,
    ) -> Result<Vec<Capture>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {CAPTURE_COLUMNS} FROM captures
                 WHERE lifecycle_state = ?1
                 ORDER BY captured_at DESC LIMIT ?2"
            ))
            .map_err(map_sql_error)?;
        let captures = collect_rows(
            statement
                .query_map(
                    params![lifecycle.as_str(), limit as i64],
                    RawCapture::from_row,
                )
                .map_err(map_sql_error)?,
        )?;
        Ok(captures)
    }

    fn search(&self, request: SearchRequest) -> Result<Vec<Capture>, CoreError> {
        if request.query.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = to_fts_query(&request.query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let lifecycle = if request.include_archived {
            "c.lifecycle_state IN ('active', 'archived')"
        } else {
            "c.lifecycle_state = 'active'"
        };
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT c.{columns}
                 FROM capture_search s
                 JOIN captures c ON c.rowid = s.rowid
                 WHERE capture_search MATCH ?1 AND {lifecycle}
                 ORDER BY bm25(capture_search), c.captured_at DESC
                 LIMIT ?2",
                columns = CAPTURE_COLUMNS.replace(", ", ", c.")
            ))
            .map_err(map_sql_error)?;
        let captures = collect_rows(
            statement
                .query_map(
                    params![fts_query, request.limit as i64],
                    RawCapture::from_row,
                )
                .map_err(map_sql_error)?,
        )?;
        Ok(captures)
    }

    fn set_processing_state(
        &self,
        id: CaptureId,
        state: ProcessingState,
    ) -> Result<Capture, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE captures SET processing_state = ?1, updated_at = ?2 WHERE id = ?3",
                params![state.as_str(), now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        self.emitir_update(
            &transaction,
            "capture",
            id.as_uuid(),
            &[("processingState", serde_json::json!(state.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        query_capture(&connection, id)
    }

    fn set_lifecycle_state(
        &self,
        id: CaptureId,
        state: LifecycleState,
    ) -> Result<Capture, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let (archived_at, deleted_at) = match state {
            LifecycleState::Active => (None, None),
            LifecycleState::Archived => (Some(now.as_str()), None),
            LifecycleState::Trashed => (None, Some(now.as_str())),
        };
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE captures
                 SET lifecycle_state = ?1, updated_at = ?2, archived_at = ?3, deleted_at = ?4
                 WHERE id = ?5",
                params![state.as_str(), now, archived_at, deleted_at, id.to_string()],
            )
            .map_err(map_sql_error)?;
        ensure_changed(changed)?;
        // Arquivar e mandar para a lixeira sao MUDANCA DE CAMPO, e nao
        // `OpBody::Delete`. O `Delete` do sync e para exclusao definitiva; o
        // ciclo de vida do M/OS e um campo, e reconcilia como qualquer outro —
        // arquivar num dispositivo e restaurar no outro precisa ser decidido
        // pelo instante, e nao pela regra de "apagar ganha".
        self.emitir_update(
            &transaction,
            "capture",
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(state.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        query_capture(&connection, id)
    }

    /// Capture que virou Task ou Resource NAO e apagavel. A FK ja recusaria
    /// (ON DELETE RESTRICT em 0007_v03_design.sql:31 e :94), mas o erro do
    /// SQLite diria "FOREIGN KEY constraint failed" — que nao ajuda ninguem.
    /// Aqui a recusa explica o motivo real: a proveniencia daquele item
    /// derivado deixaria de existir.
    fn delete_capture(&self, id: CaptureId) -> Result<(), CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        guard_deletable(&transaction, "captures", &id.to_string(), "Capture")?;
        let derived: i64 = transaction
            .query_row(
                "SELECT (SELECT count(*) FROM tasks WHERE source_capture_id = ?1)
                      + (SELECT count(*) FROM resources WHERE source_capture_id = ?1)",
                [id.to_string()],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        if derived > 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Esta Capture deu origem a uma Task ou Resource. Exclua o item derivado antes.",
                false,
            ));
        }
        transaction
            .execute(
                "INSERT INTO capture_search(capture_search, rowid, content)
                 SELECT 'delete', rowid, content FROM captures WHERE id = ?1",
                [id.to_string()],
            )
            .map_err(map_sql_error)?;
        transaction
            .execute("DELETE FROM captures WHERE id = ?1", [id.to_string()])
            .map_err(map_sql_error)?;
        // Aqui sim e `OpBody::Delete`: esta e a exclusao definitiva, a que o
        // M/OS so aceita depois de arquivar. O outro dispositivo precisa SABER
        // que sumiu — uma linha ausente e indistinguivel de uma que nunca
        // chegou.
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new("capture", id.as_uuid()),
            mos_sync::OpBody::Delete,
        )?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn rebuild_search(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        rebuild_search_projection(&connection)
    }
}

pub(crate) fn query_capture(
    connection: &rusqlite::Connection,
    id: CaptureId,
) -> Result<Capture, CoreError> {
    let raw = connection
        .query_row(
            &format!("SELECT {CAPTURE_COLUMNS} FROM captures WHERE id = ?1"),
            [id.to_string()],
            RawCapture::from_row,
        )
        .optional()
        .map_err(map_sql_error)?
        .ok_or_else(|| CoreError::new(ErrorCode::NotFound, "Capture nao encontrada.", false))?;
    raw.into_capture()
}

fn query_list(
    connection: &rusqlite::Connection,
    sql: &str,
    params: [i64; 1],
) -> Result<Vec<Capture>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let captures = collect_rows(
        statement
            .query_map(params, RawCapture::from_row)
            .map_err(map_sql_error)?,
    )?;
    Ok(captures)
}

fn collect_rows(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<RawCapture>>,
) -> Result<Vec<Capture>, CoreError> {
    rows.map(|row| row.map_err(map_sql_error)?.into_capture())
        .collect()
}

/// A regra de exclusao do M/OS, num lugar so: **nada ativo e apagado**.
///
/// Arquivar primeiro nao e burocracia — e o que garante que nenhuma exclusao
/// definitiva aconteca no meio do uso normal, sem que o item tenha passado
/// antes por um estado onde ja estava fora do caminho.
///
/// `table` vem sempre de literal do proprio crate, nunca de entrada do usuario:
/// e o unico motivo pelo qual interpolar o nome na SQL e aceitavel aqui.
pub(crate) fn guard_deletable(
    transaction: &rusqlite::Transaction<'_>,
    table: &str,
    id: &str,
    label: &str,
) -> Result<(), CoreError> {
    let lifecycle: Option<String> = transaction
        .query_row(
            &format!("SELECT lifecycle_state FROM {table} WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_sql_error)?;
    let lifecycle = lifecycle.ok_or_else(|| {
        CoreError::new(
            ErrorCode::NotFound,
            format!("{label} nao encontrado."),
            false,
        )
    })?;
    if lifecycle == "active" {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            format!("{label} ativo nao pode ser excluido. Arquive antes."),
            false,
        ));
    }
    Ok(())
}

pub(crate) fn ensure_changed(changed: usize) -> Result<(), CoreError> {
    if changed == 0 {
        Err(CoreError::new(
            ErrorCode::NotFound,
            "Capture nao encontrada.",
            false,
        ))
    } else {
        Ok(())
    }
}

/// Grava a Capture e sua projecao de busca DENTRO de uma transacao ja aberta.
///
/// A ingestao universal precisa da Capture no MESMO commit em que abre a linha
/// de ingestao: uma Capture sem ingestao seria um item orfao na Inbox, e uma
/// ingestao sem Capture seria bytes que ninguem sabe explicar.
pub(crate) fn insert_capture(
    transaction: &rusqlite::Transaction<'_>,
    capture: &NewCapture,
    now: &str,
) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO captures (
                id, content, source_kind, processing_state, lifecycle_state,
                captured_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, 'inbox', 'active', ?4, ?4, ?4)",
            params![
                capture.id.to_string(),
                capture.content,
                capture.source.as_str(),
                now
            ],
        )
        .map_err(map_sql_error)?;
    let rowid = transaction.last_insert_rowid();
    transaction
        .execute(
            "INSERT INTO capture_search (rowid, content)
             SELECT rowid, content FROM captures WHERE rowid = ?1",
            [rowid],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

pub(crate) fn format_time(value: OffsetDateTime) -> Result<String, CoreError> {
    value.format(&Rfc3339).map_err(|error| {
        CoreError::new(
            ErrorCode::DataIntegrity,
            format!("Falha ao formatar horario: {error}"),
            false,
        )
    })
}

pub(crate) fn parse_time(value: &str) -> Result<OffsetDateTime, CoreError> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|error| {
        CoreError::new(
            ErrorCode::DataIntegrity,
            format!("Horario persistido e invalido: {error}"),
            false,
        )
    })
}

pub(crate) fn to_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter_map(|token| {
            let token = token.replace('"', "\"\"");
            (!token.is_empty()).then(|| format!("\"{token}\"*"))
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{CaptureSource, NewCapture};

    fn storage() -> (tempfile::TempDir, SqliteStorage) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (directory, storage)
    }

    /// A janela e fechada nos dois lados.
    ///
    /// Sem ela o Calendario precisaria ler TODAS as Captures que existem para
    /// desenhar um mes, ou usar `recent`, que tem teto de 50 — e um calendario
    /// que para de mostrar Capture depois da quinquagesima fica errado em
    /// silencio.
    #[test]
    fn captures_between_respects_the_window() {
        let (_directory, storage) = storage();
        for content in ["antes", "dentro", "depois"] {
            storage
                .create(NewCapture::create(content, CaptureSource::Home).unwrap())
                .unwrap();
        }

        let now = time::OffsetDateTime::now_utc();
        let all = storage
            .captures_between(
                now - time::Duration::hours(1),
                now + time::Duration::hours(1),
            )
            .unwrap();
        assert_eq!(all.len(), 3, "as tres foram criadas agora");

        let none = storage
            .captures_between(now - time::Duration::days(9), now - time::Duration::days(8))
            .unwrap();
        assert!(none.is_empty(), "nenhuma foi criada semana passada");
    }

    #[test]
    fn create_and_search_commit_atomically() {
        let (_directory, storage) = storage();
        let capture = storage
            .create(NewCapture::create("Revisar ação do M/OS", CaptureSource::Home).unwrap())
            .unwrap();
        let results = storage
            .search(SearchRequest {
                query: "ação".into(),
                include_archived: false,
                limit: 20,
            })
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, capture.id);
    }

    #[test]
    fn fts_failure_rolls_back_capture() {
        let (_directory, storage) = storage();
        storage
            .connection
            .lock()
            .unwrap()
            .execute("DROP TABLE capture_search", [])
            .unwrap();

        assert!(storage
            .create(NewCapture::create("Nao persiste", CaptureSource::Home).unwrap())
            .is_err());
        let count: i64 = storage
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM captures", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn restore_lifecycle_preserves_processing_state() {
        let (_directory, storage) = storage();
        let capture = storage
            .create(NewCapture::create("Processar", CaptureSource::Home).unwrap())
            .unwrap();
        storage
            .set_processing_state(capture.id, ProcessingState::Processed)
            .unwrap();
        storage
            .set_lifecycle_state(capture.id, LifecycleState::Trashed)
            .unwrap();
        let restored = storage
            .set_lifecycle_state(capture.id, LifecycleState::Active)
            .unwrap();

        assert_eq!(restored.processing_state, ProcessingState::Processed);
        assert_eq!(restored.lifecycle_state, LifecycleState::Active);
    }

    #[test]
    fn rebuild_search_restores_the_derived_projection() {
        let (_directory, storage) = storage();
        storage
            .create(NewCapture::create("Indice reconstruivel", CaptureSource::Home).unwrap())
            .unwrap();
        storage
            .connection
            .lock()
            .unwrap()
            .execute("DELETE FROM capture_search", [])
            .unwrap();

        assert!(storage
            .search(SearchRequest {
                query: "Indice".into(),
                include_archived: false,
                limit: 20,
            })
            .unwrap()
            .is_empty());
        assert_eq!(storage.rebuild_search().unwrap(), 1);
        assert_eq!(
            storage
                .search(SearchRequest {
                    query: "Indice".into(),
                    include_archived: false,
                    limit: 20,
                })
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn write_lock_never_reports_a_capture_as_saved() {
        let (directory, storage) = storage();
        let blocker = rusqlite::Connection::open(directory.path().join("mos.db")).unwrap();
        blocker.execute_batch("BEGIN IMMEDIATE").unwrap();

        let error = storage
            .create(NewCapture::create("Nao confirmar", CaptureSource::Home).unwrap())
            .unwrap_err();
        blocker.execute_batch("ROLLBACK").unwrap();

        assert_eq!(error.code, ErrorCode::StorageBusy);
        assert!(error.retryable);
        assert!(storage.recent(10).unwrap().is_empty());
    }

    #[test]
    fn full_database_rolls_back_capture_and_search_projection() {
        let (_directory, storage) = storage();
        {
            let connection = storage.connection.lock().unwrap();
            let pages: i64 = connection
                .query_row("PRAGMA page_count", [], |row| row.get(0))
                .unwrap();
            connection
                .pragma_update(None, "max_page_count", pages)
                .unwrap();
        }

        let content = "x".repeat(1024 * 1024);
        let error = storage
            .create(NewCapture::create(&content, CaptureSource::Home).unwrap())
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::StorageUnavailable);
        assert!(storage.recent(10).unwrap().is_empty());
    }
}
