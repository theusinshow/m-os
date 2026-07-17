//! Monitoramento de processos do Windows (secao 10).
//!
//! Um loop assincrono verifica periodicamente os processos em execucao e os
//! compara com a lista `monitored_apps` (apenas os habilitados). Detecta
//! transicoes fechado->aberto e aberto->fechado sem repetir eventos, registra
//! em `activity_events`, emite eventos Tauri e dispara notificacoes nativas com
//! cooldown. Le apenas nomes de processos — nunca conteudo (secao 6).
//!
//! O loop nao bloqueia (roda em tarefa propria), respeita a configuracao
//! `process_monitoring_enabled` (pode ser desligado) e pode ser encerrado de
//! forma limpa (secao 20).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::json;
use sysinfo::{ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_sql::DbInstances;

use crate::models::{MonitoredApp, Settings};
use crate::{database, notifications, reminder, repository};

/// Cooldown minimo (segundos) entre notificacoes do mesmo processo.
const NOTIFY_COOLDOWN_SECS: i64 = 60;

/// Estado em memoria do monitoramento, protegido por mutex.
#[derive(Default)]
struct MonitorState {
    /// Processos monitorados em execucao no ultimo tick.
    previous: HashSet<String>,
    /// Ultima notificacao por processo (epoch), para cooldown.
    last_notified: HashMap<String, i64>,
    /// Processos com lembrete suprimido ("nao lembrar hoje").
    suppressed_today: HashSet<String>,
    /// Dia atual das supressoes (reiniciadas quando o dia muda).
    suppressed_day: Option<chrono::NaiveDate>,
}

/// Estado compartilhado gerenciado pelo Tauri.
#[derive(Default)]
pub struct MonitorShared {
    state: Mutex<MonitorState>,
    stopped: AtomicBool,
}

impl MonitorShared {
    /// Sinaliza o loop para encerrar (chamado ao sair do app).
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }

    /// Suprime o lembrete de abertura de um processo pelo resto do dia.
    pub fn suppress_today(&self, process_name: &str) {
        if let Ok(mut st) = self.state.lock() {
            let today = chrono::Local::now().date_naive();
            st.suppressed_day = Some(today);
            st.suppressed_today.insert(process_name.to_lowercase());
        }
    }
}

/// Calcula as transicoes entre dois snapshots de processos em execucao.
/// Retorna (abertos, fechados). Funcao pura — base dos testes.
pub fn diff_transitions(
    previous: &HashSet<String>,
    current: &HashSet<String>,
) -> (Vec<String>, Vec<String>) {
    let opened = current.difference(previous).cloned().collect();
    let closed = previous.difference(current).cloned().collect();
    (opened, closed)
}

/// Conjunto de processos monitorados (habilitados) atualmente em execucao.
fn running_monitored(sys: &System, apps: &[MonitoredApp]) -> HashSet<String> {
    let wanted: HashSet<&str> = apps.iter().map(|a| a.process_name.as_str()).collect();
    let mut running = HashSet::new();
    for process in sys.processes().values() {
        let name = process.name().to_string_lossy().to_lowercase();
        if wanted.contains(name.as_str()) {
            running.insert(name);
        }
    }
    running
}

