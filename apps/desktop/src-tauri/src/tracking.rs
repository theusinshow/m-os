//! Comandos do rastreio de tempo (ADR-032).
//!
//! A importacao do CronoCAD e um caminho de mao unica que roda uma vez. Ela
//! aparece na interface e nao num script porque quem precisa conferir se as
//! horas chegaram inteiras e o usuario, na tela dele — e porque um script exige
//! que ele saiba onde o banco mora.

use std::path::PathBuf;

use mos_core::{ActiveTimer, ActivityType, CoreError, ProjectId, StartTimer, TimeEntry, Totals};
use mos_storage_sqlite::ImportReport;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::AppState;

/// Onde o CronoCAD guarda o banco, na instalacao padrao.
///
/// Sugerido e nao imposto: o usuario escolhe o arquivo no dialogo. Serve para
/// que ele nao precise saber que `com.cronocad.app` existe.
#[tauri::command]
pub fn tracking_default_cronocad_path() -> Option<String> {
    let base = std::env::var("APPDATA").ok()?;
    let path = PathBuf::from(base)
        .join("com.cronocad.app")
        .join("cronocad.sqlite");
    path.exists().then(|| path.display().to_string())
}

/// Quando o CronoCAD foi importado, se foi.
///
/// A tela pergunta ao BANCO em vez de lembrar da sessao: fechar e reabrir o app
/// nao deveria reabilitar um botao que nao pode mais ser clicado.
#[tauri::command]
pub fn tracking_cronocad_imported_at<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<String>, CoreError> {
    app.state::<AppState>().storage.cronocad_imported_at()
}

/// Importa o banco do CronoCAD. Roda uma vez.
#[tauri::command]
pub async fn tracking_import_cronocad<R: Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Result<ImportReport, CoreError> {
    let report = app
        .state::<AppState>()
        .storage
        .import_cronocad(std::path::Path::new(&path))?;
    // Projects e Tasks nasceram: a Home, o Kanban e a lista precisam refletir.
    let _ = app.emit("data-changed", "cronocad-import");
    Ok(report)
}

/// Totais por Project, com o arredondamento configurado ja aplicado.
#[tauri::command]
pub fn tracking_totals<R: Runtime>(
    app: AppHandle<R>,
) -> Result<std::collections::HashMap<String, Totals>, CoreError> {
    app.state::<AppState>().tracking.totals_by_project()
}

/// O cronometro em curso, se houver.
///
/// A tela pergunta e recebe `accumulated_seconds` mais `last_resumed_at`, e nao
/// um numero de segundos ja pronto: assim ela pode desenhar o relogio correndo
/// sozinha, sem que o backend precise emitir um evento por segundo.
#[tauri::command]
pub fn timer_current<R: Runtime>(app: AppHandle<R>) -> Result<Option<ActiveTimer>, CoreError> {
    app.state::<AppState>().tracking.active_timer()
}

#[tauri::command]
pub fn timer_start<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    description: String,
    activity_type: String,
) -> Result<ActiveTimer, CoreError> {
    let timer = app.state::<AppState>().tracking.start_timer(StartTimer {
        project_id: ProjectId::parse(&project_id)?,
        description,
        activity_type: ActivityType::parse(&activity_type)?,
    })?;
    let _ = app.emit("timer-changed", "started");
    Ok(timer)
}

#[tauri::command]
pub fn timer_set_running<R: Runtime>(
    app: AppHandle<R>,
    running: bool,
) -> Result<ActiveTimer, CoreError> {
    let timer = app
        .state::<AppState>()
        .tracking
        .set_timer_running(running)?;
    let _ = app.emit("timer-changed", if running { "resumed" } else { "paused" });
    Ok(timer)
}

/// Encerra e devolve a sessao gravada.
#[tauri::command]
pub fn timer_stop<R: Runtime>(app: AppHandle<R>) -> Result<TimeEntry, CoreError> {
    let entry = app.state::<AppState>().tracking.stop_timer()?;
    let _ = app.emit("timer-changed", "stopped");
    // A sessao nasceu: quem mostra horas por Project precisa reler.
    let _ = app.emit("data-changed", "timer");
    Ok(entry)
}

/// As sessoes, em tempo REAL — sem arredondar e sem descontar inatividade.
#[tauri::command]
pub fn tracking_entries<R: Runtime>(
    app: AppHandle<R>,
    project_id: Option<String>,
) -> Result<Vec<TimeEntry>, CoreError> {
    let project = project_id.as_deref().map(ProjectId::parse).transpose()?;
    app.state::<AppState>().tracking.entries(project)
}
