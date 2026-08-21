//! Persistencia do Meeting Agent.
//!
//! Duas coisas aqui merecem ser lidas antes do codigo.
//!
//! **A duracao nunca e recalculada.** Ela chega medida em frames gravados e e
//! gravada como veio. Um repositorio que a derivasse de `ended_at - started_at`
//! mentiria exatamente no caso em que a verdade importa: quando um canal caiu e
//! parte do audio nao existe.
//!
//! **Reanalisar preserva o que foi aceito.** `replace_analysis` troca resumo e
//! itens, mas nao encosta em item ja `accepted` — desfazer uma Task que a pessoa
//! criou seria apagar trabalho real por causa de uma segunda opiniao do modelo.

use mos_core::{
    AudioRetention, ChannelOutcome, Confidence, CoreError, ErrorCode, InsightId, InsightKind,
    InsightStatus, LifecycleState, Meeting, MeetingAnalysis, MeetingChannel, MeetingEvidence,
    MeetingFailure, MeetingId, MeetingInsight, MeetingRepository, MeetingSource, MeetingStatus,
    NewMeeting, ProjectId, ReminderId, SearchRequest, SegmentId, TaskId, TranscriptSegment,
};
use rusqlite::{params, Connection, Row};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time, to_fts_query},
    SqliteStorage,
};

const MEETING_COLUMNS: &str = "id, title, status, failed_stage, failure_message, \
     lifecycle_state, source, started_at, ended_at, duration_ms, project_id, audio_dir, \
     retention, audio_deleted_at, mic_state, mic_lost_at_ms, mic_reason, system_state, \
     system_lost_at_ms, system_reason, created_at, updated_at, cancelled_at, notes";

const INSIGHT_COLUMNS: &str = "id, meeting_id, kind, seq, text, owner, due_hint, confidence, \
     status, created_task_id, created_reminder_id";

/// As mesmas colunas com o alias `i.`, para as consultas que fazem JOIN.
///
/// Sem elas, `id` fica ambiguo entre `meeting_insights` e `meetings` e o SQLite
/// recusa a consulta inteira — o que e melhor que escolher a coluna errada em
/// silencio, mas ainda assim precisa ser resolvido aqui.
const INSIGHT_COLUMNS_QUALIFIED: &str = "i.id, i.meeting_id, i.kind, i.seq, i.text, i.owner, \
     i.due_hint, i.confidence, i.status, i.created_task_id, i.created_reminder_id";

fn read_meeting(row: &Row<'_>) -> rusqlite::Result<Result<Meeting, CoreError>> {
    let id: String = row.get(0)?;
    let title: String = row.get(1)?;
    let status: String = row.get(2)?;
    let failed_stage: Option<String> = row.get(3)?;
    let failure_message: Option<String> = row.get(4)?;
    let lifecycle: String = row.get(5)?;
    let source: String = row.get(6)?;
    let started_at: String = row.get(7)?;
    let ended_at: Option<String> = row.get(8)?;
    let duration_ms: i64 = row.get(9)?;
    let project_id: Option<String> = row.get(10)?;
    let audio_dir: String = row.get(11)?;
    let retention: String = row.get(12)?;
    let audio_deleted_at: Option<String> = row.get(13)?;
    let mic_state: String = row.get(14)?;
    let mic_lost_at_ms: Option<i64> = row.get(15)?;
    let mic_reason: Option<String> = row.get(16)?;
    let system_state: String = row.get(17)?;
    let system_lost_at_ms: Option<i64> = row.get(18)?;
    let system_reason: Option<String> = row.get(19)?;
    let created_at: String = row.get(20)?;
    let updated_at: String = row.get(21)?;
    let cancelled_at: Option<String> = row.get(22)?;
    let notes: String = row.get(23)?;

    Ok((|| {
        let status = MeetingStatus::from_columns(&status, failed_stage.as_deref())?;
        Ok(Meeting {
            id: MeetingId::parse(&id)?,
            title,
            status,
            lifecycle_state: LifecycleState::parse(&lifecycle)?,
            source: MeetingSource::parse(&source)?,
            started_at: parse_time(&started_at)?,
            ended_at: ended_at.as_deref().map(parse_time).transpose()?,
            duration_ms,
            project_id: project_id.as_deref().map(ProjectId::parse).transpose()?,
            audio_dir,
            retention: AudioRetention::parse(&retention)?,
            audio_deleted_at: audio_deleted_at.as_deref().map(parse_time).transpose()?,
            mic: ChannelOutcome::from_columns(&mic_state, mic_lost_at_ms, mic_reason)?,
            system: ChannelOutcome::from_columns(&system_state, system_lost_at_ms, system_reason)?,
            failure: match (status, failure_message) {
                (MeetingStatus::Failed(stage), message) => Some(MeetingFailure {
                    stage,
                    message: message.unwrap_or_default(),
                }),
                _ => None,
            },
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
            cancelled_at: cancelled_at.as_deref().map(parse_time).transpose()?,
            notes,
        })
    })())
}

fn read_insight(row: &Row<'_>) -> rusqlite::Result<Result<MeetingInsight, CoreError>> {
    let id: String = row.get(0)?;
    let meeting_id: String = row.get(1)?;
    let kind: String = row.get(2)?;
    let seq: i64 = row.get(3)?;
    let text: String = row.get(4)?;
    let owner: Option<String> = row.get(5)?;
    let due_hint: Option<String> = row.get(6)?;
    let confidence: String = row.get(7)?;
    let status: String = row.get(8)?;
    let task_id: Option<String> = row.get(9)?;
    let reminder_id: Option<String> = row.get(10)?;

    Ok((|| {
        Ok(MeetingInsight {
            id: InsightId::parse(&id)?,
            meeting_id: MeetingId::parse(&meeting_id)?,
            kind: InsightKind::parse(&kind)?,
            seq,
            text,
            owner,
            due_hint,
            confidence: Confidence::parse(&confidence)?,
            status: InsightStatus::parse(&status)?,
            created_task_id: task_id.as_deref().map(TaskId::parse).transpose()?,
            created_reminder_id: reminder_id.as_deref().map(ReminderId::parse).transpose()?,
            // Preenchida por `load_evidence`. Vazia aqui porque uma consulta por
            // item seria N+1, e a evidencia e lida em bloco.
            evidence: Vec::new(),
        })
    })())
}

/// Carrega a evidencia de varios itens numa consulta so.
fn load_evidence(
    connection: &Connection,
    insights: &mut [MeetingInsight],
) -> Result<(), CoreError> {
    if insights.is_empty() {
        return Ok(());
    }
    let mut statement = connection
        .prepare(
            "SELECT e.insight_id, e.segment_id, e.seq, e.char_start, e.char_end \
             FROM meeting_evidence e \
             JOIN meeting_insights i ON i.id = e.insight_id \
             WHERE i.meeting_id = ?1 ORDER BY e.insight_id, e.seq",
        )
        .map_err(map_sql_error)?;

    let meeting_id = insights[0].meeting_id.to_string();
    let rows = statement
        .query_map(params![meeting_id], |row| {
            let insight_id: String = row.get(0)?;
            let segment_id: String = row.get(1)?;
            let seq: i64 = row.get(2)?;
            let char_start: Option<i64> = row.get(3)?;
            let char_end: Option<i64> = row.get(4)?;
            Ok((insight_id, segment_id, seq, char_start, char_end))
        })
        .map_err(map_sql_error)?;

    for row in rows {
        let (insight_id, segment_id, seq, char_start, char_end) = row.map_err(map_sql_error)?;
        let insight_id = InsightId::parse(&insight_id)?;
        if let Some(insight) = insights.iter_mut().find(|item| item.id == insight_id) {
            insight.evidence.push(MeetingEvidence {
                segment_id: SegmentId::parse(&segment_id)?,
                seq,
                char_start: char_start.map(|value| value as u32),
                char_end: char_end.map(|value| value as u32),
            });
        }
    }
    Ok(())
}

impl SqliteStorage {
    fn meeting_by_id(&self, connection: &Connection, id: MeetingId) -> Result<Meeting, CoreError> {
        connection
            .query_row(
                &format!("SELECT {MEETING_COLUMNS} FROM meetings WHERE id = ?1"),
                params![id.to_string()],
                read_meeting,
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    CoreError::new(ErrorCode::NotFound, "Reuniao nao encontrada.", false)
                }
                other => map_sql_error(other),
            })?
    }
}

