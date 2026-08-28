//! Persistencia do M/Academic.
//!
//! # As duas transacoes que este arquivo existe para garantir
//!
//! **Criar a Task de uma atividade e um gesto so.** `create_task_for_assignment`
//! grava a Task e o vinculo no mesmo commit. Em sequencia, uma queda no meio
//! deixaria uma Task orfa que ninguem relaciona de volta a faculdade, ou um
//! vinculo apontando para uma Task que nao existe.
//!
//! **Comecar a estudar tambem.** `start_study` fecha a sessao que ficou aberta
//! e abre a nova junto. Duas sessoes abertas fariam o "quanto estudei hoje"
//! somar o mesmo minuto duas vezes — e o indice unico do banco recusaria a
//! segunda, deixando a pessoa sem conseguir estudar ate descobrir onde estava a
//! sessao esquecida.
//!
//! # Emissao
//!
//! Toda mutacao emite a operacao de sincronizacao dentro da mesma transacao,
//! pela regra do `sync_emit.rs`. Cinco tipos novos viajam: `academic_semester`,
//! `academic_subject`, `academic_assignment`, `academic_exam` e
//! `academic_study_session`. O vinculo com material viaja como RELACAO
//! (`emitir_relacao`), e nao como campo: ligar o mesmo PDF a duas disciplinas em
//! dois dispositivos precisa terminar com os dois vinculos de pe, e merge por
//! campo nao serve para conjunto — e a mesma regra da §13 do `SYNC.md`.

use mos_core::{
    AcademicRepository, Assignment, AssignmentId, AssignmentStatus, CoreError, Day, Decision,
    ErrorCode, Exam, ExamId, ExamStatus, LifecycleState, NewAssignment, NewExam, NewSemester,
    NewSubject, NewTask, Plano, Pontuacao, Priority, Resource, ResourceId, Semester, SemesterId,
    StudySession, StudySessionId, Subject, SubjectId, Task, TaskId, UpdateAssignment, UpdateExam,
};
use rusqlite::{params, Connection, Row};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time},
    SqliteStorage,
};

const KIND_SEMESTER: &str = "academic_semester";
const KIND_SUBJECT: &str = "academic_subject";
const KIND_ASSIGNMENT: &str = "academic_assignment";
const KIND_EXAM: &str = "academic_exam";
const KIND_STUDY: &str = "academic_study_session";
/// O tipo da RELACAO disciplina→material, no vocabulario do Knowledge Graph.
const REL_MATERIAL: &str = "academic_subject_resource";

const SEMESTER_COLUMNS: &str =
    "id, name, institution, starts_on, ends_on, lifecycle_state, created_at, updated_at";
const SUBJECT_COLUMNS: &str = "id, semester_id, name, code, teacher, accent, notes, \
     lifecycle_state, created_at, updated_at";
const ASSIGNMENT_COLUMNS: &str = "id, subject_id, title, description, due_at, status, priority, \
     weight, max_score, score, task_id, lifecycle_state, created_at, updated_at,      decision, decided_at, planned_at, planned_minutes";
const EXAM_COLUMNS: &str = "id, subject_id, name, at, location, topics, weight, max_score, \
     score, status, lifecycle_state, created_at, updated_at,      decision, decided_at, planned_at, planned_minutes";
const STUDY_COLUMNS: &str =
    "id, subject_id, topic, notes, started_at, ended_at, seconds, created_at, updated_at";

/// Quantas colunas uma lista declara.
///
/// A busca junta a disciplina DEPOIS das colunas da entidade, e precisa saber
/// em que indice ela cai. Cravar o numero a mao funciona ate alguem acrescentar
/// uma coluna — foi exatamente o que a 0034 fez, e o teste da busca foi o unico
/// a perceber.
fn colunas_de(lista: &str) -> usize {
    lista.split(',').count()
}

fn not_found(what: &str) -> CoreError {
    CoreError::new(ErrorCode::NotFound, what, false)
}

fn lifecycle_filter(include_archived: bool) -> &'static str {
    if include_archived {
        "IN ('active', 'archived')"
    } else {
        "= 'active'"
    }
}

// ===========================================================================
// Leitura
// ===========================================================================

