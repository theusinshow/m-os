//! Os comandos do M/Academic.
//!
//! Mesma divisao do `daily.rs` e do `calendar.rs`: a regra vive em
//! `mos_core::academic` (o que e "chegando", como a media pondera, o que conta
//! como atraso) e em `mos_core::AcademicService` (o que acontece ao criar,
//! editar e concluir). Aqui fica so o que o core nao pode ter — **que hora e
//! agora no fuso de quem esta na frente da tela** — e o aviso ao renderer.
//!
//! O instante local vem do `surface.rs`, publicado pelo renderer. Compor o
//! painel em UTC jogaria toda entrega da madrugada para o dia seguinte, e uma
//! prova das 20h de sexta apareceria no sabado.

use mos_core::{
    AcademicDashboard, AcademicToday, Assignment, CoreError, Exam, Resource, Semester, StudySession,
    Subject, Task,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::AppState;

/// Avisa a Home que a faculdade mudou.
///
/// O mesmo evento generico das outras entidades: quem escuta recarrega o que
/// precisa. Um evento por tipo de mudanca obrigaria o renderer a conhecer a
/// taxonomia inteira do Academic para saber o que releer.
fn avisar<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit_to("main", "data-changed", "academic-changed");
}

fn servico<R: Runtime>(app: &AppHandle<R>) -> Result<mos_core::AcademicService, CoreError> {
    Ok(mos_core::AcademicService::new(
        app.state::<AppState>().storage.clone(),
    ))
}

// ===========================================================================
// Painel
// ===========================================================================

#[tauri::command]
pub fn academic_dashboard<R: Runtime>(app: AppHandle<R>) -> Result<AcademicDashboard, CoreError> {
    servico(&app)?.dashboard(crate::surface::now_local(&app))
}

/// O recorte de hoje. E o que o Start My Day e o End My Day consomem.
#[tauri::command]
pub fn academic_today<R: Runtime>(app: AppHandle<R>) -> Result<AcademicToday, CoreError> {
    servico(&app)?.today(crate::surface::now_local(&app))
}

// ===========================================================================
// Semestre
// ===========================================================================

#[tauri::command]
pub fn academic_semesters<R: Runtime>(
    app: AppHandle<R>,
    include_archived: bool,
) -> Result<Vec<Semester>, CoreError> {
    servico(&app)?.semesters(include_archived)
}

#[tauri::command]
pub fn academic_create_semester<R: Runtime>(
    app: AppHandle<R>,
    name: String,
    institution: String,
    starts_on: String,
    ends_on: String,
) -> Result<Semester, CoreError> {
    let semestre = servico(&app)?.create_semester(&name, &institution, &starts_on, &ends_on)?;
    avisar(&app);
    Ok(semestre)
}

#[tauri::command]
pub fn academic_update_semester<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    name: String,
    institution: String,
    starts_on: String,
    ends_on: String,
) -> Result<Semester, CoreError> {
    let semestre = servico(&app)?.update_semester(&id, &name, &institution, &starts_on, &ends_on)?;
    avisar(&app);
    Ok(semestre)
}

#[tauri::command]
pub fn academic_archive_semester<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    archived: bool,
) -> Result<Semester, CoreError> {
    let semestre = servico(&app)?.set_semester_archived(&id, archived)?;
    avisar(&app);
    Ok(semestre)
}

// ===========================================================================
// Disciplina
// ===========================================================================

#[tauri::command]
pub fn academic_subjects<R: Runtime>(
    app: AppHandle<R>,
    include_archived: bool,
) -> Result<Vec<Subject>, CoreError> {
    servico(&app)?.subjects(include_archived)
}

#[tauri::command]
pub fn academic_create_subject<R: Runtime>(
    app: AppHandle<R>,
    semester_id: String,
    name: String,
    code: String,
    teacher: String,
    accent: String,
    notes: String,
) -> Result<Subject, CoreError> {
    let subject =
        servico(&app)?.create_subject(&semester_id, &name, &code, &teacher, &accent, &notes)?;
    avisar(&app);
    Ok(subject)
}

#[tauri::command]
pub fn academic_update_subject<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    name: String,
    code: String,
    teacher: String,
    accent: String,
    notes: String,
) -> Result<Subject, CoreError> {
    let subject = servico(&app)?.update_subject(&id, &name, &code, &teacher, &accent, &notes)?;
    avisar(&app);
    Ok(subject)
}

