//! Servico centralizado de notificacoes (secao 16).
//!
//! Envia notificacoes nativas via `tauri-plugin-notification`. O controle de
//! repeticao (cooldown e "nao lembrar hoje") fica no servico de monitoramento,
//! que decide *quando* chamar `notify`.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Mostra uma notificacao nativa. Falhas sao registradas, nunca propagadas: uma
/// notificacao que nao aparece jamais deve derrubar o monitoramento.
pub fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        eprintln!("falha ao exibir notificacao: {err}");
    }
}