fn read_semester(row: &Row<'_>) -> rusqlite::Result<Result<Semester, CoreError>> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let institution: String = row.get(2)?;
    let starts_on: String = row.get(3)?;
    let ends_on: String = row.get(4)?;
    let lifecycle: String = row.get(5)?;
    let created_at: String = row.get(6)?;
    let updated_at: String = row.get(7)?;
    Ok((|| {
        Ok(Semester {
            id: SemesterId::parse(&id)?,
            name,
            institution,
            starts_on: Day::parse(&starts_on)?,
            ends_on: Day::parse(&ends_on)?,
            lifecycle_state: LifecycleState::parse(&lifecycle)?,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}

fn read_subject(row: &Row<'_>) -> rusqlite::Result<Result<Subject, CoreError>> {
    let id: String = row.get(0)?;
    let semester_id: String = row.get(1)?;
    let name: String = row.get(2)?;
    let code: String = row.get(3)?;
    let teacher: String = row.get(4)?;
    let accent: String = row.get(5)?;
    let notes: String = row.get(6)?;
    let lifecycle: String = row.get(7)?;
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;
    Ok((|| {
        Ok(Subject {
            id: SubjectId::parse(&id)?,
            semester_id: SemesterId::parse(&semester_id)?,
            name,
            code,
            teacher,
            accent,
            notes,
            lifecycle_state: LifecycleState::parse(&lifecycle)?,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}

fn read_assignment(row: &Row<'_>) -> rusqlite::Result<Result<Assignment, CoreError>> {
    let id: String = row.get(0)?;
    let subject_id: String = row.get(1)?;
    let title: String = row.get(2)?;
    let description: String = row.get(3)?;
    let due_at: Option<String> = row.get(4)?;
    let status: String = row.get(5)?;
    let priority: String = row.get(6)?;
    let weight: f64 = row.get(7)?;
    let max_score: Option<f64> = row.get(8)?;
    let score: Option<f64> = row.get(9)?;
    let task_id: Option<String> = row.get(10)?;
    let lifecycle: String = row.get(11)?;
    let created_at: String = row.get(12)?;
    let updated_at: String = row.get(13)?;
    let decision: String = row.get(14)?;
    let decided_at: Option<String> = row.get(15)?;
    let planned_at: Option<String> = row.get(16)?;
    let planned_minutes: i64 = row.get(17)?;
    Ok((|| {
        Ok(Assignment {
            id: AssignmentId::parse(&id)?,
            subject_id: SubjectId::parse(&subject_id)?,
            title,
            description,
            due_at: due_at.as_deref().map(parse_time).transpose()?,
            status: AssignmentStatus::parse(&status)?,
            priority: Priority::parse(&priority)?,
            weight,
            max_score,
            score,
            task_id: task_id.as_deref().map(TaskId::parse).transpose()?,
            decision: Decision::parse(&decision)?,
            decided_at: decided_at.as_deref().map(parse_time).transpose()?,
            planned_at: planned_at.as_deref().map(parse_time).transpose()?,
            planned_minutes,
            lifecycle_state: LifecycleState::parse(&lifecycle)?,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}

fn read_exam(row: &Row<'_>) -> rusqlite::Result<Result<Exam, CoreError>> {
    let id: String = row.get(0)?;
    let subject_id: String = row.get(1)?;
    let name: String = row.get(2)?;
    let at: String = row.get(3)?;
    let location: String = row.get(4)?;
    let topics: String = row.get(5)?;
    let weight: f64 = row.get(6)?;
    let max_score: Option<f64> = row.get(7)?;
    let score: Option<f64> = row.get(8)?;
    let status: String = row.get(9)?;
    let lifecycle: String = row.get(10)?;
    let created_at: String = row.get(11)?;
    let updated_at: String = row.get(12)?;
    let decision: String = row.get(13)?;
    let decided_at: Option<String> = row.get(14)?;
    let planned_at: Option<String> = row.get(15)?;
    let planned_minutes: i64 = row.get(16)?;
    Ok((|| {
        Ok(Exam {
            id: ExamId::parse(&id)?,
            subject_id: SubjectId::parse(&subject_id)?,
            name,
            at: parse_time(&at)?,
            location,
            topics,
            weight,
            max_score,
            score,
            status: ExamStatus::parse(&status)?,
            decision: Decision::parse(&decision)?,
            decided_at: decided_at.as_deref().map(parse_time).transpose()?,
            planned_at: planned_at.as_deref().map(parse_time).transpose()?,
            planned_minutes,
            lifecycle_state: LifecycleState::parse(&lifecycle)?,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}

fn read_study(row: &Row<'_>) -> rusqlite::Result<Result<StudySession, CoreError>> {
    let id: String = row.get(0)?;
    let subject_id: String = row.get(1)?;
    let topic: String = row.get(2)?;
    let notes: String = row.get(3)?;
    let started_at: String = row.get(4)?;
    let ended_at: Option<String> = row.get(5)?;
    let seconds: i64 = row.get(6)?;
    let created_at: String = row.get(7)?;
    let updated_at: String = row.get(8)?;
    Ok((|| {
        Ok(StudySession {
            id: StudySessionId::parse(&id)?,
            subject_id: SubjectId::parse(&subject_id)?,
            topic,
            notes,
            started_at: parse_time(&started_at)?,
            ended_at: ended_at.as_deref().map(parse_time).transpose()?,
            seconds,
            created_at: parse_time(&created_at)?,
            updated_at: parse_time(&updated_at)?,
        })
    })())
}

macro_rules! consulta {
    ($nome:ident, $tipo:ty, $leitor:ident) => {
        fn $nome(connection: &Connection, sql: &str) -> Result<Vec<$tipo>, CoreError> {
            let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
            let rows = statement.query_map([], $leitor).map_err(map_sql_error)?;
            let mut itens = Vec::new();
            for row in rows {
                itens.push(row.map_err(map_sql_error)??);
            }
            Ok(itens)
        }
    };
}

consulta!(query_semesters, Semester, read_semester);
consulta!(query_subjects, Subject, read_subject);
consulta!(query_assignments, Assignment, read_assignment);
consulta!(query_exams, Exam, read_exam);
consulta!(query_studies, StudySession, read_study);

fn one_semester(connection: &Connection, id: SemesterId) -> Result<Semester, CoreError> {
    query_semesters(
        connection,
        &format!(
            "SELECT {SEMESTER_COLUMNS} FROM academic_semesters WHERE id = '{}'",
            id
        ),
    )?
    .pop()
    .ok_or_else(|| not_found("Semestre nao encontrado."))
}

fn one_subject(connection: &Connection, id: SubjectId) -> Result<Subject, CoreError> {
    query_subjects(
        connection,
        &format!(
            "SELECT {SUBJECT_COLUMNS} FROM academic_subjects WHERE id = '{}'",
            id
        ),
    )?
    .pop()
    .ok_or_else(|| not_found("Disciplina nao encontrada."))
}

fn one_assignment(connection: &Connection, id: AssignmentId) -> Result<Assignment, CoreError> {
    query_assignments(
        connection,
        &format!(
            "SELECT {ASSIGNMENT_COLUMNS} FROM academic_assignments WHERE id = '{}'",
            id
        ),
    )?
    .pop()
    .ok_or_else(|| not_found("Atividade nao encontrada."))
}

fn one_exam(connection: &Connection, id: ExamId) -> Result<Exam, CoreError> {
    query_exams(
        connection,
        &format!(
            "SELECT {EXAM_COLUMNS} FROM academic_exams WHERE id = '{}'",
            id
        ),
    )?
    .pop()
    .ok_or_else(|| not_found("Avaliacao nao encontrada."))
}

fn one_study(connection: &Connection, id: StudySessionId) -> Result<StudySession, CoreError> {
    query_studies(
        connection,
        &format!(
            "SELECT {STUDY_COLUMNS} FROM academic_study_sessions WHERE id = '{}'",
            id
        ),
    )?
    .pop()
    .ok_or_else(|| not_found("Sessao de estudo nao encontrada."))
}

// ===========================================================================
// A porta
// ===========================================================================

impl AcademicRepository for SqliteStorage {
    fn semesters(&self, include_archived: bool) -> Result<Vec<Semester>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_semesters(
            &connection,
            &format!(
                "SELECT {SEMESTER_COLUMNS} FROM academic_semesters
                 WHERE lifecycle_state {} ORDER BY starts_on DESC",
                lifecycle_filter(include_archived)
            ),
        )
    }

    fn create_semester(&self, semester: NewSemester) -> Result<Semester, CoreError> {
        let id = semester.id;
        let now = format_time(semester.created_at)?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO academic_semesters (
                    id, name, institution, starts_on, ends_on, lifecycle_state, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?6)",
                params![
                    id.to_string(),
                    semester.name,
                    semester.institution,
                    semester.starts_on.as_str(),
                    semester.ends_on.as_str(),
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new(KIND_SEMESTER, id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    ("name".to_owned(), serde_json::json!(semester.name)),
                    (
                        "institution".to_owned(),
                        serde_json::json!(semester.institution),
                    ),
                    (
                        "startsOn".to_owned(),
                        serde_json::json!(semester.starts_on.as_str()),
                    ),
                    (
                        "endsOn".to_owned(),
                        serde_json::json!(semester.ends_on.as_str()),
                    ),
                    ("createdAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_semester(&connection, id)
    }

    fn update_semester(
        &self,
        id: SemesterId,
        name: &str,
        institution: &str,
        starts_on: &Day,
        ends_on: &Day,
    ) -> Result<Semester, CoreError> {
        if ends_on < starts_on {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "O semestre termina antes de comecar.",
                false,
            ));
        }
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_semesters
                 SET name = ?1, institution = ?2, starts_on = ?3, ends_on = ?4, updated_at = ?5
                 WHERE id = ?6",
                params![
                    name,
                    institution,
                    starts_on.as_str(),
                    ends_on.as_str(),
                    now,
                    id.to_string()
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Semestre nao encontrado."));
        }
        self.emitir_update(
            &transaction,
            KIND_SEMESTER,
            id.as_uuid(),
            &[
                ("name", serde_json::json!(name)),
                ("institution", serde_json::json!(institution)),
                ("startsOn", serde_json::json!(starts_on.as_str())),
                ("endsOn", serde_json::json!(ends_on.as_str())),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_semester(&connection, id)
    }

    fn set_semester_lifecycle(
        &self,
        id: SemesterId,
        state: LifecycleState,
    ) -> Result<Semester, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_semesters SET lifecycle_state = ?1, updated_at = ?2 WHERE id = ?3",
                params![state.as_str(), now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Semestre nao encontrado."));
        }
        // Arquivar e mudanca de CAMPO, e nunca `OpBody::Delete`: o Delete tem
        // semantica de "apagar ganha de editar", e o semestre arquivado por
        // engano precisa poder voltar. Mesma regra da §12 do `SYNC.md`.
        self.emitir_update(
            &transaction,
            KIND_SEMESTER,
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(state.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_semester(&connection, id)
    }

    fn subjects(&self, include_archived: bool) -> Result<Vec<Subject>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_subjects(
            &connection,
            &format!(
                "SELECT {SUBJECT_COLUMNS} FROM academic_subjects
                 WHERE lifecycle_state {} ORDER BY name",
                lifecycle_filter(include_archived)
            ),
        )
    }

    fn create_subject(&self, subject: NewSubject) -> Result<Subject, CoreError> {
        let id = subject.id;
        let now = format_time(subject.created_at)?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO academic_subjects (
                    id, semester_id, name, code, teacher, accent, notes,
                    lifecycle_state, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?8)",
                params![
                    id.to_string(),
                    subject.semester_id.to_string(),
                    subject.name,
                    subject.code,
                    subject.teacher,
                    subject.accent,
                    subject.notes,
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new(KIND_SUBJECT, id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    (
                        "semesterId".to_owned(),
                        serde_json::json!(subject.semester_id.to_string()),
                    ),
                    ("name".to_owned(), serde_json::json!(subject.name)),
                    ("code".to_owned(), serde_json::json!(subject.code)),
                    ("teacher".to_owned(), serde_json::json!(subject.teacher)),
                    ("accent".to_owned(), serde_json::json!(subject.accent)),
                    ("notes".to_owned(), serde_json::json!(subject.notes)),
                    ("createdAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_subject(&connection, id)
    }

    fn update_subject(
        &self,
        id: SubjectId,
        name: &str,
        code: &str,
        teacher: &str,
        accent: &str,
        notes: &str,
    ) -> Result<Subject, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_subjects
                 SET name = ?1, code = ?2, teacher = ?3, accent = ?4, notes = ?5, updated_at = ?6
                 WHERE id = ?7",
                params![name, code, teacher, accent, notes, now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Disciplina nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_SUBJECT,
            id.as_uuid(),
            &[
                ("name", serde_json::json!(name)),
                ("code", serde_json::json!(code)),
                ("teacher", serde_json::json!(teacher)),
                ("accent", serde_json::json!(accent)),
                ("notes", serde_json::json!(notes)),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_subject(&connection, id)
    }

    fn set_subject_lifecycle(
        &self,
        id: SubjectId,
        state: LifecycleState,
    ) -> Result<Subject, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_subjects SET lifecycle_state = ?1, updated_at = ?2 WHERE id = ?3",
                params![state.as_str(), now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Disciplina nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_SUBJECT,
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(state.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_subject(&connection, id)
    }

    fn assignments(&self, include_archived: bool) -> Result<Vec<Assignment>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_assignments(
            &connection,
            &format!(
                "SELECT {ASSIGNMENT_COLUMNS} FROM academic_assignments
                 WHERE lifecycle_state {}
                 ORDER BY due_at IS NULL, due_at, created_at",
                lifecycle_filter(include_archived)
            ),
        )
    }

    fn create_assignment(&self, assignment: NewAssignment) -> Result<Assignment, CoreError> {
        let id = assignment.id;
        let now = format_time(assignment.created_at)?;
        let due = assignment.due_at.map(format_time).transpose()?;
        let (score, max_score) = assignment
            .pontuacao
            .map(Pontuacao::colunas)
            .unwrap_or((None, None));
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO academic_assignments (
                    id, subject_id, title, description, due_at, status, priority,
                    weight, max_score, score, lifecycle_state, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?7, ?8, ?9, 'active', ?10, ?10)",
                params![
                    id.to_string(),
                    assignment.subject_id.to_string(),
                    assignment.title,
                    assignment.description,
                    due,
                    assignment.priority.as_str(),
                    assignment.weight,
                    max_score,
                    score,
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new(KIND_ASSIGNMENT, id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    (
                        "subjectId".to_owned(),
                        serde_json::json!(assignment.subject_id.to_string()),
                    ),
                    ("title".to_owned(), serde_json::json!(assignment.title)),
                    (
                        "description".to_owned(),
                        serde_json::json!(assignment.description),
                    ),
                    ("dueAt".to_owned(), serde_json::json!(due)),
                    ("status".to_owned(), serde_json::json!("pending")),
                    (
                        "priority".to_owned(),
                        serde_json::json!(assignment.priority.as_str()),
                    ),
                    ("weight".to_owned(), serde_json::json!(assignment.weight)),
                    ("maxScore".to_owned(), serde_json::json!(max_score)),
                    ("score".to_owned(), serde_json::json!(score)),
                    ("createdAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_assignment(&connection, id)
    }

    fn update_assignment(&self, input: UpdateAssignment) -> Result<Assignment, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let due = input.due_at.map(format_time).transpose()?;
        let (score, max_score) = Pontuacao::nova(input.score, input.max_score)?
            .map(Pontuacao::colunas)
            .unwrap_or((None, None));
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_assignments
                 SET title = ?1, description = ?2, due_at = ?3, priority = ?4, weight = ?5,
                     max_score = ?6, score = ?7, status = ?8, updated_at = ?9
                 WHERE id = ?10",
                params![
                    input.title,
                    input.description,
                    due,
                    input.priority.as_str(),
                    input.weight,
                    max_score,
                    score,
                    input.status.as_str(),
                    now,
                    input.id.to_string(),
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Atividade nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_ASSIGNMENT,
            input.id.as_uuid(),
            &[
                ("title", serde_json::json!(input.title)),
                ("description", serde_json::json!(input.description)),
                ("dueAt", serde_json::json!(due)),
                ("priority", serde_json::json!(input.priority.as_str())),
                ("weight", serde_json::json!(input.weight)),
                ("maxScore", serde_json::json!(max_score)),
                ("score", serde_json::json!(score)),
                ("status", serde_json::json!(input.status.as_str())),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_assignment(&connection, input.id)
    }

    fn set_assignment_status(
        &self,
        id: AssignmentId,
        status: AssignmentStatus,
    ) -> Result<Assignment, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let atual = one_assignment(&transaction, id)?;
        transaction
            .execute(
                "UPDATE academic_assignments SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status.as_str(), now, id.to_string()],
            )
            .map_err(map_sql_error)?;

        // A Task ligada acompanha. Sem isto os dois lados divergem em silencio:
        // a atividade diria "entregue" e a Task continuaria no quadro pedindo
        // acao — que e exatamente a sincronizacao fragil que o §14 do pedido
        // proibe.
        if let Some(task_id) = atual.task_id {
            let destino = if status.is_settled() { "done" } else { "doing" };
            let ja: String = transaction
                .query_row(
                    "SELECT work_state FROM tasks WHERE id = ?1",
                    params![task_id.to_string()],
                    |row| row.get(0),
                )
                .map_err(map_sql_error)?;
            // So mexe quando ha o que mudar: concluir uma atividade ja entregue
            // nao pode reabrir uma Task que a pessoa arrastou para outra coluna.
            if (destino == "done") != (ja == "done") {
                let completed = (destino == "done").then_some(now.as_str());
                transaction
                    .execute(
                        "UPDATE tasks SET work_state = ?1, updated_at = ?2, completed_at = ?3
                         WHERE id = ?4",
                        params![destino, now, completed, task_id.to_string()],
                    )
                    .map_err(map_sql_error)?;
                self.emitir_update(
                    &transaction,
                    "task",
                    task_id.as_uuid(),
                    &[("workState", serde_json::json!(destino))],
                )?;
            }
        }

        self.emitir_update(
            &transaction,
            KIND_ASSIGNMENT,
            id.as_uuid(),
            &[("status", serde_json::json!(status.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_assignment(&connection, id)
    }

    fn set_assignment_lifecycle(
        &self,
        id: AssignmentId,
        state: LifecycleState,
    ) -> Result<Assignment, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_assignments SET lifecycle_state = ?1, updated_at = ?2 WHERE id = ?3",
                params![state.as_str(), now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Atividade nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_ASSIGNMENT,
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(state.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_assignment(&connection, id)
    }

    fn create_task_for_assignment(&self, id: AssignmentId) -> Result<Task, CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let assignment = one_assignment(&transaction, id)?;
        if assignment.task_id.is_some() {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Esta atividade ja tem uma Task.",
                false,
            ));
        }
        let subject = one_subject(&transaction, assignment.subject_id)?;

        // O titulo diz a DISCIPLINA junto: no quadro, "Lista 03" sozinha nao se
        // distingue da lista 03 de outra materia, e o Kanban nao mostra de onde
        // a Task veio.
        let titulo = format!("{} — {}", subject.name, assignment.title);
        let task = NewTask::create(&titulo, &assignment.description, None)?;
        let task_id = task.id;
        crate::work_repository::insert_task(self, &transaction, task, None)?;

        let now = format_time(OffsetDateTime::now_utc())?;
        transaction
            .execute(
                "UPDATE academic_assignments SET task_id = ?1, updated_at = ?2 WHERE id = ?3",
                params![task_id.to_string(), now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        self.emitir_update(
            &transaction,
            KIND_ASSIGNMENT,
            id.as_uuid(),
            &[("taskId", serde_json::json!(task_id.to_string()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        crate::work_repository::query_task(&connection, task_id)
    }

    fn unlink_assignment_task(&self, id: AssignmentId) -> Result<Assignment, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_assignments SET task_id = NULL, updated_at = ?1 WHERE id = ?2",
                params![now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Atividade nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_ASSIGNMENT,
            id.as_uuid(),
            &[("taskId", serde_json::Value::Null)],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_assignment(&connection, id)
    }

    fn exams(&self, include_archived: bool) -> Result<Vec<Exam>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_exams(
            &connection,
            &format!(
                "SELECT {EXAM_COLUMNS} FROM academic_exams
                 WHERE lifecycle_state {} ORDER BY at",
                lifecycle_filter(include_archived)
            ),
        )
    }

    fn create_exam(&self, exam: NewExam) -> Result<Exam, CoreError> {
        let id = exam.id;
        let now = format_time(exam.created_at)?;
        let at = format_time(exam.at)?;
        let (score, max_score) = exam
            .pontuacao
            .map(Pontuacao::colunas)
            .unwrap_or((None, None));
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT INTO academic_exams (
                    id, subject_id, name, at, location, topics, weight, max_score, score,
                    status, lifecycle_state, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'scheduled', 'active', ?10, ?10)",
                params![
                    id.to_string(),
                    exam.subject_id.to_string(),
                    exam.name,
                    at,
                    exam.location,
                    exam.topics,
                    exam.weight,
                    max_score,
                    score,
                    now,
                ],
            )
            .map_err(map_sql_error)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new(KIND_EXAM, id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    (
                        "subjectId".to_owned(),
                        serde_json::json!(exam.subject_id.to_string()),
                    ),
                    ("name".to_owned(), serde_json::json!(exam.name)),
                    ("at".to_owned(), serde_json::json!(at)),
                    ("location".to_owned(), serde_json::json!(exam.location)),
                    ("topics".to_owned(), serde_json::json!(exam.topics)),
                    ("weight".to_owned(), serde_json::json!(exam.weight)),
                    ("maxScore".to_owned(), serde_json::json!(max_score)),
                    ("score".to_owned(), serde_json::json!(score)),
                    ("status".to_owned(), serde_json::json!("scheduled")),
                    ("createdAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_exam(&connection, id)
    }

    fn update_exam(&self, input: UpdateExam) -> Result<Exam, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let at = format_time(input.at)?;
        let (score, max_score) = Pontuacao::nova(input.score, input.max_score)?
            .map(Pontuacao::colunas)
            .unwrap_or((None, None));
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_exams
                 SET name = ?1, at = ?2, location = ?3, topics = ?4, weight = ?5,
                     max_score = ?6, score = ?7, status = ?8, updated_at = ?9
                 WHERE id = ?10",
                params![
                    input.name,
                    at,
                    input.location,
                    input.topics,
                    input.weight,
                    max_score,
                    score,
                    input.status.as_str(),
                    now,
                    input.id.to_string(),
                ],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Avaliacao nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_EXAM,
            input.id.as_uuid(),
            &[
                ("name", serde_json::json!(input.name)),
                ("at", serde_json::json!(at)),
                ("location", serde_json::json!(input.location)),
                ("topics", serde_json::json!(input.topics)),
                ("weight", serde_json::json!(input.weight)),
                ("maxScore", serde_json::json!(max_score)),
                ("score", serde_json::json!(score)),
                ("status", serde_json::json!(input.status.as_str())),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_exam(&connection, input.id)
    }

    fn set_exam_lifecycle(&self, id: ExamId, state: LifecycleState) -> Result<Exam, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_exams SET lifecycle_state = ?1, updated_at = ?2 WHERE id = ?3",
                params![state.as_str(), now, id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(not_found("Avaliacao nao encontrada."));
        }
        self.emitir_update(
            &transaction,
            KIND_EXAM,
            id.as_uuid(),
            &[("lifecycleState", serde_json::json!(state.as_str()))],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_exam(&connection, id)
    }

    fn subject_resources(&self, id: SubjectId) -> Result<Vec<Resource>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        crate::resource_repository::query_resources(
            &connection,
            &format!(
                "SELECT r.{columns} FROM academic_subject_resources j
                 JOIN resources r ON r.id = j.resource_id
                 WHERE j.subject_id = '{id}' AND r.lifecycle_state = 'active'
                 ORDER BY j.created_at DESC",
                columns = crate::resource_repository::RESOURCE_COLUMNS.replace(", ", ", r."),
            ),
        )
    }

    fn material_counts(&self) -> Result<Vec<(SubjectId, usize)>, CoreError> {
        // Uma consulta agregada, e nao uma por disciplina: o painel pede a
        // contagem de todas de uma vez, e N+1 aqui seria uma consulta por
        // materia a cada refresh da Home.
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT j.subject_id, count(*) FROM academic_subject_resources j
                 JOIN resources r ON r.id = j.resource_id
                 WHERE r.lifecycle_state = 'active'
                 GROUP BY j.subject_id",
            )
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(map_sql_error)?;
        let mut contagens = Vec::new();
        for row in rows {
            let (id, quantos) = row.map_err(map_sql_error)?;
            contagens.push((SubjectId::parse(&id)?, quantos.max(0) as usize));
        }
        Ok(contagens)
    }

    fn link_material(
        &self,
        subject: SubjectId,
        resource: ResourceId,
        linked: bool,
    ) -> Result<(), CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        if linked {
            transaction
                .execute(
                    "INSERT OR IGNORE INTO academic_subject_resources (subject_id, resource_id, created_at)
                     VALUES (?1, ?2, ?3)",
                    params![subject.to_string(), resource.to_string(), now],
                )
                .map_err(map_sql_error)?;
        } else {
            transaction
                .execute(
                    "DELETE FROM academic_subject_resources WHERE subject_id = ?1 AND resource_id = ?2",
                    params![subject.to_string(), resource.to_string()],
                )
                .map_err(map_sql_error)?;
        }
        self.emitir_relacao(
            &transaction,
            REL_MATERIAL,
            subject.as_uuid(),
            resource.as_uuid(),
            linked,
        )?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    fn study_sessions(&self, limit: usize) -> Result<Vec<StudySession>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        query_studies(
            &connection,
            &format!(
                "SELECT {STUDY_COLUMNS} FROM academic_study_sessions
                 ORDER BY started_at DESC LIMIT {limit}"
            ),
        )
    }

    fn running_study(&self) -> Result<Option<StudySession>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        Ok(query_studies(
            &connection,
            &format!(
                "SELECT {STUDY_COLUMNS} FROM academic_study_sessions
                 WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1"
            ),
        )?
        .pop())
    }

    fn start_study(&self, subject: SubjectId, topic: &str) -> Result<StudySession, CoreError> {
        let id = StudySessionId::new();
        let agora = OffsetDateTime::now_utc();
        let now = format_time(agora)?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        // Fecha a que ficou aberta, com o tempo de parede.
        //
        // O app pode ter sido fechado com o cronometro rodando; recusar a nova
        // sessao por causa disso deixaria a pessoa travada sem entender o
        // motivo. Fechar pelo relogio e uma aproximacao, e e melhor que o
        // impedimento — ela pode corrigir os minutos depois.
        let abertas = query_studies(
            &transaction,
            &format!("SELECT {STUDY_COLUMNS} FROM academic_study_sessions WHERE ended_at IS NULL"),
        )?;
        for aberta in abertas {
            let segundos = (agora - aberta.started_at).whole_seconds().max(0);
            transaction
                .execute(
                    "UPDATE academic_study_sessions
                     SET ended_at = ?1, seconds = ?2, updated_at = ?1 WHERE id = ?3",
                    params![now, segundos, aberta.id.to_string()],
                )
                .map_err(map_sql_error)?;
            self.emitir_update(
                &transaction,
                KIND_STUDY,
                aberta.id.as_uuid(),
                &[
                    ("endedAt", serde_json::json!(now)),
                    ("seconds", serde_json::json!(segundos)),
                ],
            )?;
        }

        transaction
            .execute(
                "INSERT INTO academic_study_sessions (
                    id, subject_id, topic, notes, started_at, seconds, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, '', ?4, 0, ?4, ?4)",
                params![id.to_string(), subject.to_string(), topic.trim(), now],
            )
            .map_err(map_sql_error)?;
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new(KIND_STUDY, id.as_uuid()),
            mos_sync::OpBody::Create {
                fields: [
                    (
                        "subjectId".to_owned(),
                        serde_json::json!(subject.to_string()),
                    ),
                    ("topic".to_owned(), serde_json::json!(topic.trim())),
                    ("startedAt".to_owned(), serde_json::json!(now)),
                    ("createdAt".to_owned(), serde_json::json!(now)),
                ]
                .into_iter()
                .collect(),
            },
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_study(&connection, id)
    }

    fn finish_study(
        &self,
        id: StudySessionId,
        seconds: i64,
        notes: &str,
    ) -> Result<StudySession, CoreError> {
        if seconds < 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Uma sessao nao pode durar tempo negativo.",
                false,
            ));
        }
        let now = format_time(OffsetDateTime::now_utc())?;
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let changed = transaction
            .execute(
                "UPDATE academic_study_sessions
                 SET ended_at = ?1, seconds = ?2, notes = ?3, updated_at = ?1
                 WHERE id = ?4 AND ended_at IS NULL",
                params![now, seconds, notes.trim(), id.to_string()],
            )
            .map_err(map_sql_error)?;
        if changed == 0 {
            return Err(CoreError::new(
                ErrorCode::InvalidTransition,
                "Esta sessao de estudo ja foi encerrada.",
                false,
            ));
        }
        self.emitir_update(
            &transaction,
            KIND_STUDY,
            id.as_uuid(),
            &[
                ("endedAt", serde_json::json!(now)),
                ("seconds", serde_json::json!(seconds)),
                ("notes", serde_json::json!(notes.trim())),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_study(&connection, id)
    }

    fn search_academic(
        &self,
        request: mos_core::SearchRequest,
    ) -> Result<Vec<mos_core::SearchItem>, CoreError> {
        let termo = request.query.trim();
        if termo.is_empty() {
            return Ok(Vec::new());
        }
        // O escape e o mesmo do `search_objectives`: sem ele, um `%` digitado
        // vira curinga e a busca por "50%" devolve tudo.
        let padrao = format!(
            "%{}%",
            termo
                .replace('\u{5C}', "\u{5C}\u{5C}")
                .replace('%', "\u{5C}%")
                .replace('_', "\u{5C}_")
        );
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let limite = request.limit.max(1);

        let mut statement = connection
            .prepare(&format!(
                "SELECT {SUBJECT_COLUMNS} FROM academic_subjects
                 WHERE lifecycle_state = 'active'
                   AND (name LIKE ?1 ESCAPE '\\' OR code LIKE ?1 ESCAPE '\\'
                        OR teacher LIKE ?1 ESCAPE '\\')
                 ORDER BY name LIMIT {limite}"
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![padrao], read_subject)
            .map_err(map_sql_error)?;
        let mut subjects = Vec::new();
        for row in rows {
            subjects.push(row.map_err(map_sql_error)??);
        }
        drop(statement);

        // O nome da disciplina vem no JOIN, e nao numa consulta por acerto: dez
        // provas encontradas fariam dez idas ao banco so para escrever o nome.
        let mut statement = connection
            .prepare(&format!(
                "SELECT e.{colunas}, s.name FROM academic_exams e
                 JOIN academic_subjects s ON s.id = e.subject_id
                 WHERE e.lifecycle_state = 'active' AND s.lifecycle_state = 'active'
                   AND (e.name LIKE ?1 ESCAPE '\\' OR e.topics LIKE ?1 ESCAPE '\\'
                        OR e.location LIKE ?1 ESCAPE '\\')
                 ORDER BY e.at DESC LIMIT {limite}",
                colunas = EXAM_COLUMNS.replace(", ", ", e."),
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![padrao], |row| {
                let exam = read_exam(row)?;
                // O indice acompanha EXAM_COLUMNS: a materia vem logo depois da
                // ultima coluna da prova. Numero cravado a mao aqui e o que
                // quebrou quando a 0034 acrescentou quatro colunas — por isso
                // ele agora e derivado da propria constante.
                let subject: String = row.get(colunas_de(EXAM_COLUMNS))?;
                Ok((exam, subject))
            })
            .map_err(map_sql_error)?;
        let mut exams = Vec::new();
        for row in rows {
            let (exam, subject) = row.map_err(map_sql_error)?;
            exams.push((exam?, subject));
        }
        drop(statement);

        let mut statement = connection
            .prepare(&format!(
                "SELECT a.{colunas}, s.name FROM academic_assignments a
                 JOIN academic_subjects s ON s.id = a.subject_id
                 WHERE a.lifecycle_state = 'active' AND s.lifecycle_state = 'active'
                   AND (a.title LIKE ?1 ESCAPE '\\' OR a.description LIKE ?1 ESCAPE '\\')
                 ORDER BY a.due_at IS NULL, a.due_at LIMIT {limite}",
                colunas = ASSIGNMENT_COLUMNS.replace(", ", ", a."),
            ))
            .map_err(map_sql_error)?;
        let rows = statement
            .query_map(params![padrao], |row| {
                let assignment = read_assignment(row)?;
                let subject: String = row.get(colunas_de(ASSIGNMENT_COLUMNS))?;
                Ok((assignment, subject))
            })
            .map_err(map_sql_error)?;
        let mut assignments = Vec::new();
        for row in rows {
            let (assignment, subject) = row.map_err(map_sql_error)?;
            assignments.push((assignment?, subject));
        }
        drop(statement);

        // A disciplina vem primeiro: quem procura "Estatica" quer a materia, e
        // as provas dela sao o detalhe que vem depois.
        let mut items: Vec<mos_core::SearchItem> = subjects
            .into_iter()
            .map(|subject| mos_core::SearchItem::Subject { subject })
            .collect();
        items.extend(
            exams
                .into_iter()
                .map(|(exam, subject)| mos_core::SearchItem::Exam { exam, subject }),
        );
        items.extend(assignments.into_iter().map(|(assignment, subject)| {
            mos_core::SearchItem::Assignment {
                assignment,
                subject,
            }
        }));
        Ok(items)
    }

    fn discard_study(&self, id: StudySessionId) -> Result<(), CoreError> {
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        transaction
            .execute(
                "DELETE FROM academic_study_sessions WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(map_sql_error)?;
        // `Delete` de verdade, e nao mudanca de campo: uma sessao descartada nao
        // tem "voltar" — ela nunca deveria ter existido. E a mesma assimetria do
        // objetivo removido do dia, na 0028.
        self.emitir(
            &transaction,
            mos_sync::EntityRef::new(KIND_STUDY, id.as_uuid()),
            mos_sync::OpBody::Delete,
        )?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }

    // =======================================================================
    // A decisao da pessoa
    // =======================================================================
    //
    // Estes quatro metodos escrevem colunas que o sync do Univirtus NUNCA toca
    // (`academic_provider_repository.rs` lista colunas explicitamente, e nenhuma
    // delas e `decision`, `decided_at` ou `planned_at`). E o que faz "ja
    // entreguei" sobreviver a proxima sincronizacao.

    fn set_assignment_decision(
        &self,
        id: AssignmentId,
        decision: Decision,
    ) -> Result<Assignment, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        // `decided_at` volta a NULL ao desfazer: guardar a hora de uma decisao
        // que nao existe mais faria o historico contar um evento que nao houve.
        let decided_at = if decision == Decision::None {
            None
        } else {
            Some(now.clone())
        };
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let mudou = transaction
            .execute(
                "UPDATE academic_assignments
                    SET decision = ?2, decided_at = ?3, updated_at = ?4
                  WHERE id = ?1",
                params![id.to_string(), decision.as_str(), decided_at, now],
            )
            .map_err(map_sql_error)?;
        if mudou == 0 {
            return Err(not_found("Atividade"));
        }
        self.emitir_update(
            &transaction,
            KIND_ASSIGNMENT,
            id.as_uuid(),
            &[
                ("decision", serde_json::json!(decision.as_str())),
                ("decidedAt", serde_json::json!(decided_at)),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_assignment(&connection, id)
    }

    fn set_exam_decision(&self, id: ExamId, decision: Decision) -> Result<Exam, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let decided_at = if decision == Decision::None {
            None
        } else {
            Some(now.clone())
        };
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let mudou = transaction
            .execute(
                "UPDATE academic_exams
                    SET decision = ?2, decided_at = ?3, updated_at = ?4
                  WHERE id = ?1",
                params![id.to_string(), decision.as_str(), decided_at, now],
            )
            .map_err(map_sql_error)?;
        if mudou == 0 {
            return Err(not_found("Avaliacao"));
        }
        self.emitir_update(
            &transaction,
            KIND_EXAM,
            id.as_uuid(),
            &[
                ("decision", serde_json::json!(decision.as_str())),
                ("decidedAt", serde_json::json!(decided_at)),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_exam(&connection, id)
    }

    fn plan_assignment(
        &self,
        id: AssignmentId,
        plano: Option<Plano>,
    ) -> Result<Assignment, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let quando = plano.map(|p| format_time(p.quando)).transpose()?;
        let minutos = plano.map(|p| p.minutos).unwrap_or(0);
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let mudou = transaction
            .execute(
                "UPDATE academic_assignments
                    SET planned_at = ?2, planned_minutes = ?3, updated_at = ?4
                  WHERE id = ?1",
                params![id.to_string(), quando, minutos, now],
            )
            .map_err(map_sql_error)?;
        if mudou == 0 {
            return Err(not_found("Atividade"));
        }
        self.emitir_update(
            &transaction,
            KIND_ASSIGNMENT,
            id.as_uuid(),
            &[
                ("plannedAt", serde_json::json!(quando)),
                ("plannedMinutes", serde_json::json!(minutos)),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_assignment(&connection, id)
    }

    fn plan_exam(&self, id: ExamId, plano: Option<Plano>) -> Result<Exam, CoreError> {
        let now = format_time(OffsetDateTime::now_utc())?;
        let quando = plano.map(|p| format_time(p.quando)).transpose()?;
        let minutos = plano.map(|p| p.minutos).unwrap_or(0);
        let connection = self.escrita()?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        let mudou = transaction
            .execute(
                "UPDATE academic_exams
                    SET planned_at = ?2, planned_minutes = ?3, updated_at = ?4
                  WHERE id = ?1",
                params![id.to_string(), quando, minutos, now],
            )
            .map_err(map_sql_error)?;
        if mudou == 0 {
            return Err(not_found("Avaliacao"));
        }
        self.emitir_update(
            &transaction,
            KIND_EXAM,
            id.as_uuid(),
            &[
                ("plannedAt", serde_json::json!(quando)),
                ("plannedMinutes", serde_json::json!(minutos)),
            ],
        )?;
        transaction.commit().map_err(map_sql_error)?;
        one_exam(&connection, id)
    }
}
