//! Comandos do rastreio de tempo (ADR-032).
//!
//! A importacao do CronoCAD e um caminho de mao unica que roda uma vez. Ela
//! aparece na interface e nao num script porque quem precisa conferir se as
//! horas chegaram inteiras e o usuario, na tela dele — e porque um script exige
//! que ele saiba onde o banco mora.

use std::path::PathBuf;

use mos_core::{CoreError, ProjectId, TimeEntry, Totals};
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

/// As sessoes, em tempo REAL — sem arredondar e sem descontar inatividade.
#[tauri::command]
pub fn tracking_entries<R: Runtime>(
    app: AppHandle<R>,
    project_id: Option<String>,
) -> Result<Vec<TimeEntry>, CoreError> {
    let project = project_id.as_deref().map(ProjectId::parse).transpose()?;
    app.state::<AppState>().tracking.entries(project)
}
