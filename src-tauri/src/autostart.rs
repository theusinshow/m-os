//! Integracao com o autostart do sistema ("Iniciar com o Windows" — secao 13).
//!
//! Reflete a configuracao `start_with_windows` no registro de inicializacao do
//! SO via `tauri-plugin-autostart`. Falhas sao registradas, nunca propagadas: um
//! erro ao (des)registrar o autostart jamais deve impedir salvar as demais
//! configuracoes.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// Garante que o estado do autostart do SO corresponda a `enabled`.
pub fn sync(app: &AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    let current = manager.is_enabled().unwrap_or(false);
    if enabled == current {
        return;
    }
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(err) = result {
        eprintln!("falha ao ajustar o autostart: {err}");
    }
}
