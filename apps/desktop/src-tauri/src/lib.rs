use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use mos_core::{
    BackupInspection, BackupReceipt, Capture, CaptureService, CoreError, CreateCaptureInput,
    CreateProjectInput, CreateTaskInput, DataService, Project, SearchItem, Task, TaskState,
    UpdateProjectInput, UpdateTaskInput, WorkService,
};
use mos_storage_sqlite::{SqliteStorage, StorageHealth};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const DEFAULT_CAPTURE_SHORTCUT: &str = "Ctrl+Shift+Space";

struct AppState {
    captures: CaptureService,
    work: WorkService,
    data: DataService,
    storage: Arc<SqliteStorage>,
    shortcut_status: Mutex<String>,
    active_shortcut: Mutex<Option<String>>,
    snapshot_status: Arc<Mutex<String>>,
    settings_path: PathBuf,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserSettings {
    capture_shortcut: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    inbox_count: usize,
    project_count: usize,
    task_count: usize,
    shortcut: String,
    snapshot: String,
    storage: StorageHealth,
}

#[tauri::command]
fn create_capture(
    input: CreateCaptureInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.create(input)?;
    let _ = app.emit_to("main", "capture-changed", capture.id.to_string());
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(capture)
}

#[tauri::command]
fn get_capture(id: &str, state: tauri::State<'_, AppState>) -> Result<Capture, CoreError> {
    state.captures.get(id)
}

#[tauri::command]
fn list_recent(state: tauri::State<'_, AppState>) -> Result<Vec<Capture>, CoreError> {
    state.captures.recent(8)
}

#[tauri::command]
fn list_inbox(state: tauri::State<'_, AppState>) -> Result<Vec<Capture>, CoreError> {
    state.captures.inbox(200)
}

#[tauri::command]
fn list_archived(state: tauri::State<'_, AppState>) -> Result<Vec<Capture>, CoreError> {
    state.captures.archived(200)
}

#[tauri::command]
fn list_trashed(state: tauri::State<'_, AppState>) -> Result<Vec<Capture>, CoreError> {
    state.captures.trashed(200)
}

#[tauri::command]
fn search_captures(
    query: &str,
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Capture>, CoreError> {
    state.captures.search(query, include_archived, 50)
}

#[tauri::command]
fn search_all(
    query: &str,
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchItem>, CoreError> {
    state.work.search(query, include_archived)
}

#[tauri::command]
fn mark_capture_processed(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.mark_processed(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn move_capture_to_inbox(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.move_to_inbox(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn archive_capture(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.archive(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn trash_capture(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.trash(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn restore_capture(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.restore(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn rebuild_search(state: tauri::State<'_, AppState>) -> Result<usize, CoreError> {
    state.work.rebuild_search()
}

#[tauri::command]
fn create_project(
    input: CreateProjectInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Project, CoreError> {
    let project = state.work.create_project(input)?;
    notify_data_changed(&app, "project-created");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(project)
}

#[tauri::command]
fn update_project(
    input: UpdateProjectInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Project, CoreError> {
    let project = state.work.update_project(input)?;
    notify_data_changed(&app, "project-updated");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(project)
}

#[tauri::command]
fn get_project(id: &str, state: tauri::State<'_, AppState>) -> Result<Project, CoreError> {
    state.work.project(id)
}

#[tauri::command]
fn list_projects(
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Project>, CoreError> {
    state.work.projects(include_archived)
}

#[tauri::command]
fn set_project_archived(
    id: &str,
    archived: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Project, CoreError> {
    let project = state.work.set_project_archived(id, archived)?;
    notify_data_changed(&app, "project-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(project)
}

#[tauri::command]
fn create_task(
    input: CreateTaskInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Task, CoreError> {
    let task = state.work.create_task(input)?;
    notify_data_changed(&app, "task-created");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(task)
}

#[tauri::command]
fn update_task(
    input: UpdateTaskInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Task, CoreError> {
    let task = state.work.update_task(input)?;
    notify_data_changed(&app, "task-updated");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(task)
}

#[tauri::command]
fn get_task(id: &str, state: tauri::State<'_, AppState>) -> Result<Task, CoreError> {
    state.work.task(id)
}

#[tauri::command]
fn list_tasks(
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Task>, CoreError> {
    state.work.tasks(include_archived)
}

#[tauri::command]
fn set_task_state(
    id: &str,
    task_state: TaskState,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Task, CoreError> {
    let task = state.work.set_task_state(id, task_state)?;
    notify_data_changed(&app, "task-state");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(task)
}

#[tauri::command]
fn set_task_archived(
    id: &str,
    archived: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Task, CoreError> {
    let task = state.work.set_task_archived(id, archived)?;
    notify_data_changed(&app, "task-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(task)
}

#[tauri::command]
fn create_backup(
    path: &str,
    state: tauri::State<'_, AppState>,
) -> Result<BackupReceipt, CoreError> {
    state.data.create_backup(PathBuf::from(path).as_path())
}

#[tauri::command]
fn inspect_backup(
    path: &str,
    state: tauri::State<'_, AppState>,
) -> Result<BackupInspection, CoreError> {
    state.data.inspect_backup(PathBuf::from(path).as_path())
}

#[tauri::command]
fn restore_backup(
    path: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<BackupReceipt, CoreError> {
    let receipt = state.data.restore_backup(PathBuf::from(path).as_path())?;
    let _ = app.emit_to("main", "dataset-restored", ());
    Ok(receipt)
}

#[tauri::command]
fn export_json(path: &str, state: tauri::State<'_, AppState>) -> Result<BackupReceipt, CoreError> {
    state.data.export_json(PathBuf::from(path).as_path())
}

#[tauri::command]
fn get_app_status(state: tauri::State<'_, AppState>) -> Result<AppStatus, CoreError> {
    Ok(AppStatus {
        inbox_count: state.captures.inbox(200)?.len(),
        project_count: state.work.projects(false)?.len(),
        task_count: state.work.tasks(false)?.len(),
        shortcut: state
            .shortcut_status
            .lock()
            .map_err(|error| lock_error(error.to_string()))?
            .clone(),
        snapshot: state
            .snapshot_status
            .lock()
            .map_err(|error| lock_error(error.to_string()))?
            .clone(),
        storage: state.storage.health()?,
    })
}

#[tauri::command]
fn set_capture_shortcut(
    shortcut: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, CoreError> {
    let requested = shortcut.trim();
    if requested.is_empty() {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Informe um atalho.",
            false,
        ));
    }

    let mut active = state
        .active_shortcut
        .lock()
        .map_err(|error| lock_error(error.to_string()))?;
    if active.as_deref() == Some(requested) {
        return Ok(format!("Registrado: {requested}"));
    }
    let previous = active.take();
    if let Some(previous) = &previous {
        app.global_shortcut()
            .unregister(previous.as_str())
            .map_err(shortcut_error)?;
    }

    let result = match app.global_shortcut().register(requested) {
        Ok(()) => match persist_shortcut(&state.settings_path, requested) {
            Ok(()) => {
                *active = Some(requested.into());
                Ok(format!("Registrado: {requested}"))
            }
            Err(error) => {
                let _ = app.global_shortcut().unregister(requested);
                if let Some(previous) = &previous {
                    if app.global_shortcut().register(previous.as_str()).is_ok() {
                        *active = Some(previous.clone());
                    }
                }
                Err(error)
            }
        },
        Err(error) => {
            let mut message = format!("Nao foi possivel registrar {requested}: {error}");
            if let Some(previous) = previous {
                if app.global_shortcut().register(previous.as_str()).is_ok() {
                    *active = Some(previous.clone());
                    message.push_str(&format!(". {previous} continua ativo."));
                }
            }
            Err(CoreError::new(
                mos_core::ErrorCode::InvalidInput,
                message,
                true,
            ))
        }
    };
    let status = match &result {
        Ok(message) => message.clone(),
        Err(error) => error.message.clone(),
    };
    *state
        .shortcut_status
        .lock()
        .map_err(|error| lock_error(error.to_string()))? = status;
    result
}

#[tauri::command]
fn show_quick_capture(app: AppHandle) {
    reveal_window(&app, "quick-capture");
}

#[tauri::command]
fn hide_quick_capture(app: AppHandle) {
    if let Some(window) = app.get_webview_window("quick-capture") {
        let _ = window.hide();
    }
}

fn notify_capture_changed(app: &AppHandle, id: &str) {
    let _ = app.emit_to("main", "capture-changed", id);
}

fn notify_data_changed(app: &AppHandle, reason: &str) {
    let _ = app.emit_to("main", "data-changed", reason);
}

fn schedule_snapshot(data: &DataService, snapshot_status: &Arc<Mutex<String>>, app: &AppHandle) {
    let data = data.clone();
    let snapshot_status = snapshot_status.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let message = match data.ensure_daily_snapshot() {
            Ok(Some(_)) => "Snapshot diario criado.".to_owned(),
            Ok(None) => "Snapshot diario ja existe.".to_owned(),
            Err(error) => format!("Falha no snapshot diario: {}", error.message),
        };
        if let Ok(mut status) = snapshot_status.lock() {
            *status = message.clone();
        }
        let _ = app.emit_to("main", "snapshot-status-changed", message);
    });
}

fn reveal_window<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        if label == "quick-capture" {
            if let (Ok(Some(monitor)), Ok(size)) = (window.current_monitor(), window.outer_size()) {
                let monitor_size = monitor.size();
                let monitor_position = monitor.position();
                let x =
                    monitor_position.x + (monitor_size.width.saturating_sub(size.width) / 2) as i32;
                let y = monitor_position.y
                    + ((monitor_size.height as f64 * 0.34) as i32 - size.height as i32 / 2);
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }
        }
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.emit("window-revealed", ());
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir M/OS", true, None::<&str>)?;
    let capture = MenuItem::with_id(app, "capture", "Captura rapida", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &capture, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .tooltip("M/OS")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => reveal_window(app, "main"),
            "capture" => reveal_window(app, "quick-capture"),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

fn shortcut_error(error: tauri_plugin_global_shortcut::Error) -> CoreError {
    CoreError::new(
        mos_core::ErrorCode::InvalidInput,
        format!("Nao foi possivel registrar o atalho: {error}"),
        true,
    )
}

fn lock_error(message: String) -> CoreError {
    CoreError::new(
        mos_core::ErrorCode::StorageUnavailable,
        format!("Estado local indisponivel: {message}"),
        false,
    )
}

fn load_shortcut(path: &std::path::Path) -> String {
    fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<UserSettings>(&json).ok())
        .map(|settings| settings.capture_shortcut)
        .filter(|shortcut| !shortcut.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CAPTURE_SHORTCUT.into())
}

fn persist_shortcut(path: &std::path::Path, shortcut: &str) -> Result<(), CoreError> {
    let json = serde_json::to_vec_pretty(&UserSettings {
        capture_shortcut: shortcut.into(),
    })
    .map_err(|error| {
        CoreError::new(
            mos_core::ErrorCode::Io,
            format!("Nao foi possivel salvar a configuracao: {error}"),
            false,
        )
    })?;
    fs::write(path, json).map_err(|error| {
        CoreError::new(
            mos_core::ErrorCode::Io,
            format!("Nao foi possivel salvar a configuracao: {error}"),
            false,
        )
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            reveal_window(app, "main");
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        reveal_window(app, "quick-capture");
                    }
                })
                .build(),
        )
        .setup(|app| {
            let data_directory = app.path().app_data_dir()?;
            fs::create_dir_all(&data_directory)?;
            let settings_path = data_directory.join("settings.json");
            let configured_shortcut = load_shortcut(&settings_path);
            let storage = Arc::new(
                SqliteStorage::open(
                    data_directory.join("m-os.db"),
                    data_directory.join("backups"),
                )
                .map_err(|error| std::io::Error::other(error.to_string()))?,
            );
            app.manage(AppState {
                captures: CaptureService::new(storage.clone()),
                work: WorkService::new(storage.clone()),
                data: DataService::new(storage.clone()),
                storage,
                shortcut_status: Mutex::new("Registrando...".into()),
                active_shortcut: Mutex::new(None),
                snapshot_status: Arc::new(Mutex::new("Snapshot ainda nao verificado.".into())),
                settings_path,
            });

            let shortcut_status = match app.global_shortcut().register(configured_shortcut.as_str())
            {
                Ok(()) => {
                    *app.state::<AppState>()
                        .active_shortcut
                        .lock()
                        .map_err(|error| std::io::Error::other(error.to_string()))? =
                        Some(configured_shortcut.clone());
                    format!("Registrado: {configured_shortcut}")
                }
                Err(error) => format!("Atalho indisponivel: {error}"),
            };
            *app.state::<AppState>()
                .shortcut_status
                .lock()
                .map_err(|error| std::io::Error::other(error.to_string()))? = shortcut_status;
            setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            create_capture,
            get_capture,
            list_recent,
            list_inbox,
            list_archived,
            list_trashed,
            search_captures,
            search_all,
            mark_capture_processed,
            move_capture_to_inbox,
            archive_capture,
            trash_capture,
            restore_capture,
            rebuild_search,
            create_project,
            update_project,
            get_project,
            list_projects,
            set_project_archived,
            create_task,
            update_task,
            get_task,
            list_tasks,
            set_task_state,
            set_task_archived,
            create_backup,
            inspect_backup,
            restore_backup,
            export_json,
            get_app_status,
            set_capture_shortcut,
            show_quick_capture,
            hide_quick_capture,
        ])
        .run(tauri::generate_context!())
        .expect("error while running M/OS");
}