impl MeetingRepository for SqliteStorage {
    fn create_meeting(&self, meeting: NewMeeting) -> Result<Meeting, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let now = format_time(meeting.started_at)?;
        connection
            .execute(
                "INSERT INTO meetings (id, title, status, lifecycle_state, source, started_at, \
                 duration_ms, project_id, audio_dir, retention, mic_state, system_state, \
                 created_at, updated_at) \
                 VALUES (?1, ?2, 'recording', 'active', ?3, ?4, 0, ?5, ?6, ?7, 'capturing', \
                 'capturing', ?4, ?4)",
                params![
                    meeting.id.to_string(),
                    meeting.title,
                    meeting.source.as_str(),
                    now,
                    meeting.project_id.map(|id| id.to_string()),
                    meeting.audio_dir,
                    meeting.retention.as_str(),
                ],
            )
            .map_err(map_sql_error)?;
        index_meeting(&connection, meeting.id)?;
        self.meeting_by_id(&connection, meeting.id)
    }

    fn meeting(&self, id: MeetingId) -> Result<Meeting, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        self.meeting_by_id(&connection, id)
    }

    fn meetings(&self, include_archived: bool) -> Result<Vec<Meeting>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let sql = if include_archived {
            format!(
                "SELECT {MEETING_COLUMNS} FROM meetings WHERE lifecycle_state <> 'trashed' \
                 ORDER BY started_at DESC"
            )
        } else {
            format!(
                "SELECT {MEETING_COLUMNS} FROM meetings WHERE lifecycle_state = 'active' \
                 ORDER BY started_at DESC"
            )
        };
        collect_meetings(&connection, &sql, params![])
    }

    fn save_meeting(&self, meeting: &Meeting) -> Result<Meeting, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let (status, failed_stage) = meeting.status.as_columns();
        let (mic_state, mic_at, mic_reason) = meeting.mic.as_columns();
        let (system_state, system_at, system_reason) = meeting.system.as_columns();

        let changed = connection
            .execute(
                "UPDATE meetings SET title = ?2, status = ?3, failed_stage = ?4, \
                 failure_message = ?5, lifecycle_state = ?6, ended_at = ?7, duration_ms = ?8, \
                 project_id = ?9, retention = ?10, audio_deleted_at = ?11, mic_state = ?12, \
                 mic_lost_at_ms = ?13, mic_reason = ?14, system_state = ?15, \
                 system_lost_at_ms = ?16, system_reason = ?17, updated_at = ?18, \
                 cancelled_at = ?19 WHERE id = ?1",
                params![
                    meeting.id.to_string(),
                    meeting.title,
                    status,
                    failed_stage,
                    meeting.failure.as_ref().map(|failure| failure.message.clone()),
                    meeting.lifecycle_state.as_str(),
                    meeting.ended_at.map(format_time).transpose()?,
                    meeting.duration_ms,
                    meeting.project_id.map(|id| id.to_string()),
                    meeting.retention.as_str(),
                    meeting.audio_deleted_at.map(format_time).transpose()?,
                    mic_state,
                    mic_at,
                    mic_reason,
                    system_state,
                    system_at,
                    system_reason,
                    format_time(meeting.updated_at)?,
                    meeting.cancelled_at.map(format_time).transpose()?,
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Reuniao nao encontrada.",
                false,
            ));
        }
        index_meeting(&connection, meeting.id)?;
        self.meeting_by_id(&connection, meeting.id)
    }

    fn capturing_meetings(&self) -> Result<Vec<Meeting>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        collect_meetings(
            &connection,
            &format!(
                "SELECT {MEETING_COLUMNS} FROM meetings \
                 WHERE status IN ('recording', 'stopping') ORDER BY started_at"
            ),
            params![],
        )
    }

    fn meetings_with_deletable_audio(
        &self,
        now: OffsetDateTime,
    ) -> Result<Vec<Meeting>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        // A consulta so ESTREITA o conjunto; quem decide e o dominio.
        // Reimplementar `audio_may_be_deleted` em SQL criaria duas regras de
        // apagar audio, e elas divergiriam na primeira mudanca.
        let candidates = collect_meetings(
            &connection,
            &format!(
                "SELECT {MEETING_COLUMNS} FROM meetings \
                 WHERE audio_deleted_at IS NULL \
                 AND status IN ('ready', 'transcribed', 'cancelled')"
            ),
            params![],
        )?;
        Ok(candidates
            .into_iter()
            .filter(|meeting| meeting.audio_may_be_deleted(now))
            .collect())
    }

    fn mark_audio_deleted(
        &self,
        id: MeetingId,
        at: OffsetDateTime,
    ) -> Result<Meeting, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE meetings SET audio_deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
                params![id.to_string(), format_time(at)?],
            )
            .map_err(map_sql_error)?;
        self.meeting_by_id(&connection, id)
    }

    fn set_meeting_project(
        &self,
        id: MeetingId,
        project_id: Option<ProjectId>,
    ) -> Result<Meeting, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE meetings SET project_id = ?2, updated_at = ?3 WHERE id = ?1",
                params![
                    id.to_string(),
                    project_id.map(|id| id.to_string()),
                    format_time(OffsetDateTime::now_utc())?
                ],
            )
            .map_err(map_sql_error)?;
        self.meeting_by_id(&connection, id)
    }

    fn set_meeting_title(&self, id: MeetingId, title: &str) -> Result<Meeting, CoreError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O titulo da reuniao nao pode ficar vazio.",
                false,
            ));
        }
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE meetings SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![
                    id.to_string(),
                    title,
                    format_time(OffsetDateTime::now_utc())?
                ],
            )
            .map_err(map_sql_error)?;
        index_meeting(&connection, id)?;
        self.meeting_by_id(&connection, id)
    }

    /// Grava as anotacoes.
    ///
    /// Sem `trim` e sem recusar vazio, ao contrario do titulo: apagar tudo o que
    /// se escreveu e uma escolha legitima, e espaco no fim de uma nota que ainda
    /// esta sendo digitada nao e erro — o autosave dispara no meio da frase.
    ///
    /// NAO indexa para busca. A nota e contexto de analise, e promove-la a
    /// resultado de busca faria a Inbox competir com ela pelo mesmo papel.
    fn set_meeting_notes(&self, id: MeetingId, notes: &str) -> Result<Meeting, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE meetings SET notes = ?2, updated_at = ?3 WHERE id = ?1",
                params![
                    id.to_string(),
                    notes,
                    format_time(OffsetDateTime::now_utc())?
                ],
            )
            .map_err(map_sql_error)?;
        self.meeting_by_id(&connection, id)
    }

    fn set_meeting_lifecycle(
        &self,
        id: MeetingId,
        lifecycle: LifecycleState,
    ) -> Result<Meeting, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE meetings SET lifecycle_state = ?2, updated_at = ?3 WHERE id = ?1",
                params![
                    id.to_string(),
                    lifecycle.as_str(),
                    format_time(OffsetDateTime::now_utc())?
                ],
            )
            .map_err(map_sql_error)?;
        self.meeting_by_id(&connection, id)
    }

    fn replace_transcript(
        &self,
        id: MeetingId,
        segments: Vec<TranscriptSegment>,
    ) -> Result<usize, CoreError> {
        let mut connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.transaction().map_err(map_sql_error)?;
        let key = id.to_string();

        transaction
            .execute("DELETE FROM meeting_segments WHERE meeting_id = ?1", params![key])
            .map_err(map_sql_error)?;
        transaction
            .execute(
                "DELETE FROM meeting_transcript_search WHERE rowid IN \
                 (SELECT rowid FROM meeting_transcript_index WHERE meeting_id = ?1)",
                params![key],
            )
            .map_err(map_sql_error)?;
        transaction
            .execute("DELETE FROM meeting_transcript_index WHERE meeting_id = ?1", params![key])
            .map_err(map_sql_error)?;

        let written = segments.len();
        for segment in &segments {
            transaction
                .execute(
                    "INSERT INTO meeting_segments (id, meeting_id, seq, start_ms, end_ms, \
                     channel, text, speaker, confidence) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        segment.id.to_string(),
                        key,
                        segment.seq,
                        segment.start_ms,
                        segment.end_ms,
                        segment.channel.as_str(),
                        segment.text,
                        segment.speaker,
                        segment.confidence,
                    ],
                )
                .map_err(map_sql_error)?;

            transaction
                .execute(
                    "INSERT INTO meeting_transcript_search (text) VALUES (?1)",
                    params![segment.text],
                )
                .map_err(map_sql_error)?;
            let rowid = transaction.last_insert_rowid();
            transaction
                .execute(
                    "INSERT INTO meeting_transcript_index (rowid, meeting_id, segment_id) \
                     VALUES (?1, ?2, ?3)",
                    params![rowid, key, segment.id.to_string()],
                )
                .map_err(map_sql_error)?;
        }

        transaction.commit().map_err(map_sql_error)?;
        Ok(written)
    }

    fn transcript(&self, id: MeetingId) -> Result<Vec<TranscriptSegment>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT id, meeting_id, seq, start_ms, end_ms, channel, text, speaker, \
                 confidence FROM meeting_segments WHERE meeting_id = ?1 ORDER BY seq",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![id.to_string()], |row| {
                let segment_id: String = row.get(0)?;
                let meeting_id: String = row.get(1)?;
                let channel: String = row.get(5)?;
                Ok((
                    segment_id,
                    meeting_id,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    channel,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<f32>>(8)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut segments = Vec::new();
        for row in rows {
            let (id, meeting_id, seq, start_ms, end_ms, channel, text, speaker, confidence) =
                row.map_err(map_sql_error)?;
            segments.push(TranscriptSegment {
                id: SegmentId::parse(&id)?,
                meeting_id: MeetingId::parse(&meeting_id)?,
                seq,
                start_ms,
                end_ms,
                channel: MeetingChannel::parse(&channel)?,
                text,
                speaker,
                confidence,
            });
        }
        Ok(segments)
    }

    fn replace_analysis(
        &self,
        analysis: MeetingAnalysis,
        insights: Vec<MeetingInsight>,
    ) -> Result<usize, CoreError> {
        let mut connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.transaction().map_err(map_sql_error)?;
        let key = analysis.meeting_id.to_string();

        transaction
            .execute(
                "INSERT INTO meeting_analyses (meeting_id, summary, model, produced_at, windows) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(meeting_id) DO UPDATE SET summary = ?2, model = ?3, \
                 produced_at = ?4, windows = ?5",
                params![
                    key,
                    analysis.summary,
                    analysis.model,
                    format_time(analysis.produced_at)?,
                    analysis.windows,
                ],
            )
            .map_err(map_sql_error)?;

        // Reanalisar NAO desfaz o que a pessoa aceitou. Um item ja `accepted`
        // tem Task no M/OS, e apaga-lo deixaria a Task orfa da reuniao que a
        // originou — que e justamente a proveniencia que esta feature existe
        // para preservar.
        transaction
            .execute(
                "DELETE FROM meeting_insights WHERE meeting_id = ?1 AND status <> 'accepted'",
                params![key],
            )
            .map_err(map_sql_error)?;

        let kept: i64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(seq), -1) FROM meeting_insights WHERE meeting_id = ?1",
                params![key],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;

        let mut written = 0usize;
        for (offset, insight) in insights.iter().enumerate() {
            let seq = kept + 1 + offset as i64;
            transaction
                .execute(
                    "INSERT INTO meeting_insights (id, meeting_id, kind, seq, text, owner, \
                     due_hint, confidence, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        insight.id.to_string(),
                        key,
                        insight.kind.as_str(),
                        seq,
                        insight.text,
                        insight.owner,
                        insight.due_hint,
                        insight.confidence.as_str(),
                        insight.status.as_str(),
                    ],
                )
                .map_err(map_sql_error)?;

            for evidence in &insight.evidence {
                // A chave estrangeira e o que RECUSA evidencia apontando para
                // segmento que nao existe. O dominio ja filtra antes, e esta e a
                // segunda defesa — a que vale mesmo quando alguem esquece a
                // primeira.
                transaction
                    .execute(
                        "INSERT INTO meeting_evidence (insight_id, segment_id, seq, \
                         char_start, char_end) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            insight.id.to_string(),
                            evidence.segment_id.to_string(),
                            evidence.seq,
                            evidence.char_start,
                            evidence.char_end,
                        ],
                    )
                    // Uma violacao de chave estrangeira aqui tem UM significado
                    // so: a analise citou um trecho que nao existe na
                    // transcricao desta reuniao. Deixar isso sair como "falha no
                    // armazenamento" esconderia a unica causa possivel atras da
                    // causa errada.
                    .map_err(|error| match error {
                        rusqlite::Error::SqliteFailure(inner, _)
                            if inner.code == rusqlite::ErrorCode::ConstraintViolation =>
                        {
                            CoreError::new(
                                ErrorCode::DataIntegrity,
                                "A analise citou um trecho que nao existe nesta transcricao.",
                                false,
                            )
                        }
                        other => map_sql_error(other),
                    })?;
            }
            written += 1;
        }

        // Reindexar DENTRO da transacao, e nao depois.
        //
        // Duas razoes, e as duas doem. `ARCHITECTURE.md` §11.2 exige que
        // entidade e projecao mudem na mesma transacao, e que a falha da
        // indexacao derrube o comando inteiro. E travar o mutex de novo depois
        // do commit, com o guard anterior ainda vivo neste escopo, seria um
        // deadlock — `Mutex` do std nao e reentrante.
        index_meeting(&transaction, analysis.meeting_id)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(written)
    }

    fn analysis(&self, id: MeetingId) -> Result<Option<MeetingAnalysis>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let found = connection
            .query_row(
                "SELECT meeting_id, summary, model, produced_at, windows FROM meeting_analyses \
                 WHERE meeting_id = ?1",
                params![id.to_string()],
                |row| {
                    let meeting_id: String = row.get(0)?;
                    let produced_at: String = row.get(3)?;
                    Ok((
                        meeting_id,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        produced_at,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(map_sql_error(other)),
            })?;

        found
            .map(|(meeting_id, summary, model, produced_at, windows)| {
                Ok(MeetingAnalysis {
                    meeting_id: MeetingId::parse(&meeting_id)?,
                    summary,
                    model,
                    produced_at: parse_time(&produced_at)?,
                    windows: windows as u32,
                })
            })
            .transpose()
    }

    fn insights(&self, id: MeetingId) -> Result<Vec<MeetingInsight>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut insights = collect_insights(
            &connection,
            &format!(
                "SELECT {INSIGHT_COLUMNS} FROM meeting_insights WHERE meeting_id = ?1 ORDER BY seq"
            ),
            params![id.to_string()],
        )?;
        load_evidence(&connection, &mut insights)?;
        Ok(insights)
    }

    fn link_insight_result(
        &self,
        insight_id: InsightId,
        task_id: Option<TaskId>,
        reminder_id: Option<ReminderId>,
    ) -> Result<MeetingInsight, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let changed = connection
            .execute(
                "UPDATE meeting_insights SET status = 'accepted', created_task_id = ?2, \
                 created_reminder_id = ?3 WHERE id = ?1",
                params![
                    insight_id.to_string(),
                    task_id.map(|id| id.to_string()),
                    reminder_id.map(|id| id.to_string()),
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::NotFound,
                "Item de reuniao nao encontrado.",
                false,
            ));
        }
        single_insight(&connection, insight_id)
    }

    fn set_insight_status(
        &self,
        insight_id: InsightId,
        status: InsightStatus,
    ) -> Result<MeetingInsight, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE meeting_insights SET status = ?2 WHERE id = ?1",
                params![insight_id.to_string(), status.as_str()],
            )
            .map_err(map_sql_error)?;
        single_insight(&connection, insight_id)
    }

    fn insights_meeting(&self, insight_id: InsightId) -> Result<MeetingId, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let raw: String = connection
            .query_row(
                "SELECT meeting_id FROM meeting_insights WHERE id = ?1",
                params![insight_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    CoreError::new(ErrorCode::NotFound, "Item de reuniao nao encontrado.", false)
                }
                other => map_sql_error(other),
            })?;
        MeetingId::parse(&raw)
    }

    fn accept_insight(
        &self,
        accept: mos_core::AcceptInsight,
        task: mos_core::NewTask,
        reminder: Option<mos_core::NewReminder>,
    ) -> Result<mos_core::AcceptedInsight, CoreError> {
        let insight_id = accept.insight_id;
        let task_id = task.id;
        let reminder_id = reminder.as_ref().map(|reminder| reminder.id);

        let mut connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.transaction().map_err(map_sql_error)?;

        // A guarda vem PRIMEIRO, dentro da transacao. Fora dela, duas
        // confirmacoes rapidas do mesmo item criariam duas Tasks para o mesmo
        // compromisso — e a segunda sobrescreveria o vinculo da primeira,
        // deixando uma Task orfa que ninguem mais relaciona a reuniao.
        let status: String = transaction
            .query_row(
                "SELECT status FROM meeting_insights WHERE id = ?1",
                params![insight_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    CoreError::new(ErrorCode::NotFound, "Item de reuniao nao encontrado.", false)
                }
                other => map_sql_error(other),
            })?;
        if InsightStatus::parse(&status)? != InsightStatus::Proposed {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Este item de reuniao ja foi resolvido.",
                false,
            ));
        }

        crate::work_repository::insert_task(self, &transaction, task, None)?;
        if let Some(reminder) = &reminder {
            crate::attention_repository::insert_reminder(&transaction, reminder)?;
        }

        transaction
            .execute(
                "UPDATE meeting_insights SET status = 'accepted', created_task_id = ?2, \
                 created_reminder_id = ?3 WHERE id = ?1",
                params![
                    insight_id.to_string(),
                    task_id.to_string(),
                    reminder_id.map(|id| id.to_string()),
                ],
            )
            .map_err(map_sql_error)?;

        transaction.commit().map_err(map_sql_error)?;
        drop(connection);

        Ok(mos_core::AcceptedInsight {
            insight: self.insight(insight_id)?,
            task_id,
            reminder_id,
        })
    }

    fn open_commitments(&self) -> Result<Vec<MeetingInsight>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        collect_insights(
            &connection,
            &format!(
                "SELECT {INSIGHT_COLUMNS_QUALIFIED} FROM meeting_insights i \
                 JOIN meetings m ON m.id = i.meeting_id \
                 WHERE i.kind = 'my_action' AND i.status = 'proposed' \
                 AND m.lifecycle_state = 'active' \
                 ORDER BY m.started_at DESC, i.seq"
            ),
            params![],
        )
    }

    fn search_meetings(&self, request: SearchRequest) -> Result<Vec<Meeting>, CoreError> {
        let query = to_fts_query(&request.query);
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if request.include_archived {
            "m.lifecycle_state <> 'trashed'"
        } else {
            "m.lifecycle_state = 'active'"
        };
        collect_meetings(
            &connection,
            &format!(
                "SELECT {} FROM meeting_search s \
                 JOIN meeting_search_index x ON x.rowid = s.rowid \
                 JOIN meetings m ON m.id = x.meeting_id \
                 WHERE meeting_search MATCH ?1 AND {lifecycle} \
                 ORDER BY rank LIMIT ?2",
                MEETING_COLUMNS
                    .split(", ")
                    .map(|column| format!("m.{column}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            params![query, request.limit as i64],
        )
    }

    fn search_transcripts(
        &self,
        request: SearchRequest,
    ) -> Result<Vec<(Meeting, String)>, CoreError> {
        let query = to_fts_query(&request.query);
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let lifecycle = if request.include_archived {
            "m.lifecycle_state <> 'trashed'"
        } else {
            "m.lifecycle_state = 'active'"
        };

        // A deduplicacao acontece no `GROUP BY` da subconsulta: uma reuniao, um
        // resultado, mesmo que a palavra apareca quarenta vezes na transcricao.
        //
        // E o trecho vem do PROPRIO SEGMENTO, e nao de `snippet()`. Duas razoes,
        // e a primeira custou um teste vermelho: as funcoes auxiliares do FTS5
        // nao valem em contexto agregado, e o SQLite recusa com "unable to use
        // function snippet in the requested context". A segunda e melhor: uma
        // fala inteira e mais contexto que um fragmento cortado no meio.
        let sql = format!(
            "SELECT {}, seg.text \
             FROM ( \
                 SELECT x.meeting_id AS mid, MIN(x.rowid) AS best \
                 FROM meeting_transcript_search f \
                 JOIN meeting_transcript_index x ON x.rowid = f.rowid \
                 WHERE meeting_transcript_search MATCH ?1 \
                 GROUP BY x.meeting_id \
             ) hits \
             JOIN meeting_transcript_index xi ON xi.rowid = hits.best \
             JOIN meetings m ON m.id = hits.mid \
             JOIN meeting_segments seg ON seg.id = xi.segment_id \
             WHERE {lifecycle} \
             ORDER BY m.started_at DESC LIMIT ?2",
            MEETING_COLUMNS
                .split(", ")
                .map(|column| format!("m.{column}"))
                .collect::<Vec<_>>()
                .join(", ")
        );

        let mut statement = connection.prepare(&sql).map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![query, request.limit as i64], |row| {
                let meeting = read_meeting(row)?;
                // O texto do segmento vem DEPOIS das colunas da reuniao, entao o
                // indice e contado e nao escrito. Ele ja estava errado uma vez:
                // a coluna `notes` da 0022 empurrou tudo em um, e um `23` fixo
                // passou a ler `notes` como se fosse o trecho da transcricao.
                let text: String = row.get(MEETING_COLUMNS.split(", ").count())?;
                Ok((meeting, text))
            })
            .map_err(map_sql_error)?;

        let mut found = Vec::new();
        for row in rows {
            let (meeting, text) = row.map_err(map_sql_error)?;
            found.push((meeting?, trim_snippet(&text)));
        }
        Ok(found)
    }

    fn rebuild_meeting_search(&self) -> Result<usize, CoreError> {
        let mut connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.transaction().map_err(map_sql_error)?;
        transaction
            .execute_batch(
                "DELETE FROM meeting_search; \
                 DELETE FROM meeting_search_index; \
                 DELETE FROM meeting_transcript_search; \
                 DELETE FROM meeting_transcript_index;",
            )
            .map_err(map_sql_error)?;

        let ids: Vec<String> = {
            let mut statement = transaction
                .prepare("SELECT id FROM meetings")
                .map_err(map_sql_error)?;
            let rows = statement
                .query_map(params![], |row| row.get::<_, String>(0))
                .map_err(map_sql_error)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(map_sql_error)?
        };

        let mut rebuilt = 0usize;
        for id in &ids {
            index_meeting(&transaction, MeetingId::parse(id)?)?;
            let mut statement = transaction
                .prepare("SELECT id, text FROM meeting_segments WHERE meeting_id = ?1 ORDER BY seq")
                .map_err(map_sql_error)?;
            let segments: Vec<(String, String)> = statement
                .query_map(params![id], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(map_sql_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(map_sql_error)?;
            drop(statement);

            for (segment_id, text) in segments {
                transaction
                    .execute(
                        "INSERT INTO meeting_transcript_search (text) VALUES (?1)",
                        params![text],
                    )
                    .map_err(map_sql_error)?;
                let rowid = transaction.last_insert_rowid();
                transaction
                    .execute(
                        "INSERT INTO meeting_transcript_index (rowid, meeting_id, segment_id) \
                         VALUES (?1, ?2, ?3)",
                        params![rowid, id, segment_id],
                    )
                    .map_err(map_sql_error)?;
            }
            rebuilt += 1;
        }

        transaction.commit().map_err(map_sql_error)?;
        Ok(rebuilt)
    }
}

/// Reescreve a linha da reuniao no indice global.
///
/// `meeting_search` e um FTS5 sem `content=`, entao ele nao sabe apagar sozinho.
/// `meeting_search_index` guarda o vinculo rowid → reuniao, e e ela que torna a
/// reindexacao possivel sem varrer a tabela virtual inteira.
fn index_meeting(connection: &Connection, id: MeetingId) -> Result<(), CoreError> {
    let key = id.to_string();
    connection
        .execute(
            "DELETE FROM meeting_search WHERE rowid IN \
             (SELECT rowid FROM meeting_search_index WHERE meeting_id = ?1)",
            params![key],
        )
        .map_err(map_sql_error)?;
    connection
        .execute("DELETE FROM meeting_search_index WHERE meeting_id = ?1", params![key])
        .map_err(map_sql_error)?;

    let title: String = connection
        .query_row("SELECT title FROM meetings WHERE id = ?1", params![key], |row| {
            row.get(0)
        })
        .map_err(map_sql_error)?;
    let summary: String = connection
        .query_row(
            "SELECT COALESCE((SELECT summary FROM meeting_analyses WHERE meeting_id = ?1), '')",
            params![key],
            |row| row.get(0),
        )
        .map_err(map_sql_error)?;
    let insights: String = connection
        .query_row(
            "SELECT COALESCE(group_concat(text, ' '), '') FROM meeting_insights \
             WHERE meeting_id = ?1",
            params![key],
            |row| row.get(0),
        )
        .map_err(map_sql_error)?;

    connection
        .execute(
            "INSERT INTO meeting_search (title, summary, insights) VALUES (?1, ?2, ?3)",
            params![title, summary, insights],
        )
        .map_err(map_sql_error)?;
    let rowid = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO meeting_search_index (rowid, meeting_id) VALUES (?1, ?2)",
            params![rowid, key],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn collect_meetings(
    connection: &Connection,
    sql: &str,
    parameters: impl rusqlite::Params,
) -> Result<Vec<Meeting>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let rows = statement
        .query_map(parameters, read_meeting)
        .map_err(map_sql_error)?;
    let mut found = Vec::new();
    for row in rows {
        found.push(row.map_err(map_sql_error)??);
    }
    Ok(found)
}

fn collect_insights(
    connection: &Connection,
    sql: &str,
    parameters: impl rusqlite::Params,
) -> Result<Vec<MeetingInsight>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let rows = statement
        .query_map(parameters, read_insight)
        .map_err(map_sql_error)?;
    let mut found = Vec::new();
    for row in rows {
        found.push(row.map_err(map_sql_error)??);
    }
    Ok(found)
}

impl SqliteStorage {
    /// Um item so, com a evidencia dele.
    fn insight(&self, insight_id: InsightId) -> Result<MeetingInsight, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        single_insight(&connection, insight_id)
    }
}

fn single_insight(
    connection: &Connection,
    insight_id: InsightId,
) -> Result<MeetingInsight, CoreError> {
    let mut found = collect_insights(
        connection,
        &format!("SELECT {INSIGHT_COLUMNS} FROM meeting_insights WHERE id = ?1"),
        params![insight_id.to_string()],
    )?;
    load_evidence(connection, &mut found)?;
    found.pop().ok_or_else(|| {
        CoreError::new(ErrorCode::NotFound, "Item de reuniao nao encontrado.", false)
    })
}


/// Encurta uma fala para caber num resultado de busca.
///
/// Corta em limite de palavra: cortar no meio de uma palavra faz o trecho
/// parecer erro de dado em vez de recorte deliberado.
fn trim_snippet(text: &str) -> String {
    const LIMITE: usize = 160;
    if text.chars().count() <= LIMITE {
        return text.to_owned();
    }
    let cortado: String = text.chars().take(LIMITE).collect();
    match cortado.rfind(' ') {
        Some(espaco) if espaco > LIMITE / 2 => format!("{}…", &cortado[..espaco]),
        _ => format!("{cortado}…"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_core::{
        apply_meeting, interleave, AudioOutcome, FailedStage, MeetingService, MeetingSource,
        MeetingTransition, NewProject, NewTask, RawSegment, WorkRepository,
    };
    use std::sync::Arc;

    fn storage() -> (tempfile::TempDir, SqliteStorage) {
        let directory = tempfile::tempdir().unwrap();
        let storage = SqliteStorage::open(
            directory.path().join("mos.db"),
            directory.path().join("backups"),
        )
        .unwrap();
        (directory, storage)
    }

    fn started() -> OffsetDateTime {
        time::macros::datetime!(2026-08-18 14:00:00 UTC)
    }

    fn start_meeting(storage: &SqliteStorage, title: &str) -> Meeting {
        storage
            .create_meeting(NewMeeting::start(
                title,
                MeetingSource::Manual,
                None,
                started(),
            ))
            .unwrap()
    }

    #[test]
    fn notas_gravam_leem_e_nascem_vazias() {
        let (_dir, storage) = storage();
        let reuniao = start_meeting(&storage, "NexoDoc");
        // Nasce vazia, e nao nula: ninguem escreveu ainda, e isso e um fato
        // completo.
        assert_eq!(reuniao.notes, "");

        let salva = storage
            .set_meeting_notes(reuniao.id, "orcamento ate sexta")
            .unwrap();
        assert_eq!(salva.notes, "orcamento ate sexta");

        let relida = storage.meeting(reuniao.id).unwrap();
        assert_eq!(relida.notes, "orcamento ate sexta");

        // Apagar tudo volta ao vazio, e nao vira NULL nem erro.
        let limpa = storage.set_meeting_notes(reuniao.id, "").unwrap();
        assert_eq!(limpa.notes, "");
    }

    fn transcribe(storage: &SqliteStorage, meeting: &Meeting) -> Vec<TranscriptSegment> {
        let segments = interleave(
            meeting.id,
            vec![RawSegment {
                start_ms: 4_000,
                end_ms: 8_000,
                text: "Eu termino os slides amanha de manha.".into(),
                confidence: Some(0.94),
            }],
            vec![RawSegment {
                start_ms: 9_000,
                end_ms: 12_000,
                text: "Combinado, eu reviso na sexta.".into(),
                confidence: Some(0.88),
            }],
        );
        storage
            .replace_transcript(meeting.id, segments.clone())
            .unwrap();
        segments
    }

    fn insight(
        meeting: &Meeting,
        kind: InsightKind,
        text: &str,
        evidence: Vec<MeetingEvidence>,
    ) -> MeetingInsight {
        MeetingInsight {
            id: InsightId::new(),
            meeting_id: meeting.id,
            kind,
            seq: 0,
            text: text.into(),
            owner: None,
            due_hint: Some("amanha".into()),
            confidence: Confidence::High,
            status: InsightStatus::Proposed,
            created_task_id: None,
            created_reminder_id: None,
            evidence,
        }
    }

    fn analysis(meeting: &Meeting) -> MeetingAnalysis {
        MeetingAnalysis {
            meeting_id: meeting.id,
            summary: "Alinhamento comercial do NexoDoc.".into(),
            model: "hermes".into(),
            produced_at: started(),
            windows: 1,
        }
    }

    #[test]
    fn a_reuniao_nasce_gravando() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");

        assert_eq!(meeting.status, MeetingStatus::Recording);
        assert_eq!(meeting.retention, AudioRetention::DeleteAfterProcessing);
        assert!(matches!(meeting.mic, ChannelOutcome::Capturing));
        assert_eq!(meeting.duration_ms, 0);
        assert_eq!(meeting.audio_dir, format!("meetings/{}", meeting.id));
    }

    #[test]
    fn a_reconciliacao_de_abertura_acha_a_reuniao_em_captura() {
        let (_dir, storage) = storage();
        let recording = start_meeting(&storage, "NexoDoc");
        let finished = start_meeting(&storage, "Ja terminada");
        let finished = apply_meeting(&finished, MeetingTransition::Stop, started()).unwrap();
        let finished =
            apply_meeting(&finished, MeetingTransition::AudioSettled, started()).unwrap();
        storage.save_meeting(&finished).unwrap();

        let capturing = storage.capturing_meetings().unwrap();
        assert_eq!(capturing.len(), 1);
        assert_eq!(capturing[0].id, recording.id);
    }

    #[test]
    fn o_estado_e_a_duracao_atravessam_o_banco() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");

        let mut interrupted =
            apply_meeting(&meeting, MeetingTransition::DetectInterrupted, started()).unwrap();
        // A duracao vem medida em frames, e o banco a guarda como veio.
        interrupted.duration_ms = 4_680_000;
        interrupted.mic = ChannelOutcome::Lost {
            at_ms: 1_930_000,
            reason: "headset desconectado".into(),
        };
        interrupted.system = ChannelOutcome::Captured;
        let saved = storage.save_meeting(&interrupted).unwrap();

        assert_eq!(saved.status, MeetingStatus::Interrupted);
        assert_eq!(saved.duration_ms, 4_680_000);
        assert_eq!(
            saved.mic,
            ChannelOutcome::Lost {
                at_ms: 1_930_000,
                reason: "headset desconectado".into()
            }
        );
        assert!(matches!(saved.system, ChannelOutcome::Captured));
    }

    #[test]
    fn falha_guarda_o_estagio_e_a_mensagem() {
        let (_dir, storage) = storage();
        let mut recorded = start_meeting(&storage, "NexoDoc");
        recorded.status = MeetingStatus::Failed(FailedStage::Transcription);
        recorded.failure = Some(MeetingFailure {
            stage: FailedStage::Transcription,
            message: "modelo local nao encontrado".into(),
        });
        let saved = storage.save_meeting(&recorded).unwrap();

        assert_eq!(
            saved.status,
            MeetingStatus::Failed(FailedStage::Transcription)
        );
        assert_eq!(saved.failure.unwrap().message, "modelo local nao encontrado");
    }

    #[test]
    fn o_banco_recusa_failed_sem_estagio() {
        // A guarda do CHECK existe porque uma linha assim passaria pelo insert e
        // so quebraria muito depois, na leitura.
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        let connection = storage.connection.lock().unwrap();
        let result = connection.execute(
            "UPDATE meetings SET status = 'failed', failed_stage = NULL WHERE id = ?1",
            params![meeting.id.to_string()],
        );
        assert!(result.is_err(), "o CHECK deveria recusar failed sem estagio");
    }

    #[test]
    fn a_transcricao_preserva_canal_e_ordem() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        transcribe(&storage, &meeting);

        let read = storage.transcript(meeting.id).unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].channel, MeetingChannel::Mic);
        assert_eq!(read[0].seq, 0);
        assert_eq!(read[1].channel, MeetingChannel::System);
        assert_eq!(read[1].start_ms, 9_000);
    }

    #[test]
    fn retranscrever_substitui_e_nao_acumula() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        transcribe(&storage, &meeting);
        transcribe(&storage, &meeting);

        assert_eq!(storage.transcript(meeting.id).unwrap().len(), 2);
    }

    #[test]
    fn a_evidencia_referencia_o_segmento_e_volta_inteira() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        let segments = transcribe(&storage, &meeting);

        let item = insight(
            &meeting,
            InsightKind::MyAction,
            "Finalizar a apresentacao",
            vec![MeetingEvidence {
                segment_id: segments[0].id,
                seq: 0,
                char_start: Some(0),
                char_end: Some(13),
            }],
        );
        storage
            .replace_analysis(analysis(&meeting), vec![item])
            .unwrap();

        let read = storage.insights(meeting.id).unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].evidence.len(), 1);
        assert_eq!(read[0].evidence[0].segment_id, segments[0].id);
        assert_eq!(read[0].due_hint.as_deref(), Some("amanha"));
    }

    #[test]
    fn evidencia_para_segmento_inexistente_e_recusada_pelo_banco() {
        // A segunda defesa. O dominio ja filtra evidencia inventada pelo modelo;
        // esta e a que vale quando alguem esquecer a primeira.
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        transcribe(&storage, &meeting);

        let item = insight(
            &meeting,
            InsightKind::MyAction,
            "Acao inventada",
            vec![MeetingEvidence {
                segment_id: SegmentId::new(),
                seq: 0,
                char_start: None,
                char_end: None,
            }],
        );
        let error = storage
            .replace_analysis(analysis(&meeting), vec![item])
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::DataIntegrity);

        // E a transacao inteira voltou: nenhum item entrou pela metade.
        assert!(storage.insights(meeting.id).unwrap().is_empty());
    }

    #[test]
    fn reanalisar_preserva_o_que_foi_aceito() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        let segments = transcribe(&storage, &meeting);
        let evidence = vec![MeetingEvidence {
            segment_id: segments[0].id,
            seq: 0,
            char_start: None,
            char_end: None,
        }];

        let aceito = insight(
            &meeting,
            InsightKind::MyAction,
            "Ja virou Task",
            evidence.clone(),
        );
        let descartavel = insight(&meeting, InsightKind::Decision, "So uma decisao", evidence);
        storage
            .replace_analysis(analysis(&meeting), vec![aceito.clone(), descartavel])
            .unwrap();

        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        let task = storage
            .create_task(NewTask::create("Ja virou Task", "", Some(project.id)).unwrap())
            .unwrap();
        storage
            .link_insight_result(aceito.id, Some(task.id), None)
            .unwrap();

        // A reanalise chega com itens novos e NAO pode desfazer a Task criada.
        storage
            .replace_analysis(
                analysis(&meeting),
                vec![insight(
                    &meeting,
                    InsightKind::Decision,
                    "Outra leitura",
                    Vec::new(),
                )],
            )
            .unwrap();

        let read = storage.insights(meeting.id).unwrap();
        let sobrevivente = read
            .iter()
            .find(|item| item.id == aceito.id)
            .expect("o item aceito precisa sobreviver a reanalise");
        assert_eq!(sobrevivente.status, InsightStatus::Accepted);
        assert_eq!(sobrevivente.created_task_id, Some(task.id));
        assert!(read.iter().any(|item| item.text == "Outra leitura"));
        assert!(
            !read.iter().any(|item| item.text == "So uma decisao"),
            "o item nao aceito e substituido"
        );
    }

    #[test]
    fn apagar_a_task_deixa_o_item_orfao_e_vivo() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        let segments = transcribe(&storage, &meeting);
        let item = insight(
            &meeting,
            InsightKind::MyAction,
            "Finalizar slides",
            vec![MeetingEvidence {
                segment_id: segments[0].id,
                seq: 0,
                char_start: None,
                char_end: None,
            }],
        );
        storage
            .replace_analysis(analysis(&meeting), vec![item.clone()])
            .unwrap();

        let task = storage
            .create_task(NewTask::create("Finalizar slides", "", None).unwrap())
            .unwrap();
        storage
            .link_insight_result(item.id, Some(task.id), None)
            .unwrap();

        storage
            .set_task_lifecycle(task.id, LifecycleState::Archived)
            .unwrap();
        storage.delete_task(task.id).unwrap();

        let read = storage.insights(meeting.id).unwrap();
        assert_eq!(read.len(), 1, "o item nao pode sumir com a Task");
        assert_eq!(read[0].created_task_id, None, "o vinculo fica perdido");
    }

    #[test]
    fn compromissos_abertos_saem_por_sql() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        let segments = transcribe(&storage, &meeting);
        let evidence = vec![MeetingEvidence {
            segment_id: segments[0].id,
            seq: 0,
            char_start: None,
            char_end: None,
        }];

        let meu = insight(
            &meeting,
            InsightKind::MyAction,
            "Eu devo os slides",
            evidence.clone(),
        );
        let dos_outros = insight(
            &meeting,
            InsightKind::OtherAction,
            "Ele revisa",
            evidence.clone(),
        );
        let ja_feito = insight(&meeting, InsightKind::MyAction, "Ja resolvido", evidence);
        storage
            .replace_analysis(
                analysis(&meeting),
                vec![meu.clone(), dos_outros, ja_feito.clone()],
            )
            .unwrap();
        storage
            .set_insight_status(ja_feito.id, InsightStatus::Accepted)
            .unwrap();

        let abertos = storage.open_commitments().unwrap();
        assert_eq!(abertos.len(), 1);
        assert_eq!(abertos[0].id, meu.id);
    }

    #[test]
    fn a_search_global_acha_pela_reuniao_e_nao_pelo_segmento() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc Comercial");
        let segments = transcribe(&storage, &meeting);
        storage
            .replace_analysis(
                analysis(&meeting),
                vec![insight(
                    &meeting,
                    InsightKind::Decision,
                    "Fechado usar Hermes no M/OS",
                    vec![MeetingEvidence {
                        segment_id: segments[0].id,
                        seq: 0,
                        char_start: None,
                        char_end: None,
                    }],
                )],
            )
            .unwrap();

        let request = |query: &str| SearchRequest {
            query: query.into(),
            include_archived: false,
            limit: 20,
        };

        // Titulo, resumo e item entram no indice global.
        assert_eq!(storage.search_meetings(request("NexoDoc")).unwrap().len(), 1);
        assert_eq!(
            storage.search_meetings(request("alinhamento")).unwrap().len(),
            1
        );
        assert_eq!(storage.search_meetings(request("Hermes")).unwrap().len(), 1);

        // A transcricao NAO entra no indice global.
        assert!(
            storage.search_meetings(request("slides")).unwrap().is_empty(),
            "segmento de transcricao nao pode aparecer na Search global"
        );
        // Mas ela e encontravel pelo indice proprio.
        assert_eq!(
            storage.search_transcripts(request("slides")).unwrap().len(),
            1
        );
    }

    #[test]
    fn a_busca_na_transcricao_deduplica_por_reuniao() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "Repetida");
        let repetidos: Vec<RawSegment> = (0..8)
            .map(|index| RawSegment {
                start_ms: index * 1000,
                end_ms: index * 1000 + 900,
                text: "orcamento".into(),
                confidence: None,
            })
            .collect();
        storage
            .replace_transcript(meeting.id, interleave(meeting.id, repetidos, Vec::new()))
            .unwrap();

        let found = storage
            .search_transcripts(SearchRequest {
                query: "orcamento".into(),
                include_archived: false,
                limit: 20,
            })
            .unwrap();
        assert_eq!(found.len(), 1, "uma reuniao, um resultado");
        assert!(!found[0].1.is_empty(), "e o trecho vem junto");
    }

    #[test]
    fn a_search_ignora_reuniao_arquivada_por_padrao() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "Arquivada");
        storage
            .set_meeting_lifecycle(meeting.id, LifecycleState::Archived)
            .unwrap();

        let request = |include_archived: bool| SearchRequest {
            query: "Arquivada".into(),
            include_archived,
            limit: 20,
        };
        assert!(storage.search_meetings(request(false)).unwrap().is_empty());
        assert_eq!(storage.search_meetings(request(true)).unwrap().len(), 1);
    }

    #[test]
    fn o_indice_e_reconstruivel() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        transcribe(&storage, &meeting);

        {
            let connection = storage.connection.lock().unwrap();
            connection
                .execute_batch(
                    "DELETE FROM meeting_search; DELETE FROM meeting_search_index; \
                     DELETE FROM meeting_transcript_search; DELETE FROM meeting_transcript_index;",
                )
                .unwrap();
        }

        assert_eq!(storage.rebuild_meeting_search().unwrap(), 1);
        let request = |query: &str| SearchRequest {
            query: query.into(),
            include_archived: false,
            limit: 20,
        };
        assert_eq!(storage.search_meetings(request("NexoDoc")).unwrap().len(), 1);
        assert_eq!(
            storage.search_transcripts(request("slides")).unwrap().len(),
            1
        );
    }

    #[test]
    fn apagar_o_project_nao_apaga_a_reuniao() {
        let (_dir, storage) = storage();
        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();
        let meeting = start_meeting(&storage, "NexoDoc");
        let meeting = storage
            .set_meeting_project(meeting.id, Some(project.id))
            .unwrap();
        assert_eq!(meeting.project_id, Some(project.id));

        storage
            .set_project_lifecycle(project.id, LifecycleState::Archived)
            .unwrap();
        storage.delete_project(project.id).unwrap();

        let read = storage.meeting(meeting.id).unwrap();
        assert_eq!(read.project_id, None, "a reuniao perde o contexto");
        assert_eq!(read.title, "NexoDoc", "mas nao a existencia");
    }

    #[test]
    fn a_retencao_decide_quem_pode_perder_o_audio() {
        let (_dir, storage) = storage();

        let mut pronta = start_meeting(&storage, "Pronta");
        pronta.status = MeetingStatus::Ready;
        pronta.ended_at = Some(started());
        storage.save_meeting(&pronta).unwrap();

        let mut falhou = start_meeting(&storage, "Falhou");
        falhou.status = MeetingStatus::Failed(FailedStage::Analysis);
        falhou.failure = Some(MeetingFailure {
            stage: FailedStage::Analysis,
            message: "hermes offline".into(),
        });
        falhou.ended_at = Some(started());
        storage.save_meeting(&falhou).unwrap();

        let mut guardar = start_meeting(&storage, "Guardar");
        guardar.status = MeetingStatus::Ready;
        guardar.retention = AudioRetention::Keep;
        guardar.ended_at = Some(started());
        storage.save_meeting(&guardar).unwrap();

        let deletaveis = storage.meetings_with_deletable_audio(started()).unwrap();
        assert_eq!(deletaveis.len(), 1);
        assert_eq!(deletaveis[0].id, pronta.id);

        let marcada = storage.mark_audio_deleted(pronta.id, started()).unwrap();
        assert!(marcada.audio_deleted_at.is_some());
        assert!(
            storage
                .meetings_with_deletable_audio(started())
                .unwrap()
                .is_empty(),
            "audio ja apagado nao volta para a fila"
        );
    }

    #[test]
    fn apagar_a_reuniao_leva_transcricao_analise_e_evidencia() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        let segments = transcribe(&storage, &meeting);
        storage
            .replace_analysis(
                analysis(&meeting),
                vec![insight(
                    &meeting,
                    InsightKind::Decision,
                    "Uma decisao",
                    vec![MeetingEvidence {
                        segment_id: segments[0].id,
                        seq: 0,
                        char_start: None,
                        char_end: None,
                    }],
                )],
            )
            .unwrap();

        let connection = storage.connection.lock().unwrap();
        connection
            .execute(
                "DELETE FROM meetings WHERE id = ?1",
                params![meeting.id.to_string()],
            )
            .unwrap();

        for tabela in [
            "meeting_segments",
            "meeting_insights",
            "meeting_analyses",
            "meeting_search_index",
            "meeting_transcript_index",
            "meeting_evidence",
        ] {
            let restante: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {tabela}"), params![], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(restante, 0, "{tabela} deveria ter sido cascateada");
        }
    }

    // ================================================================
    // MeetingService: a camada de aplicacao, exercitada contra o banco
    // real. Ela mora aqui e nao no crate do desktop porque
    // `SETUP-MAQUINA.md` §4 registra que os testes de la nao rodam.
    // ================================================================

    fn service(storage: &Arc<SqliteStorage>) -> MeetingService {
        MeetingService::new(
            storage.clone(),
            Arc::new(mos_core::FixedClock::at(started())),
        )
    }

    fn shared() -> (tempfile::TempDir, Arc<SqliteStorage>) {
        let directory = tempfile::tempdir().unwrap();
        let storage = Arc::new(
            SqliteStorage::open(
                directory.path().join("mos.db"),
                directory.path().join("backups"),
            )
            .unwrap(),
        );
        (directory, storage)
    }

    #[test]
    fn uma_gravacao_por_vez() {
        let (_dir, storage) = shared();
        let service = service(&storage);

        let primeira = service.start("NexoDoc", None).unwrap();
        let error = service.start("Outra", None).unwrap_err();

        assert_eq!(error.code, ErrorCode::InvalidTransition);
        assert!(
            error.message.contains("NexoDoc"),
            "o erro precisa dizer QUAL gravacao esta em curso: {}",
            error.message
        );
        assert_eq!(service.recording().unwrap().unwrap().id, primeira.id);
    }

    #[test]
    fn parar_libera_para_a_proxima() {
        let (_dir, storage) = shared();
        let service = service(&storage);

        let primeira = service.start("NexoDoc", None).unwrap();
        service.stop(&primeira.id.to_string()).unwrap();
        service
            .settle_audio(
                &primeira.id.to_string(),
                AudioOutcome {
                    duration_ms: 4_320_000,
                    mic: ChannelOutcome::Captured,
                    system: ChannelOutcome::Captured,
                },
            )
            .unwrap();

        assert!(service.recording().unwrap().is_none());
        assert!(service.start("A proxima", None).is_ok());
    }

    #[test]
    fn a_duracao_medida_e_a_que_fica() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        let id = meeting.id.to_string();

        service.stop(&id).unwrap();
        // O relogio nao andou (FixedClock), mas a captura mediu 1h12 em frames.
        // E a medicao que vale.
        let settled = service
            .settle_audio(
                &id,
                AudioOutcome {
                    duration_ms: 4_320_000,
                    mic: ChannelOutcome::Captured,
                    system: ChannelOutcome::Lost {
                        at_ms: 3_000_000,
                        reason: "saida desconectada".into(),
                    },
                },
            )
            .unwrap();

        assert_eq!(settled.status, MeetingStatus::Recorded);
        assert_eq!(settled.duration_ms, 4_320_000);
    }

    #[test]
    fn os_dois_canais_mudos_viram_falha_e_nao_reuniao_vazia() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        let id = meeting.id.to_string();
        service.stop(&id).unwrap();

        let settled = service
            .settle_audio(
                &id,
                AudioOutcome {
                    duration_ms: 0,
                    mic: ChannelOutcome::Unavailable {
                        reason: "sem microfone".into(),
                    },
                    system: ChannelOutcome::Unavailable {
                        reason: "loopback nao abriu".into(),
                    },
                },
            )
            .unwrap();

        assert_eq!(settled.status, MeetingStatus::Failed(FailedStage::Audio));
        assert!(settled.failure.is_some());
    }

    #[test]
    fn a_reconciliacao_recupera_a_duracao_do_disco_e_nao_apaga_nada() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();

        // O processo morreu. Na abertura seguinte, o disco diz 1h18.
        let recuperadas = service
            .reconcile_on_open(&|_| 4_680_000)
            .unwrap();

        assert_eq!(recuperadas.len(), 1);
        assert_eq!(recuperadas[0].id, meeting.id);
        assert_eq!(recuperadas[0].status, MeetingStatus::Interrupted);
        assert_eq!(recuperadas[0].duration_ms, 4_680_000);
        assert!(
            matches!(recuperadas[0].mic, ChannelOutcome::Captured),
            "um canal que estava capturando e que tem audio fica `captured`"
        );
        // E ela continua existindo, esperando a decisao da pessoa.
        assert_eq!(service.meetings(false).unwrap().len(), 1);
    }

    #[test]
    fn reconciliacao_sem_audio_nenhum_marca_o_canal_como_indisponivel() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        service.start("Morreu no primeiro segundo", None).unwrap();

        let recuperadas = service.reconcile_on_open(&|_| 0).unwrap();
        assert_eq!(recuperadas[0].duration_ms, 0);
        assert!(matches!(
            recuperadas[0].mic,
            ChannelOutcome::Unavailable { .. }
        ));
    }

    #[test]
    fn processar_recuperada_segue_para_o_pipeline_normal() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        service.reconcile_on_open(&|_| 4_680_000).unwrap();
        let id = meeting.id.to_string();

        assert_eq!(
            service.process_recovered(&id).unwrap().status,
            MeetingStatus::Recorded
        );
        assert_eq!(
            service.start_transcription(&id).unwrap().status,
            MeetingStatus::Transcribing
        );
    }

    #[test]
    fn descartar_recuperada_encerra_sem_apagar_a_linha() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        service.reconcile_on_open(&|_| 4_680_000).unwrap();

        let cancelada = service.cancel(&meeting.id.to_string()).unwrap();
        assert_eq!(cancelada.status, MeetingStatus::Cancelled);
        assert!(cancelada.cancelled_at.is_some());
        assert_eq!(
            cancelada.duration_ms, 4_680_000,
            "a duracao descartada continua consultavel"
        );
    }

    #[test]
    fn a_transcricao_entra_antes_do_estado_mudar() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        let id = meeting.id.to_string();
        service.stop(&id).unwrap();
        service
            .settle_audio(
                &id,
                AudioOutcome {
                    duration_ms: 12_000,
                    mic: ChannelOutcome::Captured,
                    system: ChannelOutcome::Captured,
                },
            )
            .unwrap();
        service.start_transcription(&id).unwrap();

        let segments = interleave(
            meeting.id,
            vec![RawSegment {
                start_ms: 0,
                end_ms: 4_000,
                text: "Eu termino os slides amanha.".into(),
                confidence: None,
            }],
            Vec::new(),
        );
        let done = service.finish_transcription(&id, segments).unwrap();

        assert_eq!(done.status, MeetingStatus::Transcribed);
        assert_eq!(service.transcript(&id).unwrap().len(), 1);
    }

    #[test]
    fn hermes_offline_deixa_a_reuniao_utilizavel() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        let id = meeting.id.to_string();
        service.stop(&id).unwrap();
        service
            .settle_audio(
                &id,
                AudioOutcome {
                    duration_ms: 12_000,
                    mic: ChannelOutcome::Captured,
                    system: ChannelOutcome::Captured,
                },
            )
            .unwrap();
        service.start_transcription(&id).unwrap();
        service
            .finish_transcription(
                &id,
                interleave(
                    meeting.id,
                    vec![RawSegment {
                        start_ms: 0,
                        end_ms: 4_000,
                        text: "Precisamos revisar isso amanha.".into(),
                        confidence: None,
                    }],
                    Vec::new(),
                ),
            )
            .unwrap();

        // O Hermes nunca respondeu. A reuniao fica em repouso, com transcricao
        // completa — e isso NAO e falha.
        let parada = service.meeting(&id).unwrap();
        assert_eq!(parada.status, MeetingStatus::Transcribed);
        assert!(parada.failure.is_none());
        assert_eq!(service.transcript(&id).unwrap().len(), 1);
    }

    #[test]
    fn falha_de_analise_preserva_a_transcricao_e_o_retry_volta_para_ela() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        let id = meeting.id.to_string();
        service.stop(&id).unwrap();
        service
            .settle_audio(
                &id,
                AudioOutcome {
                    duration_ms: 12_000,
                    mic: ChannelOutcome::Captured,
                    system: ChannelOutcome::Captured,
                },
            )
            .unwrap();
        service.start_transcription(&id).unwrap();
        service
            .finish_transcription(
                &id,
                interleave(
                    meeting.id,
                    vec![RawSegment {
                        start_ms: 0,
                        end_ms: 4_000,
                        text: "Uma fala qualquer.".into(),
                        confidence: None,
                    }],
                    Vec::new(),
                ),
            )
            .unwrap();
        service.start_analysis(&id).unwrap();

        let falhou = service
            .fail(
                &id,
                FailedStage::Analysis,
                "O Hermes respondeu num formato que nao deu para ler.",
            )
            .unwrap();
        assert_eq!(falhou.status, MeetingStatus::Failed(FailedStage::Analysis));
        assert_eq!(
            service.transcript(&id).unwrap().len(),
            1,
            "falha de analise nao pode destruir a transcricao"
        );

        let retomada = service.retry(&id).unwrap();
        assert_eq!(retomada.status, MeetingStatus::Transcribed);
        assert!(retomada.failure.is_none());
    }

    #[test]
    fn o_lote_so_oferece_o_que_o_dominio_aprova() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        let id = meeting.id.to_string();
        let segments = transcribe(&storage, &meeting);
        let evidence = vec![MeetingEvidence {
            segment_id: segments[0].id,
            seq: 0,
            char_start: None,
            char_end: None,
        }];

        let mut com_evidencia = insight(
            &meeting,
            InsightKind::MyAction,
            "Finalizar os slides",
            evidence.clone(),
        );
        com_evidencia.confidence = Confidence::High;

        let mut baixa = insight(
            &meeting,
            InsightKind::MyAction,
            "Talvez revisar amanha",
            evidence,
        );
        baixa.confidence = Confidence::Low;

        let sem_evidencia = insight(
            &meeting,
            InsightKind::MyAction,
            "Acao sem proveniencia",
            Vec::new(),
        );
        let decisao = insight(
            &meeting,
            InsightKind::Decision,
            "Uma decisao com evidencia",
            Vec::new(),
        );

        storage
            .replace_analysis(
                analysis(&meeting),
                vec![com_evidencia.clone(), baixa, sem_evidencia, decisao],
            )
            .unwrap();

        let candidatos = service.bulk_candidates(&id).unwrap();
        assert_eq!(candidatos.len(), 1, "so um item entra no lote");
        assert_eq!(candidatos[0].id, com_evidencia.id);
    }

    #[test]
    fn a_limpeza_de_audio_lista_e_marca_na_ordem_certa() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        let id = meeting.id.to_string();
        service.stop(&id).unwrap();
        service
            .settle_audio(
                &id,
                AudioOutcome {
                    duration_ms: 12_000,
                    mic: ChannelOutcome::Captured,
                    system: ChannelOutcome::Captured,
                },
            )
            .unwrap();
        service.start_transcription(&id).unwrap();
        service.finish_transcription(&id, Vec::new()).unwrap();

        // `Transcribed` com retencao padrao ja autoriza: o processamento
        // terminou, mesmo sem analise.
        let fila = service.audio_to_clean().unwrap();
        assert_eq!(fila.len(), 1);
        assert_eq!(fila[0].id, meeting.id);

        service.mark_audio_deleted(&id).unwrap();
        assert!(
            service.audio_to_clean().unwrap().is_empty(),
            "marcada, ela sai da fila e nao volta"
        );
    }

    #[test]
    fn gravar_com_project_inexistente_e_recusado_antes_de_comecar() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let error = service.start("NexoDoc", Some("nao-e-uuid")).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(
            service.recording().unwrap().is_none(),
            "nenhuma gravacao pode ter comecado"
        );
    }

    // ================================================================
    // Aceitar um item: Task, Reminder e vinculo numa transacao so
    // ================================================================

    /// Uma reuniao analisada, com um item acionavel pronto para virar Task.
    fn com_item(
        storage: &Arc<SqliteStorage>,
    ) -> (MeetingService, Meeting, MeetingInsight) {
        let service = service(storage);
        let meeting = service.start("NexoDoc", None).unwrap();
        let segments = transcribe(storage, &meeting);
        let item = insight(
            &meeting,
            InsightKind::MyAction,
            "Finalizar a apresentacao",
            vec![MeetingEvidence {
                segment_id: segments[0].id,
                seq: 0,
                char_start: None,
                char_end: None,
            }],
        );
        storage
            .replace_analysis(analysis(&meeting), vec![item.clone()])
            .unwrap();
        (service, meeting, item)
    }

    fn amanha() -> OffsetDateTime {
        started() + time::Duration::hours(19)
    }

    #[test]
    fn aceitar_cria_task_reminder_e_vinculo_de_uma_vez() {
        let (_dir, storage) = shared();
        let (service, _meeting, item) = com_item(&storage);
        let project = storage
            .create_project(NewProject::create("NexoDoc", "", "").unwrap())
            .unwrap();

        let aceito = service
            .accept_insight(mos_core::AcceptInsight {
                insight_id: item.id,
                title: "Finalizar a apresentacao".into(),
                description: "Slides do comercial".into(),
                project_id: Some(project.id),
                remind_at: Some(amanha()),
            })
            .unwrap();

        // A Task existe, no Project certo.
        let task = storage.get_task(aceito.task_id).unwrap();
        assert_eq!(task.title, "Finalizar a apresentacao");
        assert_eq!(task.project_id, Some(project.id));

        // O Reminder existe e aponta para a Task.
        let reminder_id = aceito.reminder_id.expect("o lembrete foi pedido");
        let reminder = mos_core::AttentionRepository::reminder(&*storage, reminder_id).unwrap();
        assert_eq!(
            reminder.target,
            Some(mos_core::ReminderTarget::Task(aceito.task_id))
        );
        assert_eq!(reminder.next_due_at, Some(amanha()));
        // O corpo cita a REUNIAO: quando ele tocar amanha, "de onde veio isto?"
        // precisa ter resposta sem abrir mais nada.
        assert!(
            reminder.body.contains("NexoDoc"),
            "o corpo precisa citar a reuniao: {:?}",
            reminder.body
        );

        // E o item ficou ligado aos dois.
        assert_eq!(aceito.insight.status, InsightStatus::Accepted);
        assert_eq!(aceito.insight.created_task_id, Some(aceito.task_id));
        assert_eq!(aceito.insight.created_reminder_id, Some(reminder_id));
    }

    #[test]
    fn aceitar_sem_lembrete_cria_so_a_task() {
        let (_dir, storage) = shared();
        let (service, _meeting, item) = com_item(&storage);

        let aceito = service
            .accept_insight(mos_core::AcceptInsight {
                insight_id: item.id,
                title: "So a Task".into(),
                description: String::new(),
                project_id: None,
                remind_at: None,
            })
            .unwrap();

        assert!(aceito.reminder_id.is_none());
        assert!(aceito.insight.created_reminder_id.is_none());
        assert!(storage.get_task(aceito.task_id).is_ok());
    }

    #[test]
    fn aceitar_duas_vezes_e_recusado() {
        // Sem a guarda, duas confirmacoes rapidas criariam duas Tasks para o
        // mesmo compromisso, e a segunda deixaria a primeira orfa.
        let (_dir, storage) = shared();
        let (service, _meeting, item) = com_item(&storage);
        let pedido = || mos_core::AcceptInsight {
            insight_id: item.id,
            title: "Finalizar".into(),
            description: String::new(),
            project_id: None,
            remind_at: None,
        };

        service.accept_insight(pedido()).unwrap();
        let error = service.accept_insight(pedido()).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidTransition);

        let tasks = storage.tasks(false).unwrap();
        assert_eq!(tasks.len(), 1, "so uma Task para um compromisso");
    }

    #[test]
    fn um_lembrete_no_passado_derruba_a_aceitacao_inteira() {
        // A prova da atomicidade: o Reminder e validado no dominio ANTES da
        // transacao, e nada e criado. Sem isso, a Task existiria com um lembrete
        // que nunca vai tocar.
        let (_dir, storage) = shared();
        let (service, _meeting, item) = com_item(&storage);

        let error = service
            .accept_insight(mos_core::AcceptInsight {
                insight_id: item.id,
                title: "Tarde demais".into(),
                description: String::new(),
                project_id: None,
                remind_at: Some(started() - time::Duration::days(2)),
            })
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);

        assert!(storage.tasks(false).unwrap().is_empty(), "nenhuma Task ficou");
        assert_eq!(
            storage.insights(item.meeting_id).unwrap()[0].status,
            InsightStatus::Proposed,
            "o item continua oferecido"
        );
    }

    #[test]
    fn titulo_vazio_derruba_a_aceitacao_e_nao_cria_nada() {
        let (_dir, storage) = shared();
        let (service, _meeting, item) = com_item(&storage);

        let error = service
            .accept_insight(mos_core::AcceptInsight {
                insight_id: item.id,
                title: "   ".into(),
                description: String::new(),
                project_id: None,
                remind_at: Some(amanha()),
            })
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(storage.tasks(false).unwrap().is_empty());
    }

    #[test]
    fn item_inexistente_nao_cria_task() {
        let (_dir, storage) = shared();
        let service = service(&storage);
        let error = service
            .accept_insight(mos_core::AcceptInsight {
                insight_id: InsightId::new(),
                title: "Fantasma".into(),
                description: String::new(),
                project_id: None,
                remind_at: None,
            })
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::NotFound);
        assert!(storage.tasks(false).unwrap().is_empty());
    }

    #[test]
    fn descartar_tira_da_lista_e_nao_apaga() {
        let (_dir, storage) = shared();
        let (service, meeting, item) = com_item(&storage);

        let descartado = service.dismiss_insight(&item.id.to_string()).unwrap();
        assert_eq!(descartado.status, InsightStatus::Dismissed);
        // Continua no banco: descartar nao e apagar.
        assert_eq!(storage.insights(meeting.id).unwrap().len(), 1);
        // E deixa de ser oferecido no lote.
        assert!(service.bulk_candidates(&meeting.id.to_string()).unwrap().is_empty());
    }

    #[test]
    fn reabrir_devolve_o_item_ao_lote() {
        // E a metade do desfazer que mora no dominio da reuniao.
        let (_dir, storage) = shared();
        let (service, meeting, item) = com_item(&storage);
        service
            .accept_insight(mos_core::AcceptInsight {
                insight_id: item.id,
                title: "Finalizar".into(),
                description: String::new(),
                project_id: None,
                remind_at: None,
            })
            .unwrap();
        assert!(service.bulk_candidates(&meeting.id.to_string()).unwrap().is_empty());

        let reaberto = service.reopen_insight(&item.id.to_string()).unwrap();
        assert_eq!(reaberto.status, InsightStatus::Proposed);
        assert_eq!(
            service.bulk_candidates(&meeting.id.to_string()).unwrap().len(),
            1,
            "quem desfez provavelmente quer refazer diferente"
        );
    }

    #[test]
    fn o_preview_sai_pelo_servico_com_a_razao_do_bloqueio() {
        let (_dir, storage) = shared();
        let (service, meeting, _item) = com_item(&storage);
        // Um segundo item, sem evidencia.
        let orfao = insight(&meeting, InsightKind::MyAction, "Sem proveniencia", Vec::new());
        let segments = storage.transcript(meeting.id).unwrap();
        let bom = insight(
            &meeting,
            InsightKind::MyAction,
            "Com proveniencia",
            vec![MeetingEvidence {
                segment_id: segments[0].id,
                seq: 0,
                char_start: None,
                char_end: None,
            }],
        );
        storage
            .replace_analysis(analysis(&meeting), vec![bom, orfao])
            .unwrap();

        let previews = service.previews(&meeting.id.to_string()).unwrap();
        let sem = previews
            .iter()
            .find(|preview| preview.title == "Sem proveniencia")
            .unwrap();
        assert!(!sem.eligible_for_bulk);
        assert!(sem.blocked_reason.contains("evidencia"));

        let com = previews
            .iter()
            .find(|preview| preview.title == "Com proveniencia")
            .unwrap();
        assert!(com.eligible_for_bulk);
        assert!(com.blocked_reason.is_empty());
    }

    #[test]
    fn o_titulo_vazio_e_recusado() {
        let (_dir, storage) = storage();
        let meeting = start_meeting(&storage, "NexoDoc");
        let error = storage.set_meeting_title(meeting.id, "   ").unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn reuniao_inexistente_devolve_not_found() {
        let (_dir, storage) = storage();
        let error = storage.meeting(MeetingId::new()).unwrap_err();
        assert_eq!(error.code, ErrorCode::NotFound);
    }
}
