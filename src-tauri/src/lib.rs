//! Ponto de composicao do backend Tauri do CronoCAD.
//!
//! Responsabilidades desta fundacao:
//!  * garantir instancia unica, trazendo a janela existente de volta;
//!  * registrar o plugin SQL com as migrations versionadas;
//!  * registrar o plugin de notificacao;
//!  * construir a bandeja do sistema;
//!  * aplicar "fechar para a bandeja" (secao 15), mantendo o app ativo.
//!
//! Alem disso, registra os comandos de dominio, gerencia o estado do
//! monitoramento de processos (secao 10) e o inicia em tarefa propria,
//! encerrando-o de forma limpa ao sair. A deteccao de inatividade (secao 11)
//! entra na Fase 5.

mod autostart;
mod commands;
mod database;
mod domain;
mod error;
mod idle;
mod models;
mod monitoring;
mod notifications;
mod pdf;
mod reminder;
mod repository;
mod state;
mod timer_service;
mod tray;

use tauri::{Manager, RunEvent, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Instancia unica. Precisa ser o PRIMEIRO plugin registrado.
    //
    // O app fecha para a bandeja e continua vivo (ver `on_window_event` abaixo).
    // Sem esta guarda, abrir o app pelo icone com ele ja rodando escondido subia
    // um segundo processo sobre o mesmo SQLite: a janela nao aparecia e os dois
    // processos disputavam o banco. Agora a tentativa de abrir de novo apenas
    // traz a janela existente de volta.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        tray::show_main_window(app);
    }));

    builder
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(database::DB_URL, database::migrations())
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_clients,
            commands::get_client,
            commands::create_client,
            commands::update_client,
            commands::archive_client,
            commands::list_projects,
            commands::get_project,
            commands::create_project,
            commands::update_project,
            commands::list_project_totals,
            commands::set_project_status,
            commands::update_project_notes,
            commands::list_todos,
            commands::create_todo,
            commands::set_todo_done,
            commands::update_todo_text,
            commands::delete_todo,
            commands::get_active_timer,
            commands::start_timer,
            commands::pause_timer,
            commands::resume_timer,
            commands::stop_timer,
            commands::discard_timer,
            commands::list_time_entries,
            commands::create_time_entry,
            commands::update_time_entry,
            commands::delete_time_entry,
            commands::restore_time_entry,
            commands::list_activity_events,
            commands::save_text_file,
            commands::export_report_pdf,
            commands::export_invoice_pdf,
            commands::get_pending_reminder,
            commands::quit_app,
            commands::get_settings,
            commands::update_settings,
            commands::list_monitored_apps,
            commands::create_monitored_app,
            commands::update_monitored_app,
            commands::delete_monitored_app,
            commands::suppress_app_reminder_today,
            commands::discount_idle,
        ])
        .manage(monitoring::MonitorShared::default())
        .manage(idle::IdleShared::default())
        .manage(reminder::ReminderState::default())
        .setup(|app| {
            tray::build_tray(app.handle())?;

            // Sincroniza a bandeja com um eventual cronometro remanescente e
            // notifica o frontend (recuperacao — secao 9).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                timer_service::refresh(&handle).await;
            });

            // Inicia o monitoramento de processos em tarefa propria (secao 10).
            let monitor_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                monitoring::run(monitor_handle).await;
            });

            // Inicia a deteccao de inatividade em tarefa propria (secao 11).
            let idle_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                idle::run(idle_handle).await;
            });

            // Alinha o autostart do SO com a configuracao persistida (secao 13).
            let autostart_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let db = autostart_handle.state::<tauri_plugin_sql::DbInstances>();
                if let Ok(pool) = database::pool(&db).await {
                    if let Ok(settings) = repository::settings::get(&pool).await {
                        autostart::sync(&autostart_handle, settings.start_with_windows);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Fechar para a bandeja: intercepta o fechamento da janela principal
            // e apenas a esconde, mantendo o app vivo (secao 15). O encerramento
            // definitivo acontece por "Sair completamente" na bandeja.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("erro ao iniciar a aplicacao CronoCAD")
        .run(|app, event| {
            // Encerra os loops de monitoramento e inatividade de forma limpa ao
            // sair (secao 20).
            if let RunEvent::Exit = event {
                app.state::<monitoring::MonitorShared>().stop();
                app.state::<idle::IdleShared>().stop();
            }
        });
}