#[tauri::command]
pub fn academic_archive_subject<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    archived: bool,
) -> Result<Subject, CoreError> {
    let subject = servico(&app)?.set_subject_archived(&id, archived)?;
    avisar(&app);
    Ok(subject)
}

// ===========================================================================
// Atividade
// ===========================================================================

#[tauri::command]
pub fn academic_assignments<R: Runtime>(
    app: AppHandle<R>,
    include_archived: bool,
) -> Result<Vec<Assignment>, CoreError> {
    servico(&app)?.assignments(include_archived)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn academic_create_assignment<R: Runtime>(
    app: AppHandle<R>,
    subject_id: String,
    title: String,
    description: String,
    due_at: Option<String>,
    priority: String,
    weight: f64,
    score: Option<f64>,
    max_score: Option<f64>,
) -> Result<Assignment, CoreError> {
    let assignment = servico(&app)?.create_assignment(
        &subject_id,
        &title,
        &description,
        due_at.as_deref(),
        &priority,
        weight,
        score,
        max_score,
    )?;
    avisar(&app);
    Ok(assignment)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn academic_update_assignment<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    title: String,
    description: String,
    due_at: Option<String>,
    priority: String,
    weight: f64,
    score: Option<f64>,
    max_score: Option<f64>,
    status: String,
) -> Result<Assignment, CoreError> {
    let assignment = servico(&app)?.update_assignment(
        &id,
        &title,
        &description,
        due_at.as_deref(),
        &priority,
        weight,
        score,
        max_score,
        &status,
    )?;
    avisar(&app);
    Ok(assignment)
}

#[tauri::command]
pub fn academic_set_assignment_status<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    status: String,
) -> Result<Assignment, CoreError> {
    let assignment = servico(&app)?.set_assignment_status(&id, &status)?;
    avisar(&app);
    Ok(assignment)
}

#[tauri::command]
pub fn academic_archive_assignment<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    archived: bool,
) -> Result<Assignment, CoreError> {
    let assignment = servico(&app)?.set_assignment_archived(&id, archived)?;
    avisar(&app);
    Ok(assignment)
}

/// Cria a Task do M/OS que executa esta atividade.
#[tauri::command]
pub fn academic_create_task<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<Task, CoreError> {
    let task = servico(&app)?.create_task_for_assignment(&id)?;
    avisar(&app);
    Ok(task)
}

#[tauri::command]
pub fn academic_unlink_task<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<Assignment, CoreError> {
    let assignment = servico(&app)?.unlink_assignment_task(&id)?;
    avisar(&app);
    Ok(assignment)
}

// ===========================================================================
// Avaliacao
// ===========================================================================

#[tauri::command]
pub fn academic_exams<R: Runtime>(
    app: AppHandle<R>,
    include_archived: bool,
) -> Result<Vec<Exam>, CoreError> {
    servico(&app)?.exams(include_archived)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn academic_create_exam<R: Runtime>(
    app: AppHandle<R>,
    subject_id: String,
    name: String,
    at: String,
    location: String,
    topics: String,
    weight: f64,
    score: Option<f64>,
    max_score: Option<f64>,
) -> Result<Exam, CoreError> {
    let exam = servico(&app)?.create_exam(
        &subject_id,
        &name,
        &at,
        &location,
        &topics,
        weight,
        score,
        max_score,
    )?;
    avisar(&app);
    Ok(exam)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn academic_update_exam<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    name: String,
    at: String,
    location: String,
    topics: String,
    weight: f64,
    score: Option<f64>,
    max_score: Option<f64>,
    status: String,
) -> Result<Exam, CoreError> {
    let exam = servico(&app)?.update_exam(
        &id, &name, &at, &location, &topics, weight, score, max_score, &status,
    )?;
    avisar(&app);
    Ok(exam)
}

#[tauri::command]
pub fn academic_archive_exam<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    archived: bool,
) -> Result<Exam, CoreError> {
    let exam = servico(&app)?.set_exam_archived(&id, archived)?;
    avisar(&app);
    Ok(exam)
}

// ===========================================================================
// Materiais
// ===========================================================================

#[tauri::command]
pub fn academic_materials<R: Runtime>(
    app: AppHandle<R>,
    subject_id: String,
) -> Result<Vec<Resource>, CoreError> {
    servico(&app)?.subject_resources(&subject_id)
}