/// Loop principal do monitoramento. Deve ser lancado em uma tarefa (nao bloqueia).
pub async fn run(app: AppHandle) {
    let mut sys = System::new();

    loop {
        let shared = app.state::<MonitorShared>();
        if shared.stopped.load(Ordering::Relaxed) {
            break;
        }

        // Carrega configuracao atual; em erro (ex.: banco fechando), aguarda.
        let db = app.state::<DbInstances>();
        let Ok(pool) = database::pool(&db).await else {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };
        let Ok(settings) = repository::settings::get(&pool).await else {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };
        let interval = settings.process_check_interval_seconds.max(1) as u64;

        if settings.process_monitoring_enabled {
            let apps = repository::monitored_apps::list_enabled(&pool)
                .await
                .unwrap_or_default();

            // Snapshot do estado (reinicia supressoes se o dia mudou).
            let (previous, suppressed, mut cooldowns) = {
                let mut st = shared.state.lock().expect("mutex do monitoramento");
                let today = chrono::Local::now().date_naive();
                if st.suppressed_day != Some(today) {
                    st.suppressed_day = Some(today);
                    st.suppressed_today.clear();
                }
                (
                    st.previous.clone(),
                    st.suppressed_today.clone(),
                    st.last_notified.clone(),
                )
            };

            sys.refresh_processes(ProcessesToUpdate::All, true);
            let current = running_monitored(&sys, &apps);
            let (opened, closed) = diff_transitions(&previous, &current);
            let now_epoch = chrono::Utc::now().timestamp();

            for proc in &opened {
                handle_open(
                    &app,
                    &pool,
                    &apps,
                    proc,
                    &settings,
                    &suppressed,
                    &mut cooldowns,
                    now_epoch,
                )
                .await;
            }
            for proc in &closed {
                handle_close(
                    &app,
                    &pool,
                    &apps,
                    proc,
                    &settings,
                    &mut cooldowns,
                    now_epoch,
                )
                .await;
            }

            // Persiste o novo snapshot e os cooldowns atualizados.
            if let Ok(mut st) = shared.state.lock() {
                st.previous = current;
                st.last_notified = cooldowns;
            }
        }

        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

fn cooldown_ok(cooldowns: &HashMap<String, i64>, proc: &str, now: i64) -> bool {
    match cooldowns.get(proc) {
        Some(last) => now - last >= NOTIFY_COOLDOWN_SECS,
        None => true,
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_open(
    app: &AppHandle,
    pool: &sqlx::Pool<sqlx::Sqlite>,
    apps: &[MonitoredApp],
    proc: &str,
    settings: &Settings,
    suppressed: &HashSet<String>,
    cooldowns: &mut HashMap<String, i64>,
    now_epoch: i64,
) {
    let _ = repository::record_event(pool, "app_opened", Some(proc), None).await;

    // "Nao lembrar hoje": nao emite nem notifica (mas o evento fica registrado).
    if suppressed.contains(proc) {
        return;
    }

    let cfg = apps.iter().find(|a| a.process_name == proc);
    let display = cfg
        .map(|a| a.display_name.clone())
        .unwrap_or_else(|| proc.to_string());
    let has_timer = repository::timer::active(pool)
        .await
        .ok()
        .flatten()
        .is_some();

    let _ = app.emit(
        "monitored-app-opened",
        json!({ "processName": proc, "displayName": display, "hasActiveTimer": has_timer }),
    );

    let remind =
        settings.remind_when_monitored_app_opens && cfg.map(|a| a.remind_on_open).unwrap_or(false);
    if !has_timer && remind && cooldown_ok(cooldowns, proc, now_epoch) {
        // Widget flutuante sobre o CAD + notificacao nativa como reforco.
        reminder::show(app, proc, &display);
        notifications::notify(
            app,
            &format!("{display} foi aberto"),
            "Iniciar cronometro? Escolha o projeto na janela do CronoCAD.",
        );
        cooldowns.insert(proc.to_string(), now_epoch);
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_close(
    app: &AppHandle,
    pool: &sqlx::Pool<sqlx::Sqlite>,
    apps: &[MonitoredApp],
    proc: &str,
    settings: &Settings,
    cooldowns: &mut HashMap<String, i64>,
    now_epoch: i64,
) {
    let _ = repository::record_event(pool, "app_closed", Some(proc), None).await;

    let cfg = apps.iter().find(|a| a.process_name == proc);
    let display = cfg
        .map(|a| a.display_name.clone())
        .unwrap_or_else(|| proc.to_string());
    let has_timer = repository::timer::active(pool)
        .await
        .ok()
        .flatten()
        .is_some();

    let _ = app.emit(
        "monitored-app-closed",
        json!({ "processName": proc, "displayName": display, "hasActiveTimer": has_timer }),
    );

    let remind = settings.remind_when_monitored_app_closes
        && cfg.map(|a| a.remind_on_close).unwrap_or(false);
    if has_timer && remind && cooldown_ok(cooldowns, proc, now_epoch) {
        notifications::notify(
            app,
            &format!("{display} foi fechado"),
            "Deseja encerrar o registro atual? Abra o CronoCAD.",
        );
        cooldowns.insert(proc.to_string(), now_epoch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn detecta_abertura_e_fechamento() {
        let prev = set(&["acad.exe"]);
        let curr = set(&["acad.exe", "revit.exe"]);
        let (opened, closed) = diff_transitions(&prev, &curr);
        assert_eq!(opened, vec!["revit.exe".to_string()]);
        assert!(closed.is_empty());

        let (opened2, closed2) = diff_transitions(&curr, &prev);
        assert!(opened2.is_empty());
        assert_eq!(closed2, vec!["revit.exe".to_string()]);
    }

    #[test]
    fn sem_mudanca_nao_gera_transicao() {
        let s = set(&["acad.exe", "revit.exe"]);
        let (opened, closed) = diff_transitions(&s, &s);
        assert!(opened.is_empty());
        assert!(closed.is_empty());
    }

    #[test]
    fn cooldown_respeitado() {
        let mut cooldowns = HashMap::new();
        assert!(cooldown_ok(&cooldowns, "acad.exe", 1000));
        cooldowns.insert("acad.exe".to_string(), 1000);
        assert!(!cooldown_ok(
            &cooldowns,
            "acad.exe",
            1000 + NOTIFY_COOLDOWN_SECS - 1
        ));
        assert!(cooldown_ok(
            &cooldowns,
            "acad.exe",
            1000 + NOTIFY_COOLDOWN_SECS
        ));
    }
}
