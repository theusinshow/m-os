//! Comandos Tauri expostos ao frontend.
//!
//! Principio de seguranca (secao 19): comandos sao especificos e tipados;
//! nenhum SQL arbitrario e exposto. Cada comando valida suas entradas e retorna
//! `AppError` (string legivel) em caso de falha. O acesso ao banco usa o pool
//! compartilhado do plugin (`database::pool`).

use tauri::{AppHandle, Manager, State};
use tauri_plugin_sql::DbInstances;

use crate::database;
use crate::error::AppError;
use crate::models::{
    validate_status, ActiveTimer, ActivityEvent, Client, ClientInput, EntryUpdateInput,
    ManualEntryInput, MonitoredApp, MonitoredAppInput, Project, ProjectInput, ProjectTodo,
    ProjectTotal, Settings, StartTimerInput, TimeEntry,
};
use crate::monitoring::MonitorShared;
use crate::repository::{
    activity_events, clients, monitored_apps, notes, projects, settings, time_entries, timer,
};
use crate::state::AppInfo;
use crate::timer_service;

/// Retorna nome e versao do aplicativo (contrato de verificacao da ponte).
#[tauri::command]
pub fn app_info() -> Result<AppInfo, AppError> {
    Ok(AppInfo::default())
}

// ---------------------------------------------------------------------------
// Clients
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_clients(
    db: State<'_, DbInstances>,
    include_archived: Option<bool>,
) -> Result<Vec<Client>, AppError> {
    let pool = database::pool(&db).await?;
    clients::list(&pool, include_archived.unwrap_or(false)).await
}

#[tauri::command]
pub async fn get_client(db: State<'_, DbInstances>, id: String) -> Result<Client, AppError> {
    let pool = database::pool(&db).await?;
    clients::get(&pool, &id).await
}

#[tauri::command]
pub async fn create_client(
    db: State<'_, DbInstances>,
    input: ClientInput,
) -> Result<Client, AppError> {
    let valid = input.validate()?;
    let pool = database::pool(&db).await?;
    clients::create(&pool, valid).await
}

#[tauri::command]
pub async fn update_client(
    db: State<'_, DbInstances>,
    id: String,
    input: ClientInput,
) -> Result<Client, AppError> {
    let valid = input.validate()?;
    let pool = database::pool(&db).await?;
    clients::update(&pool, &id, valid).await
}

#[tauri::command]
pub async fn archive_client(db: State<'_, DbInstances>, id: String) -> Result<Client, AppError> {
    let pool = database::pool(&db).await?;
    clients::archive(&pool, &id).await
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_projects(
    db: State<'_, DbInstances>,
    include_archived: Option<bool>,
) -> Result<Vec<Project>, AppError> {
    let pool = database::pool(&db).await?;
    projects::list(&pool, include_archived.unwrap_or(false)).await
}

#[tauri::command]
pub async fn get_project(db: State<'_, DbInstances>, id: String) -> Result<Project, AppError> {
    let pool = database::pool(&db).await?;
    projects::get(&pool, &id).await
}

#[tauri::command]
pub async fn create_project(
    db: State<'_, DbInstances>,
    input: ProjectInput,
) -> Result<Project, AppError> {
    let valid = input.validate()?;
    let pool = database::pool(&db).await?;
    projects::create(&pool, valid).await
}

#[tauri::command]
pub async fn update_project(
    db: State<'_, DbInstances>,
    id: String,
    input: ProjectInput,
) -> Result<Project, AppError> {
    let valid = input.validate()?;
    let pool = database::pool(&db).await?;
    projects::update(&pool, &id, valid).await
}

#[tauri::command]
pub async fn list_project_totals(
    db: State<'_, DbInstances>,
) -> Result<Vec<ProjectTotal>, AppError> {
    let pool = database::pool(&db).await?;
    time_entries::totals_by_project(&pool).await
}

#[tauri::command]
pub async fn set_project_status(
    db: State<'_, DbInstances>,
    id: String,
    status: String,
) -> Result<Project, AppError> {
    validate_status(&status)?;
    let pool = database::pool(&db).await?;
    projects::set_status(&pool, &id, &status).await
}

// ---------------------------------------------------------------------------
// Anotacoes e pendencias por projeto
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn update_project_notes(
    db: State<'_, DbInstances>,
    project_id: String,
    notes: Option<String>,
) -> Result<Project, AppError> {
    let pool = database::pool(&db).await?;
    notes::set_notes(&pool, &project_id, notes).await
}

#[tauri::command]
pub async fn list_todos(db: State<'_, DbInstances>) -> Result<Vec<ProjectTodo>, AppError> {
    let pool = database::pool(&db).await?;
    notes::list_todos(&pool).await
}

#[tauri::command]
pub async fn create_todo(
    db: State<'_, DbInstances>,
    project_id: String,
    text: String,
) -> Result<ProjectTodo, AppError> {
    let pool = database::pool(&db).await?;
    notes::create_todo(&pool, &project_id, &text).await
}

#[tauri::command]
pub async fn set_todo_done(
    db: State<'_, DbInstances>,
    id: String,
    done: bool,
) -> Result<ProjectTodo, AppError> {
    let pool = database::pool(&db).await?;
    notes::set_todo_done(&pool, &id, done).await
}

#[tauri::command]
pub async fn update_todo_text(
    db: State<'_, DbInstances>,
    id: String,
    text: String,
) -> Result<ProjectTodo, AppError> {
    let pool = database::pool(&db).await?;
    notes::update_todo_text(&pool, &id, &text).await
}

#[tauri::command]
pub async fn delete_todo(db: State<'_, DbInstances>, id: String) -> Result<(), AppError> {
    let pool = database::pool(&db).await?;
    notes::delete_todo(&pool, &id).await
}

// ---------------------------------------------------------------------------
// Cronometro (motor — secao 9)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_active_timer(app: AppHandle) -> Result<Option<ActiveTimer>, AppError> {
    timer_service::get_active(&app).await
}

