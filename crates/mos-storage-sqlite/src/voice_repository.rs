//! Persistencia do Voice Inbox.
//!
//! **Nao existe `update_transcript` aqui, e a ausencia e a regra.** A
//! transcricao entra uma vez, junto com a Capture, dentro de `capture_note`.
//! Um caminho que a reescrevesse destruiria a unica garantia que a feature
//! promete — a de que o que foi dito continua sendo o que esta gravado.

use mos_core::{
    Capture, CaptureId, CoreError, ErrorCode, NewCapture, NewVoiceNote, ProjectId, TaskId,
    VoiceNote, VoiceNoteId, VoiceNoteStatus, VoiceRepository,
};
use rusqlite::{params, Connection, Row};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time, query_capture},
    SqliteStorage,
};

const VOICE_COLUMNS: &str = "id, status, audio_dir, duration_ms, peak_level, transcript, \
     provider, capture_id, context_project_id, context_task_id, failure_message, \
     audio_deleted_at, started_at, updated_at";

fn read_note(row: &Row<'_>) -> rusqlite::Result<Result<VoiceNote, CoreError>> {
    let id: String = row.get(0)?;
    let status: String = row.get(1)?;
    let audio_dir: String = row.get(2)?;
    let duration_ms: i64 = row.get(3)?;
    let peak_level: i64 = row.get(4)?;
    let transcript: String = row.get(5)?;
    let provider: String = row.get(6)?;
    let capture_id: Option<String> = row.get(7)?;
    let context_project_id: Option<String> = row.get(8)?;
    let context_task_id: Option<String> = row.get(9)?;
    let failure_message: String = row.get(10)?;
    let audio_deleted_at: Option<String> = row.get(11)?;
    let started_at: String = row.get(12)?;
    let updated_at: String = row.get(13)?;

    Ok((|| {
        Ok(VoiceNote {
            id: VoiceNoteId::parse(&id)?,
            status: VoiceNoteStatus::parse(&status)?,
            audio_dir,
            duration_ms,
            // O banco guarda INTEGER e o dominio le u64. Um valor negativo aqui
            // seria corrupcao, e vira zero em vez de estourar: um pico
            // impossivel nao pode impedir a nota de ser lida.
            peak_level: peak_level.max(0) as u64,
            transcript,
            provider,
            capture_id: capture_id.as_deref().map(CaptureId::parse).transpose()?,
            context_project_id: context_project_id
                .as_deref()
                .map(ProjectId::parse)
                .transpose()?,
            context_task_id: context_task_id.as_deref().map(TaskId::parse).transpose()?,
            failure_message,
            audio_deleted_at: audio_deleted_at.as_deref().map(parse_time).transpose()?,
            started_at: parse_time(&started_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}

fn query_note(connection: &Connection, id: VoiceNoteId) -> Result<VoiceNote, CoreError> {
    connection
        .query_row(
            &format!("SELECT {VOICE_COLUMNS} FROM voice_notes WHERE id = ?1"),
            params![id.to_string()],
            read_note,
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                CoreError::new(ErrorCode::NotFound, "Nota de voz nao encontrada.", false)
            }
            other => map_sql_error(other),
        })?
}

/// Grava a nota inteira. Uma escrita por transicao, nunca por campo.
fn write_note(connection: &Connection, note: &VoiceNote) -> Result<usize, CoreError> {
    connection
        .execute(
            "UPDATE voice_notes SET status = ?2, duration_ms = ?3, peak_level = ?4, \
             transcript = ?5, provider = ?6, capture_id = ?7, failure_message = ?8, \
             updated_at = ?9 WHERE id = ?1",
            params![
                note.id.to_string(),
                note.status.as_str(),
                note.duration_ms,
                note.peak_level as i64,
                note.transcript,
                note.provider,
                note.capture_id.map(|id| id.to_string()),
                note.failure_message,
                format_time(note.updated_at)?,
            ],
        )
        .map_err(map_sql_error)
}

impl VoiceRepository for SqliteStorage {
    fn create_note(&self, note: NewVoiceNote) -> Result<VoiceNote, CoreError> {
        let id = note.id;
        let now = format_time(note.started_at)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "INSERT INTO voice_notes (
                    id, status, audio_dir, context_project_id, context_task_id,
                    started_at, created_at, updated_at
                 ) VALUES (?1, 'recording', ?2, ?3, ?4, ?5, ?5, ?5)",
                params![
                    id.to_string(),
                    note.audio_dir,
                    note.context_project_id.map(|value| value.to_string()),
                    note.context_task_id.map(|value| value.to_string()),
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        query_note(&connection, id)
    }

    fn note(&self, id: VoiceNoteId) -> Result<VoiceNote, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_note(&connection, id)
    }

    fn save_note(&self, note: &VoiceNote) -> Result<VoiceNote, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = write_note(&connection, note)?;
        if changed != 1 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Nota de voz nao encontrada.",
                false,
            ));
        }
        query_note(&connection, note.id)
    }

    fn unfinished_notes(&self) -> Result<Vec<VoiceNote>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {VOICE_COLUMNS} FROM voice_notes \
                 WHERE status IN ('recording', 'recorded', 'transcribing', 'failed') \
                 ORDER BY started_at DESC"
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], read_note)
            .map_err(map_sql_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_sql_error)?;
        rows.into_iter().collect()
    }

    fn capture_note(
        &self,
        note: &VoiceNote,
        capture: NewCapture,
    ) -> Result<(VoiceNote, Capture), CoreError> {
        // O dominio ja decidiu; o adapter so confere que quem chamou nao
        // inverteu as duas metades. Sem esta guarda, uma nota apontando para uma
        // Capture diferente da que esta sendo inserida passaria em silencio, e a
        // proveniencia mentiria para sempre.
        if note.capture_id != Some(capture.id) {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "A nota de voz nao aponta para a Capture que esta sendo criada.",
                false,
            ));
        }

        let capture_id = capture.id;
        let now = format_time(capture.captured_at)?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        transaction
            .execute(
                "INSERT INTO captures (
                    id, content, source_kind, processing_state, lifecycle_state,
                    captured_at, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, 'inbox', 'active', ?4, ?4, ?4)",
                params![
                    capture_id.to_string(),
                    capture.content,
                    capture.source.as_str(),
                    now
                ],
            )
            .map_err(map_sql_error)?;
        let rowid = transaction.last_insert_rowid();
        // A projecao de busca entra na MESMA transacao. Uma Capture que existisse
        // sem estar no indice seria uma fala que o Search nao encontra, e o
        // sintoma so apareceria semanas depois, procurando por ela.
        transaction
            .execute(
                "INSERT INTO capture_search (rowid, content)
                 SELECT rowid, content FROM captures WHERE rowid = ?1",
                [rowid],
            )
            .map_err(map_sql_error)?;

        let changed = write_note(&transaction, note)?;
        if changed != 1 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Nota de voz nao encontrada.",
                false,
            ));
        }
        transaction.commit().map_err(map_sql_error)?;

        let stored_note = query_note(&connection, note.id)?;
        let stored_capture = query_capture(&connection, capture_id)?;
        Ok((stored_note, stored_capture))
    }

    fn mark_audio_deleted(
        &self,
        id: VoiceNoteId,
        at: OffsetDateTime,
    ) -> Result<VoiceNote, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let stamp = format_time(at)?;
        let changed = connection
            .execute(
                "UPDATE voice_notes SET audio_deleted_at = ?2, updated_at = ?2 \
                 WHERE id = ?1 AND audio_deleted_at IS NULL",
                params![id.to_string(), stamp],
            )
            .map_err(map_sql_error)?;
        if changed != 1 {
            // Ja marcada nao e erro: apagar duas vezes o mesmo diretorio e o
            // desfecho normal de uma limpeza que rodou junto de um retry.
            return query_note(&connection, id);
        }
        query_note(&connection, id)
    }

    fn delete_note(&self, id: VoiceNoteId) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let note = query_note(&connection, id)?;
        if note.capture_id.is_some() {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Esta nota ja virou Capture e nao pode ser apagada.",
                false,
            ));
        }
        connection
            .execute("DELETE FROM voice_notes WHERE id = ?1", params![id.to_string()])
            .map_err(map_sql_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{
        apply_voice, CaptureRepository, CaptureSource, VoiceTransition,
    };

    fn storage() -> (SqliteStorage, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (storage, directory)
    }

    fn agora() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).unwrap()
    }

    fn nota(storage: &SqliteStorage) -> VoiceNote {
        storage
            .create_note(NewVoiceNote::create(agora(), None, None))
            .unwrap()
    }

    fn gravada(storage: &SqliteStorage) -> VoiceNote {
        let note = nota(storage);
        let recorded = apply_voice(
            &note,
            VoiceTransition::Recorded {
                duration_ms: 4_200,
                peak_level: 800,
            },
            agora(),
        )
        .unwrap();
        let saved = storage.save_note(&recorded).unwrap();
        let transcribing =
            apply_voice(&saved, VoiceTransition::Transcribing, agora()).unwrap();
        storage.save_note(&transcribing).unwrap()
    }

    #[test]
    fn a_nota_nasce_gravando_com_diretorio_derivado_do_id() {
        let (storage, _guard) = storage();
        let note = nota(&storage);
        assert_eq!(note.status, VoiceNoteStatus::Recording);
        assert_eq!(note.audio_dir, format!("voice/{}", note.id));
        assert!(note.status.audio_still_needed());
    }

    #[test]
    fn a_capture_de_voz_nasce_na_inbox_e_ja_no_indice_de_busca() {
        let (storage, _guard) = storage();
        let note = gravada(&storage);

        let capture = NewCapture::create(
            "me lembra amanha as nove de revisar o memorial",
            CaptureSource::Voice,
        )
        .unwrap();
        let closed = apply_voice(
            &note,
            VoiceTransition::Captured {
                capture_id: capture.id,
                transcript: "me lembra amanha as nove de revisar o memorial".into(),
                provider: "whisper.cpp · large-v3-turbo".into(),
            },
            agora(),
        )
        .unwrap();
        let (stored_note, stored_capture) = storage.capture_note(&closed, capture).unwrap();

        assert_eq!(stored_note.status, VoiceNoteStatus::Captured);
        assert_eq!(stored_note.capture_id, Some(stored_capture.id));
        assert_eq!(stored_capture.source, CaptureSource::Voice);
        assert_eq!(
            stored_capture.processing_state,
            mos_core::ProcessingState::Inbox
        );

        // E o Search ja a encontra — na mesma transacao, sem rebuild.
        let encontrado = storage
            .search(mos_core::SearchRequest {
                query: "memorial".into(),
                include_archived: false,
                limit: 10,
            })
            .unwrap();
        assert_eq!(encontrado.len(), 1);
        assert_eq!(encontrado[0].id, stored_capture.id);
    }

    #[test]
    fn uma_nota_nao_pode_apontar_para_outra_capture() {
        let (storage, _guard) = storage();
        let note = gravada(&storage);
        let capture = NewCapture::create("um texto", CaptureSource::Voice).unwrap();
        // A nota fecha sobre uma Capture, e outra e passada para gravar.
        let closed = apply_voice(
            &note,
            VoiceTransition::Captured {
                capture_id: CaptureId::new(),
                transcript: "um texto".into(),
                provider: String::new(),
            },
            agora(),
        )
        .unwrap();
        let erro = storage.capture_note(&closed, capture).unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn a_reconciliacao_encontra_o_que_ficou_pelo_caminho() {
        let (storage, _guard) = storage();
        // Uma gravando (o processo morreu), uma falhada (espera retry) e uma
        // que virou Capture (nao interessa mais).
        let _gravando = nota(&storage);

        let pendente = gravada(&storage);
        let falhou = apply_voice(
            &pendente,
            VoiceTransition::Failed {
                message: "o transcritor nao esta configurado".into(),
            },
            agora(),
        )
        .unwrap();
        storage.save_note(&falhou).unwrap();

        let pronta = gravada(&storage);
        let capture = NewCapture::create("ja virou texto", CaptureSource::Voice).unwrap();
        let closed = apply_voice(
            &pronta,
            VoiceTransition::Captured {
                capture_id: capture.id,
                transcript: "ja virou texto".into(),
                provider: String::new(),
            },
            agora(),
        )
        .unwrap();
        storage.capture_note(&closed, capture).unwrap();

        let pendentes = storage.unfinished_notes().unwrap();
        assert_eq!(pendentes.len(), 2);
        assert!(pendentes
            .iter()
            .all(|note| note.status.audio_still_needed()));
    }

    #[test]
    fn apagar_o_audio_duas_vezes_nao_e_erro() {
        let (storage, _guard) = storage();
        let note = gravada(&storage);
        let primeira = storage.mark_audio_deleted(note.id, agora()).unwrap();
        assert!(primeira.audio_deleted_at.is_some());
        let segunda = storage.mark_audio_deleted(note.id, agora()).unwrap();
        assert_eq!(segunda.audio_deleted_at, primeira.audio_deleted_at);
    }

    #[test]
    fn uma_nota_que_virou_capture_nao_pode_ser_apagada() {
        let (storage, _guard) = storage();
        let note = gravada(&storage);
        let capture = NewCapture::create("um pensamento", CaptureSource::Voice).unwrap();
        let closed = apply_voice(
            &note,
            VoiceTransition::Captured {
                capture_id: capture.id,
                transcript: "um pensamento".into(),
                provider: String::new(),
            },
            agora(),
        )
        .unwrap();
        storage.capture_note(&closed, capture).unwrap();

        let erro = storage.delete_note(note.id).unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidTransition);
    }

    #[test]
    fn uma_nota_cancelada_some_do_banco() {
        let (storage, _guard) = storage();
        let note = nota(&storage);
        storage.delete_note(note.id).unwrap();
        assert_eq!(
            storage.note(note.id).unwrap_err().code,
            ErrorCode::NotFound
        );
    }
}
