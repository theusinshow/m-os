//! Bandeja do sistema (secao 15).
//!
//! O menu reflete o estado do cronometro: os itens sao habilitados/desabilitados
//! conforme haja cronometro ativo e seu status, e "Projeto atual" mostra o nome.
//! As acoes de pausar/continuar/encerrar chamam o mesmo `timer_service` usado
//! pelos comandos; "Iniciar trabalho" e "Abrir" trazem a janela para o usuario
//! escolher o projeto.

use tauri::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Wry,
};

use crate::timer_service;

/// IDs dos itens de menu, centralizados para evitar strings soltas.
pub mod ids {
    pub const OPEN: &str = "tray_open";
    pub const START: &str = "tray_start";
    pub const PAUSE: &str = "tray_pause";
    pub const RESUME: &str = "tray_resume";
    pub const STOP: &str = "tray_stop";
    pub const QUIT: &str = "tray_quit";
}

/// Handles dos itens de menu dependentes de estado, guardados no estado do app
/// para atualizacao dinamica (habilitar/desabilitar e texto).
pub struct TrayItems {
    start: MenuItem<Wry>,
    pause: MenuItem<Wry>,
    resume: MenuItem<Wry>,
    stop: MenuItem<Wry>,
    current: MenuItem<Wry>,
}

/// Constroi o icone e o menu da bandeja e registra os manipuladores.
pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, ids::OPEN, "Abrir CronoCAD", true, None::<&str>)?;
    let start = MenuItem::with_id(app, ids::START, "Iniciar trabalho", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, ids::PAUSE, "Pausar cronometro", false, None::<&str>)?;
    let resume = MenuItem::with_id(
        app,
        ids::RESUME,
        "Continuar cronometro",
        false,
        None::<&str>,
    )?;
    let stop = MenuItem::with_id(app, ids::STOP, "Encerrar cronometro", false, None::<&str>)?;
    let current = MenuItem::with_id(
        app,
        "tray_current_project",
        "Projeto atual: —",
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, ids::QUIT, "Sair completamente", true, None::<&str>)?;

    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &open, &sep1, &start, &pause, &resume, &stop, &current, &sep2, &quit,
        ],
    )?;

    // Guarda os handles para atualizacao dinamica do estado.
    app.manage(TrayItems {
        start: start.clone(),
        pause: pause.clone(),
        resume: resume.clone(),
        stop: stop.clone(),
        current: current.clone(),
    });

    let mut builder = TrayIconBuilder::with_id("cronocad-tray")
        .tooltip("CronoCAD")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event);

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Atualiza os itens conforme o estado do cronometro. `state` = (status, nome do
/// projeto) quando ha cronometro, ou `None` quando nao ha.
pub fn update_state(app: &AppHandle, state: Option<(String, String)>) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let running = matches!(&state, Some((s, _)) if s == "running");
    let paused = matches!(&state, Some((s, _)) if s == "paused");
    let has_timer = state.is_some();

    let _ = items.start.set_enabled(!has_timer);
    let _ = items.pause.set_enabled(running);
    let _ = items.resume.set_enabled(paused);
    let _ = items.stop.set_enabled(has_timer);

    let name = state
        .map(|(_, n)| if n.is_empty() { "—".to_string() } else { n })
        .unwrap_or_else(|| "—".to_string());
    let _ = items.current.set_text(format!("Projeto atual: {name}"));
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        ids::OPEN | ids::START => show_main_window(app),
        ids::PAUSE => spawn(app, Action::Pause),
        ids::RESUME => spawn(app, Action::Resume),
        ids::STOP => spawn(app, Action::Stop),
        ids::QUIT => request_quit(app),
        _ => {}
    }
}

/// Sair completamente: se houver cronometro ativo, traz a janela e pede
/// confirmacao (evento `request-quit`); caso contrario, encerra direto. O estado
/// do cronometro ja esta persistido, entao a recuperacao cobre qualquer caso.
fn request_quit(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let has_timer = timer_service::get_active(&app)
            .await
            .ok()
            .flatten()
            .is_some();
        if has_timer {
            show_main_window(&app);
            let _ = app.emit("request-quit", ());
        } else {
            app.exit(0);
        }
    });
}

enum Action {
    Pause,
    Resume,
    Stop,
}

fn spawn(app: &AppHandle, action: Action) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = match action {
            Action::Pause => timer_service::pause(&app).await.map(|_| ()),
            Action::Resume => timer_service::resume(&app).await.map(|_| ()),
            Action::Stop => timer_service::stop(&app).await.map(|_| ()),
        };
        if let Err(err) = result {
            eprintln!("acao da bandeja falhou: {err}");
        }
    });
}

/// Mostra e foca a janela principal (usado pela bandeja e por notificacoes).
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