#[tauri::command]
pub async fn start_timer(app: AppHandle, input: StartTimerInput) -> Result<ActiveTimer, AppError> {
    let valid = input.validate()?;
    timer_service::start(&app, valid).await
}

#[tauri::command]
pub async fn pause_timer(app: AppHandle) -> Result<ActiveTimer, AppError> {
    timer_service::pause(&app).await
}

#[tauri::command]
pub async fn resume_timer(app: AppHandle) -> Result<ActiveTimer, AppError> {
    timer_service::resume(&app).await
}

#[tauri::command]
pub async fn stop_timer(app: AppHandle) -> Result<TimeEntry, AppError> {
    timer_service::stop(&app).await
}

#[tauri::command]
pub async fn discard_timer(app: AppHandle) -> Result<(), AppError> {
    timer_service::discard(&app).await
}

/// Desconta tempo inativo do cronometro ativo (secao 11). Nao altera a duracao
/// bruta; apenas o tempo inativo que sera aplicado a sessao ao encerrar.
#[tauri::command]
pub async fn discount_idle(
    db: State<'_, DbInstances>,
    seconds: i64,
) -> Result<ActiveTimer, AppError> {
    let pool = database::pool(&db).await?;
    timer::add_idle(&pool, seconds).await
}

// ---------------------------------------------------------------------------
// Sessoes (time_entries)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_time_entries(
    db: State<'_, DbInstances>,
    limit: Option<i64>,
    include_deleted: Option<bool>,
) -> Result<Vec<TimeEntry>, AppError> {
    let pool = database::pool(&db).await?;
    time_entries::list_recent(
        &pool,
        limit.unwrap_or(200),
        include_deleted.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub async fn create_time_entry(
    db: State<'_, DbInstances>,
    input: ManualEntryInput,
) -> Result<TimeEntry, AppError> {
    let valid = input.validate()?;
    let pool = database::pool(&db).await?;
    time_entries::create_manual(&pool, valid).await
}

#[tauri::command]
pub async fn update_time_entry(
    db: State<'_, DbInstances>,
    id: String,
    input: EntryUpdateInput,
) -> Result<TimeEntry, AppError> {
    let valid = input.validate()?;
    let pool = database::pool(&db).await?;
    time_entries::update(&pool, &id, valid).await
}

#[tauri::command]
pub async fn delete_time_entry(db: State<'_, DbInstances>, id: String) -> Result<(), AppError> {
    let pool = database::pool(&db).await?;
    time_entries::soft_delete(&pool, &id).await
}

#[tauri::command]
pub async fn restore_time_entry(
    db: State<'_, DbInstances>,
    id: String,
) -> Result<TimeEntry, AppError> {
    let pool = database::pool(&db).await?;
    time_entries::restore(&pool, &id).await
}

// ---------------------------------------------------------------------------
// Reconstrucao do dia (secao 14)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_activity_events(
    db: State<'_, DbInstances>,
    from: String,
    to: String,
) -> Result<Vec<ActivityEvent>, AppError> {
    let pool = database::pool(&db).await?;
    activity_events::list_between(&pool, &from, &to).await
}

/// Abre um dialogo nativo para salvar um arquivo de texto (ex.: CSV) e grava o
/// conteudo. Retorna `true` se salvo, `false` se o usuario cancelou.
#[tauri::command]
pub async fn save_text_file(
    app: AppHandle,
    content: String,
    suggested_name: String,
) -> Result<bool, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let path = app
        .dialog()
        .file()
        .set_file_name(&suggested_name)
        .add_filter("CSV", &["csv"])
        .blocking_save_file();

    match path {
        Some(file_path) => {
            let path_buf = file_path
                .into_path()
                .map_err(|e| AppError::Io(e.to_string()))?;
            std::fs::write(&path_buf, content).map_err(|e| AppError::Io(e.to_string()))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Gera o relatorio em PDF e o salva num caminho escolhido pelo usuario.
/// Retorna `true` se salvo, `false` se cancelado.
#[tauri::command]
pub async fn export_report_pdf(
    app: AppHandle,
    report: crate::pdf::ReportPdfData,
    suggested_name: String,
) -> Result<bool, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let path = app
        .dialog()
        .file()
        .set_file_name(&suggested_name)
        .add_filter("PDF", &["pdf"])
        .blocking_save_file();

    match path {
        Some(file_path) => {
            let path_buf = file_path
                .into_path()
                .map_err(|e| AppError::Io(e.to_string()))?;
            let bytes = crate::pdf::build_report(&report).map_err(AppError::Io)?;
            std::fs::write(&path_buf, bytes).map_err(|e| AppError::Io(e.to_string()))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Gera a fatura por cliente em PDF e a salva num caminho escolhido.
#[tauri::command]
pub async fn export_invoice_pdf(
    app: AppHandle,
    invoice: crate::pdf::InvoiceData,
    suggested_name: String,
) -> Result<bool, AppError> {
    use tauri_plugin_dialog::DialogExt;

    let path = app
        .dialog()
        .file()
        .set_file_name(&suggested_name)
        .add_filter("PDF", &["pdf"])
        .blocking_save_file();

    match path {
        Some(file_path) => {
            let path_buf = file_path
                .into_path()
                .map_err(|e| AppError::Io(e.to_string()))?;
            let bytes = crate::pdf::build_invoice(&invoice).map_err(AppError::Io)?;
            std::fs::write(&path_buf, bytes).map_err(|e| AppError::Io(e.to_string()))?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Lembrete pendente para o widget flutuante recuperar ao carregar (secao 10).
#[tauri::command]
pub fn get_pending_reminder(
    app: AppHandle,
) -> Result<Option<crate::reminder::ReminderInfo>, AppError> {
    Ok(crate::reminder::pending(&app))
}

/// Encerra o aplicativo (usado pela confirmacao de saida com cronometro ativo).
#[tauri::command]
pub fn quit_app(app: AppHandle) -> Result<(), AppError> {
    app.exit(0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Configuracoes
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_settings(db: State<'_, DbInstances>) -> Result<Settings, AppError> {
    let pool = database::pool(&db).await?;
    settings::get(&pool).await
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    db: State<'_, DbInstances>,
    settings: Settings,
) -> Result<Settings, AppError> {
    let valid = settings.validated()?;
    let pool = database::pool(&db).await?;
    let saved = self::settings::update(&pool, valid).await?;
    // Reflete "Iniciar com o Windows" no autostart do SO (secao 13).
    crate::autostart::sync(&app, saved.start_with_windows);
    Ok(saved)
}

// ---------------------------------------------------------------------------
// Programas monitorados (secao 10)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_monitored_apps(
    db: State<'_, DbInstances>,
) -> Result<Vec<MonitoredApp>, AppError> {
    let pool = database::pool(&db).await?;
    monitored_apps::list(&pool).await
}

#[tauri::command]
pub async fn create_monitored_app(
    db: State<'_, DbInstances>,
    input: MonitoredAppInput,
) -> Result<MonitoredApp, AppError> {
    let valid = input.validate()?;
    let pool = database::pool(&db).await?;
    monitored_apps::create(&pool, valid).await
}

#[tauri::command]
pub async fn update_monitored_app(
    db: State<'_, DbInstances>,
    id: String,
    input: MonitoredAppInput,
) -> Result<MonitoredApp, AppError> {
    let valid = input.validate()?;
    let pool = database::pool(&db).await?;
    monitored_apps::update(&pool, &id, valid).await
}

#[tauri::command]
pub async fn delete_monitored_app(db: State<'_, DbInstances>, id: String) -> Result<(), AppError> {
    let pool = database::pool(&db).await?;
    monitored_apps::delete(&pool, &id).await
}

/// Suprime o lembrete de abertura de um processo pelo resto do dia (secao 10).
#[tauri::command]
pub fn suppress_app_reminder_today(app: AppHandle, process_name: String) -> Result<(), AppError> {
    app.state::<MonitorShared>().suppress_today(&process_name);
    Ok(())
}
