mod storage;

use std::{fs, sync::Mutex};

use serde::Serialize;
use storage::{CaptureReceipt, CaptureRow, Storage, StorageStatus};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const CAPTURE_SHORTCUT: &str = "Ctrl+Shift+Space";

struct AppState {
    storage: Storage,
    shortcut_status: Mutex<String>,
    active_shortcut: Mutex<Option<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpikeStatus {
    shell: &'static str,
    shortcut: String,
    storage: StorageStatus,
}

#[tauri::command]
fn save_capture(
    content: &str,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<CaptureReceipt, String> {
    let receipt = state.storage.save_capture(content)?;
    let _ = app.emit_to("main", "capture-saved", receipt.id);
    Ok(receipt)
}

#[tauri::command]
fn list_captures(
    query: Option<&str>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<CaptureRow>, String> {
    state.storage.list_captures(query)
}

#[tauri::command]
fn get_spike_status(state: tauri::State<'_, AppState>) -> Result<SpikeStatus, String> {
    Ok(SpikeStatus {
        shell: "Tauri 2 + WebView2",
        shortcut: state
            .shortcut_status
            .lock()
            .map_err(|error| error.to_string())?
            .clone(),
        storage: state.storage.status()?,
    })
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

#[tauri::command]
fn set_capture_shortcut(
    shortcut: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let requested = shortcut.trim();
    if requested.is_empty() {
        return Err("Informe um atalho.".into());
    }

    let mut active = state
        .active_shortcut
        .lock()
        .map_err(|error| error.to_string())?;
    if active.as_deref() == Some(requested) {
        return Ok(format!("registered: {requested}"));
    }

    let previous = active.take();
    if let Some(previous) = &previous {
        app.global_shortcut()
            .unregister(previous.as_str())
            .map_err(|error| error.to_string())?;
    }

    match app.global_shortcut().register(requested) {
        Ok(()) => {
            *active = Some(requested.into());
            let message = format!("registered: {requested}");
            *state
                .shortcut_status
                .lock()
                .map_err(|error| error.to_string())? = message.clone();
            Ok(message)
        }
        Err(error) => {
            let mut message = format!("registration failed for {requested}: {error}");
            if let Some(previous) = previous {
                match app.global_shortcut().register(previous.as_str()) {
                    Ok(()) => {
                        *active = Some(previous.clone());
                        message.push_str(&format!("; restored {previous}"));
                    }
                    Err(restore_error) => {
                        message.push_str(&format!("; restore also failed: {restore_error}"));
                    }
                }
            }
            *state
                .shortcut_status
                .lock()
                .map_err(|lock_error| lock_error.to_string())? = message.clone();
            Err(message)
        }
    }
}

fn reveal_window<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir M/OS shell", true, None::<&str>)?;
    let capture = MenuItem::with_id(app, "capture", "Captura rapida", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &capture, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .tooltip("M/OS shell spike")
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            reveal_window(app, "main");
        }))
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
            let data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&data_dir)?;
            let storage =
                Storage::open(&data_dir.join("shell-spike.db")).map_err(std::io::Error::other)?;
            app.manage(AppState {
                storage,
                shortcut_status: Mutex::new("registration pending".into()),
                active_shortcut: Mutex::new(None),
            });

            let shortcut_status = match app.global_shortcut().register(CAPTURE_SHORTCUT) {
                Ok(()) => {
                    *app.state::<AppState>()
                        .active_shortcut
                        .lock()
                        .map_err(|error| std::io::Error::other(error.to_string()))? =
                        Some(CAPTURE_SHORTCUT.into());
                    format!("registered: {CAPTURE_SHORTCUT}")
                }
                Err(error) => format!("registration failed: {error}"),
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
            save_capture,
            list_captures,
            get_spike_status,
            show_quick_capture,
            hide_quick_capture,
            set_capture_shortcut
        ])
        .run(tauri::generate_context!())
        .expect("error while running the M/OS shell spike");
}