#[tauri::command]
pub fn academic_link_material<R: Runtime>(
    app: AppHandle<R>,
    subject_id: String,
    resource_id: String,
    linked: bool,
) -> Result<(), CoreError> {
    servico(&app)?.link_material(&subject_id, &resource_id, linked)?;
    avisar(&app);
    Ok(())
}

// ===========================================================================
// Estudo
// ===========================================================================

#[tauri::command]
pub fn academic_study_sessions<R: Runtime>(
    app: AppHandle<R>,
    limit: Option<usize>,
) -> Result<Vec<StudySession>, CoreError> {
    servico(&app)?.study_sessions(limit.unwrap_or(50))
}

#[tauri::command]
pub fn academic_start_study<R: Runtime>(
    app: AppHandle<R>,
    subject_id: String,
    topic: String,
) -> Result<StudySession, CoreError> {
    let sessao = servico(&app)?.start_study(&subject_id, &topic)?;
    avisar(&app);
    Ok(sessao)
}

#[tauri::command]
pub fn academic_finish_study<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    seconds: i64,
    notes: String,
) -> Result<StudySession, CoreError> {
    let sessao = servico(&app)?.finish_study(&id, seconds, &notes)?;
    avisar(&app);
    Ok(sessao)
}

#[tauri::command]
pub fn academic_discard_study<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), CoreError> {
    servico(&app)?.discard_study(&id)?;
    avisar(&app);
    Ok(())
}

// ===========================================================================
// A decisao da pessoa
// ===========================================================================
//
// Separados dos comandos de `status` de proposito. Aqueles descrevem o FATO
// ACADEMICO e o sync do Univirtus os escreve; estes sao a decisao de quem
// estuda, e nenhum provedor externo os toca. Ver `mos_core::academic_decision`.

fn repositorio<R: Runtime>(app: &AppHandle<R>) -> std::sync::Arc<mos_storage_sqlite::SqliteStorage> {
    app.state::<AppState>().storage.clone()
}

fn plano_de(planned_at: Option<String>, minutes: i64) -> Result<Option<mos_core::Plano>, CoreError> {
    let Some(bruto) = planned_at else {
        return Ok(None);
    };
    if bruto.trim().is_empty() {
        return Ok(None);
    }
    let quando = time::OffsetDateTime::parse(&bruto, &time::format_description::well_known::Rfc3339)
        .map_err(|_| {
            CoreError::new(
                mos_core::ErrorCode::InvalidInput,
                "Data do plano invalida.",
                false,
            )
        })?;
    Ok(Some(mos_core::Plano::novo(quando, minutes)?))
}

#[tauri::command]
pub fn academic_set_assignment_decision<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    decision: String,
) -> Result<Assignment, CoreError> {
    let decisao = mos_core::Decision::parse(&decision)?;
    let atividade = mos_core::AcademicRepository::set_assignment_decision(
        repositorio(&app).as_ref(),
        mos_core::AssignmentId::parse(&id)?,
        decisao,
    )?;
    avisar(&app);
    Ok(atividade)
}

#[tauri::command]
pub fn academic_set_exam_decision<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    decision: String,
) -> Result<Exam, CoreError> {
    let decisao = mos_core::Decision::parse(&decision)?;
    let prova = mos_core::AcademicRepository::set_exam_decision(
        repositorio(&app).as_ref(),
        mos_core::ExamId::parse(&id)?,
        decisao,
    )?;
    avisar(&app);
    Ok(prova)
}

/// Quando pretendo fazer, e por quanto tempo. `planned_at` vazio desfaz.
#[tauri::command]
pub fn academic_plan_assignment<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    planned_at: Option<String>,
    minutes: i64,
) -> Result<Assignment, CoreError> {
    let plano = plano_de(planned_at, minutes)?;
    let atividade = mos_core::AcademicRepository::plan_assignment(
        repositorio(&app).as_ref(),
        mos_core::AssignmentId::parse(&id)?,
        plano,
    )?;
    avisar(&app);
    Ok(atividade)
}

#[tauri::command]
pub fn academic_plan_exam<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    planned_at: Option<String>,
    minutes: i64,
) -> Result<Exam, CoreError> {
    let plano = plano_de(planned_at, minutes)?;
    let prova = mos_core::AcademicRepository::plan_exam(
        repositorio(&app).as_ref(),
        mos_core::ExamId::parse(&id)?,
        plano,
    )?;
    avisar(&app);
    Ok(prova)
}
