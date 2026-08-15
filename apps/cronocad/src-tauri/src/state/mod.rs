//! Estado compartilhado da aplicacao mantido pelo Tauri.
//!
//! Nesta fundacao guarda apenas metadados. Nas proximas fases passara a conter
//! handles dos servicos de monitoramento e inatividade (para poderem ser
//! encerrados corretamente — secao 20) e um cache do estado do cronometro.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

impl Default for AppInfo {
    fn default() -> Self {
        Self {
            name: "CronoCAD".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
