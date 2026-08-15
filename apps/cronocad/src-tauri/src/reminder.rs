//! Widget flutuante de lembrete (janela `reminder`).
//!
//! Uma pequena janela sempre-no-topo, sem bordas e fora da barra de tarefas,
//! exibida sobre o programa CAD quando ele e aberto sem cronometro ativo
//! (secao 10). O conteudo (escolher projeto / iniciar) e renderizado pelo
//! frontend; aqui posicionamos, exibimos e guardamos o lembrete pendente para
//! que o widget o recupere ao carregar (robusto a corrida de inicializacao).

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, PhysicalPosition};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInfo {
    pub process_name: String,
    pub display_name: String,
}

/// Ultimo lembrete a exibir, lido pelo widget ao carregar.
#[derive(Default)]
pub struct ReminderState(pub Mutex<Option<ReminderInfo>>);

/// Guarda o lembrete pendente, posiciona o widget no canto superior direito e o
/// exibe sobre o CAD.
pub fn show(app: &AppHandle, process_name: &str, display_name: &str) {
    if let Some(state) = app.try_state::<ReminderState>() {
        if let Ok(mut pending) = state.0.lock() {
            *pending = Some(ReminderInfo {
                process_name: process_name.to_string(),
                display_name: display_name.to_string(),
            });
        }
    }

    let Some(window) = app.get_webview_window("reminder") else {
        return;
    };

    if let (Ok(Some(monitor)), Ok(win_size)) = (window.current_monitor(), window.outer_size()) {
        let margin = 24i32;
        let screen = monitor.size();
        let x = (screen.width as i32 - win_size.width as i32 - margin).max(0);
        let y = margin;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }

    let _ = window.show();
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
}

/// Lembrete pendente (para o widget recuperar ao carregar).
pub fn pending(app: &AppHandle) -> Option<ReminderInfo> {
    app.try_state::<ReminderState>()
        .and_then(|s| s.0.lock().ok().and_then(|g| g.clone()))
}
