//! O laco que observa: programas abertos e periodos parados (ADR-032).
//!
//! Le APENAS nomes de executavel e o tempo desde o ultimo toque no teclado ou
//! mouse. Nunca titulo de janela, nunca conteudo de arquivo, nunca captura de
//! tela. Essa fronteira e o que separa observar de vigiar, e ela e mantida pela
//! escolha da API — nao por disciplina de quem escreve a proxima linha.
//!
//! A regra que atravessa o modulo inteiro: **observacao nao vira hora sozinha**.
//! O evento fica guardado, a Linha do Tempo mostra o vao, e quem decide se
//! aquilo foi trabalho e a pessoa.

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use mos_core::{ActivityKind, NewActivityEvent};
use sysinfo::{ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::AppState;

/// Espera minima entre dois lembretes do mesmo programa.
///
/// Sem ela, um AutoCAD que fecha e reabre tres vezes em dois minutos — coisa
/// banal ao trocar de arquivo — dispara tres notificacoes, e a quarta o usuario
/// desliga o recurso.
const REMINDER_COOLDOWN: Duration = Duration::from_secs(60);

/// Espera quando o banco nao responde. Nao desiste: o banco pode estar em
/// migration, e desistir mataria o monitoramento ate o proximo reinicio.
const RETRY: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Observed {
    /// Processos monitorados vistos rodando na ultima passada.
    running: BTreeSet<String>,
    /// Quando cada processo notificou pela ultima vez, em epoch.
    last_reminder: HashMap<String, i64>,
    /// Se o usuario ja esta contado como parado agora.
    idle: bool,
}

/// O estado do laco, compartilhado com o resto do aplicativo.
#[derive(Default)]
pub struct Monitor {
    observed: Mutex<Observed>,
    stopped: AtomicBool,
}

impl Monitor {
    /// Encerra o laco. Chamado ao sair — um laco que sobrevive a janela mantem
    /// o processo vivo depois de o usuario ter fechado o aplicativo.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}

/// Ha quantos segundos ninguem toca no teclado ou no mouse.
///
/// `GetLastInputInfo` devolve a marca em milissegundos desde o boot, e essa
/// marca e de 32 bits: ela da a volta a cada 49 dias e pode ficar MAIOR que o
/// tempo atual. A subtracao usa `wrapping_sub` por isso — sem ela, a volta do
/// contador viraria uma inatividade de 49 dias, e o dia inteiro de trabalho
/// seria descontado da fatura.
#[cfg(windows)]
fn idle_seconds() -> Option<i64> {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // Zero e falha. Devolver `None` mantem o estado anterior em vez de afirmar
    // que o usuario acabou de mexer — afirmar seria apagar uma inatividade real.
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return None;
    }
    let now = unsafe { GetTickCount() };
    Some(i64::from(now.wrapping_sub(info.dwTime)) / 1_000)
}

/// Fora do Windows nao ha o que medir, e fingir zero seria pior: zero significa
/// "acabou de mexer", e a Linha do Tempo passaria a nunca ver inatividade.
#[cfg(not(windows))]
fn idle_seconds() -> Option<i64> {
    None
}

/// Os processos monitorados que estao rodando agora.
///
/// Compara em minusculas porque o Windows nao diferencia, e o cadastro pode ter
/// sido digitado como `Revit.exe`.
fn running_monitored(system: &System, wanted: &BTreeSet<String>) -> BTreeSet<String> {
    system
        .processes()
        .values()
        .filter_map(|process| {
            let name = process.name().to_string_lossy().to_lowercase();
            wanted.contains(&name).then_some(name)
        })
        .collect()
}

/// Roda ate `stop()`. Deve ser lancado em tarefa propria — nao bloqueia.
pub async fn run<R: Runtime>(app: AppHandle<R>) {
    let mut system = System::new();

    loop {
        if app.state::<Monitor>().stopped.load(Ordering::Relaxed) {
            break;
        }

        let state = app.state::<AppState>();
        let Ok(settings) = state.monitoring.settings() else {
            tokio::time::sleep(RETRY).await;
            continue;
        };

        if settings.process_monitoring_enabled {
            let apps = state.monitoring.apps().unwrap_or_default();
            let wanted: BTreeSet<String> = apps
                .iter()
                .filter(|entry| entry.enabled)
                .map(|entry| entry.process_name.to_lowercase())
                .collect();

            if !wanted.is_empty() {
                system.refresh_processes(ProcessesToUpdate::All, true);
                let current = running_monitored(&system, &wanted);
                let previous = {
                    let observed = app.state::<Monitor>();
                    let guard = observed.observed.lock();
                    guard.map(|state| state.running.clone()).unwrap_or_default()
                };

                let (opened, closed) = mos_core::diff_transitions(&previous, &current);
                for process in &opened {
                    announce(&app, process, &apps, ActivityKind::AppOpened, &settings);
                }
                for process in &closed {
                    announce(&app, process, &apps, ActivityKind::AppClosed, &settings);
                }

                if let Ok(mut observed) = app.state::<Monitor>().observed.lock() {
                    observed.running = current;
                }
            }
        }

        if settings.idle_detection_enabled {
            check_idle(&app, settings.idle_threshold_minutes);
        }

        tokio::time::sleep(Duration::from_secs(
            settings.check_interval_seconds.max(1) as u64
        ))
        .await;
    }
}

/// Grava o evento, avisa a interface e — se couber — lembra.
///
/// O evento e gravado ANTES de qualquer decisao sobre lembrete, inclusive
/// quando o lembrete esta desligado: a Linha do Tempo e feita do que aconteceu,
/// e nao do que o usuario escolheu ser avisado sobre.
fn announce<R: Runtime>(
    app: &AppHandle<R>,
    process: &str,
    apps: &[mos_core::MonitoredApp],
    kind: ActivityKind,
    settings: &mos_core::MonitoringSettings,
) {
    let state = app.state::<AppState>();
    let _ = state.monitoring.record(NewActivityEvent {
        kind,
        process_name: process.to_string(),
        detected_at: time::OffsetDateTime::now_utc(),
    });

    let entry = apps
        .iter()
        .find(|entry| entry.process_name.eq_ignore_ascii_case(process));
    let display = entry
        .map(|entry| entry.display_name.clone())
        .unwrap_or_else(|| process.to_string());
    let opened = kind == ActivityKind::AppOpened;
    let running = state.tracking.active_timer().ok().flatten().is_some();

    let _ = app.emit(
        if opened {
            "monitored-app-opened"
        } else {
            "monitored-app-closed"
        },
        serde_json::json!({
            "processName": process,
            "displayName": display,
            "hasActiveTimer": running,
        }),
    );

    // Abriu o CAD e NAO ha cronometro: o trabalho pode comecar sem registro.
    // Fechou o CAD e HA cronometro: o registro pode continuar sem trabalho.
    // Fora esses dois casos o lembrete seria ruido.
    let useful = if opened { !running } else { running };
    let allowed = if opened {
        settings.remind_on_open && entry.map(|entry| entry.remind_on_open).unwrap_or(false)
    } else {
        settings.remind_on_close && entry.map(|entry| entry.remind_on_close).unwrap_or(false)
    };
    if !useful || !allowed || !cooldown_passed(app, process) {
        return;
    }

    remind(
        app,
        &format!(
            "{display} {}",
            if opened { "foi aberto" } else { "foi fechado" }
        ),
        if opened {
            "Iniciar o cronometro? Escolha o Project na pagina de Tempo."
        } else {
            "Encerrar o registro atual? A pagina de Tempo tem o botao."
        },
    );
}

/// True quando o cooldown venceu — e ja marca o novo instante.
fn cooldown_passed<R: Runtime>(app: &AppHandle<R>, process: &str) -> bool {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let monitor = app.state::<Monitor>();
    let Ok(mut observed) = monitor.observed.lock() else {
        return false;
    };
    let fresh = observed
        .last_reminder
        .get(process)
        .is_none_or(|last| now - last >= REMINDER_COOLDOWN.as_secs() as i64);
    if fresh {
        observed.last_reminder.insert(process.to_string(), now);
    }
    fresh
}

/// Marca a entrada e a saida da inatividade, uma vez cada.
///
/// O evento nasce quando o limiar e CRUZADO, e nao a cada passada: um usuario
/// almocando geraria um evento a cada cinco segundos, e a Linha do Tempo do dia
/// viraria uma parede de eventos identicos.
fn check_idle<R: Runtime>(app: &AppHandle<R>, threshold_minutes: i64) {
    let Some(seconds) = idle_seconds() else {
        return;
    };
    let idle_now = seconds >= threshold_minutes.max(1) * 60;

    let changed = {
        let monitor = app.state::<Monitor>();
        let Ok(mut observed) = monitor.observed.lock() else {
            return;
        };
        let changed = observed.idle != idle_now;
        observed.idle = idle_now;
        changed
    };
    if !changed {
        return;
    }

    let _ = app.state::<AppState>().monitoring.record(NewActivityEvent {
        kind: if idle_now {
            ActivityKind::IdleStarted
        } else {
            ActivityKind::IdleEnded
        },
        process_name: String::new(),
        detected_at: time::OffsetDateTime::now_utc(),
    });
    let _ = app.emit("idle-changed", idle_now);
}

/// Notificacao do sistema. Falha em silencio de proposito: um lembrete que nao
/// saiu e um lembrete perdido, nao um erro que valha interromper o usuario.
fn remind<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}
