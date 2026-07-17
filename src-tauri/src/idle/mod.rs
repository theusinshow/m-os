//! Deteccao de inatividade (secao 11).
//!
//! Consulta ha quanto tempo nao ha entrada de teclado/mouse via a API do
//! Windows `GetLastInputInfo`. **Nao** captura quais teclas, coordenadas ou
//! conteudo — apenas o tempo ocioso. Quando o limite configurado e ultrapassado
//! registra `idle_started`; ao retornar a atividade, registra `idle_ended` e
//! notifica o frontend para que o usuario decida sobre o periodo (manter,
//! descontar ou editar) — nunca descontando automaticamente.
//!
//! O loop nao bloqueia, respeita `idle_detection_enabled` e encerra de forma
//! limpa (secao 20).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_sql::DbInstances;

use crate::{database, repository};

/// Intervalo de amostragem do tempo ocioso.
const IDLE_POLL_SECS: u64 = 5;

/// Segundos desde a ultima entrada de teclado/mouse do sistema.
#[cfg(windows)]
pub fn system_idle_seconds() -> i64 {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `info` esta corretamente inicializado com seu `cbSize`; a API
    // apenas preenche `dwTime` com o tick da ultima entrada.
    let ok = unsafe { GetLastInputInfo(&mut info).as_bool() };
    if !ok {
        return 0;
    }
    let now = unsafe { GetTickCount() };
    let idle_ms = now.wrapping_sub(info.dwTime);
    (idle_ms / 1000) as i64
}

/// Fora do Windows nao ha deteccao (o app so tem alvo Windows no MVP).
#[cfg(not(windows))]
pub fn system_idle_seconds() -> i64 {
    0
}

/// Transicao do estado de inatividade. Funcao pura — base dos testes.
#[derive(Debug, PartialEq, Eq)]
pub enum IdleTransition {
    BecameIdle,
    BecameActive,
    NoChange,
}

pub fn classify(was_idle: bool, idle_seconds: i64, threshold_seconds: i64) -> IdleTransition {
    let now_idle = idle_seconds >= threshold_seconds;
    match (was_idle, now_idle) {
        (false, true) => IdleTransition::BecameIdle,
        (true, false) => IdleTransition::BecameActive,
        _ => IdleTransition::NoChange,
    }
}

#[derive(Default)]
struct IdleState {
    is_idle: bool,
    idle_start_epoch: i64,
}

/// Estado compartilhado da deteccao de inatividade.
#[derive(Default)]
pub struct IdleShared {
    state: Mutex<IdleState>,
    stopped: AtomicBool,
}

impl IdleShared {
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}

async fn running_timer_exists(pool: &sqlx::Pool<sqlx::Sqlite>) -> bool {
    matches!(
        repository::timer::active(pool).await,
        Ok(Some(t)) if t.status == "running"
    )
}

/// Loop principal da deteccao de inatividade. Lancado em tarefa propria.
pub async fn run(app: AppHandle) {
    loop {
        let shared = app.state::<IdleShared>();
        if shared.stopped.load(Ordering::Relaxed) {
            break;
        }

        let db = app.state::<DbInstances>();
        let Ok(pool) = database::pool(&db).await else {
            tokio::time::sleep(Duration::from_secs(IDLE_POLL_SECS)).await;
            continue;
        };
        let Ok(settings) = repository::settings::get(&pool).await else {
            tokio::time::sleep(Duration::from_secs(IDLE_POLL_SECS)).await;
            continue;
        };

        if settings.idle_detection_enabled {
            let threshold = settings.idle_threshold_minutes.max(1) * 60;
            let idle = system_idle_seconds();
            let was_idle = shared.state.lock().map(|s| s.is_idle).unwrap_or(false);
            let now_epoch = chrono::Utc::now().timestamp();

            match classify(was_idle, idle, threshold) {
                IdleTransition::BecameIdle => {
                    if let Ok(mut st) = shared.state.lock() {
                        st.is_idle = true;
                        st.idle_start_epoch = now_epoch - idle;
                    }
                    let _ = repository::record_event(&pool, "idle_started", None, None).await;
                    let _ = app.emit("idle-started", json!({ "idleSeconds": idle }));
                }
                IdleTransition::BecameActive => {
                    let start = shared
                        .state
                        .lock()
                        .map(|s| s.idle_start_epoch)
                        .unwrap_or(now_epoch);
                    if let Ok(mut st) = shared.state.lock() {
                        st.is_idle = false;
                    }
                    let duration = (now_epoch - start).max(0);
                    let _ = repository::record_event(
                        &pool,
                        "idle_ended",
                        None,
                        Some(&format!("{{\"idleSeconds\":{duration}}}")),
                    )
                    .await;
                    let has_timer = running_timer_exists(&pool).await;
                    let _ = app.emit(
                        "idle-ended",
                        json!({ "idleSeconds": duration, "hasActiveTimer": has_timer }),
                    );
                }
                IdleTransition::NoChange => {}
            }
        } else if let Ok(mut st) = shared.state.lock() {
            // Monitoramento desligado: zera o estado para nao emitir ao religar.
            st.is_idle = false;
        }

        tokio::time::sleep(Duration::from_secs(IDLE_POLL_SECS)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifica_transicoes_de_inatividade() {
        // limite = 600s (10 min)
        assert_eq!(classify(false, 500, 600), IdleTransition::NoChange);
        assert_eq!(classify(false, 600, 600), IdleTransition::BecameIdle);
        assert_eq!(classify(true, 700, 600), IdleTransition::NoChange);
        assert_eq!(classify(true, 3, 600), IdleTransition::BecameActive);
    }
}
