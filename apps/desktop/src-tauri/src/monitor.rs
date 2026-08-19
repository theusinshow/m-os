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

/// O que a janelinha de lembrete precisa saber para se desenhar.
///
/// Fica no estado e nao so no evento porque a janela pode nascer DEPOIS de o
/// evento ter sido emitido: na primeira abertura ela ainda esta carregando o
/// bundle quando o AutoCAD ja abriu. Um lembrete que depende de a janela estar
/// pronta e um lembrete que se perde justamente na primeira vez.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingReminder {
    pub process_name: String,
    pub display_name: String,
    /// `true` = abriu o programa; `false` = fechou.
    pub opened: bool,
    pub has_active_timer: bool,
}

/// Vinte segundos de microfone aberto antes de oferecer.
///
/// Nao e conservadorismo: microfone que abre por dois segundos e teste de som,
/// atalho de push-to-talk, notificacao do sistema. Reuniao mantem aberto. Sem a
/// espera o popup vira ruido, e popup ruidoso e desligado no primeiro dia — o
/// que custa a feature inteira em troca de nada.
const ESPERA_DE_OFERTA: i64 = 20;

#[derive(Default)]
struct Observed {
    /// Processos monitorados vistos rodando na ultima passada.
    running: BTreeSet<String>,
    /// Quando cada processo notificou pela ultima vez, em epoch.
    last_reminder: HashMap<String, i64>,
    /// Para qual processo a oferta de gravar ja foi mostrada.
    ///
    /// Sem isto a janelinha reapareceria a cada volta do laco, que sao poucos
    /// segundos. Limpa quando o processo some da lista de microfones abertos —
    /// entao um Meet que cai e volta oferece de novo, e isso e desejado: pode
    /// ser outra reuniao.
    ja_ofereceu: Option<String>,
    /// Ate quando cada processo esta silenciado, em epoch.
    ///
    /// O instante vem da INTERFACE e nao daqui: "hoje" acaba a meia-noite
    /// local, e o backend guarda tudo em UTC sem saber o fuso de quem clicou.
    /// A janela calcula o fim do dia dela e manda o instante pronto.
    suppressed_until: HashMap<String, i64>,
    /// Se o usuario ja esta contado como parado agora.
    idle: bool,
    /// O lembrete que a janelinha deve mostrar quando abrir.
    pending: Option<PendingReminder>,
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

    /// O lembrete pendente, se houver.
    pub fn pending(&self) -> Option<PendingReminder> {
        self.observed
            .lock()
            .ok()
            .and_then(|state| state.pending.clone())
    }

    pub fn clear_pending(&self) {
        if let Ok(mut state) = self.observed.lock() {
            state.pending = None;
        }
    }

    /// Silencia um programa ate o instante dado (epoch em segundos).
    pub fn suppress(&self, process_name: &str, until_epoch: i64) {
        if let Ok(mut state) = self.observed.lock() {
            state.pending = None;
            state
                .suppressed_until
                .insert(process_name.to_lowercase(), until_epoch);
        }
    }

    /// Quais programas estao silenciados agora, e ate quando.
    ///
    /// Existe para a tela poder MOSTRAR o silencio. Um modo que se liga e nunca
    /// mais aparece e um modo que o usuario esquece que ligou — e semanas
    /// depois ele conclui que o lembrete parou de funcionar.
    pub fn silenced_now(&self, now: i64) -> Vec<(String, i64)> {
        let Ok(state) = self.observed.lock() else {
            return Vec::new();
        };
        let mut list: Vec<(String, i64)> = state
            .suppressed_until
            .iter()
            .filter(|(_, until)| now < **until)
            .map(|(process, until)| (process.clone(), *until))
            .collect();
        list.sort();
        list
    }

    /// Volta a lembrar deste programa.
    pub fn unsilence(&self, process_name: &str) {
        if let Ok(mut state) = self.observed.lock() {
            state.suppressed_until.remove(&process_name.to_lowercase());
        }
    }

    /// Silenciado agora? Limpa a marca vencida de passagem, para o mapa nao
    /// crescer com programas silenciados semanas atras.
    fn silenced(&self, process_name: &str, now: i64) -> bool {
        let Ok(mut state) = self.observed.lock() else {
            return false;
        };
        match state.suppressed_until.get(process_name).copied() {
            Some(until) if now < until => true,
            Some(_) => {
                state.suppressed_until.remove(process_name);
                false
            }
            None => false,
        }
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

        // A deteccao de reuniao anda no MESMO laco, e nao num proprio: as duas
        // perguntam ao sistema no mesmo ritmo, e dois lacos acordando a cada
        // poucos segundos custam bateria por nada.
        //
        // TEMPORARIO ate a migration 0023: `ligado` fixo em `true` enquanto o
        // campo nao existe nas configuracoes. Ver a nota do commit.
        {
            let abertos = crate::microfone::abertos_agora();
            let agora = time::OffsetDateTime::now_utc().unix_timestamp();

            let gravando = app.state::<crate::meeting::RecordingState>().gravando();
            let silenciados = app
                .state::<Monitor>()
                .silenced_now(agora)
                .into_iter()
                .map(|(processo, _)| processo)
                .collect();

            // Microfone fechou: a proxima abertura oferece de novo, e isso e
            // desejado — pode ser outra reuniao.
            if let Ok(mut observed) = app.state::<Monitor>().observed.lock() {
                let ainda_aberto = observed
                    .ja_ofereceu
                    .as_ref()
                    .map(|alvo| abertos.iter().any(|entrada| &entrada.processo == alvo))
                    .unwrap_or(false);
                if !ainda_aberto {
                    observed.ja_ofereceu = None;
                }
            }

            let contexto = mos_core::ContextoDaOferta {
                gravando,
                silenciados,
                ligado: settings.meeting_detection_enabled,
                espera_segundos: ESPERA_DE_OFERTA,
            };
            if let mos_core::DecisaoDeOferta::Oferecer(processo) =
                mos_core::decidir_oferta(&abertos, &contexto)
            {
                oferecer(&app, &processo);
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
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    // "Nao lembrar hoje" silencia o LEMBRETE, e nunca o registro: o evento ja
    // foi gravado acima. Quem pediu silencio pediu para nao ser interrompido,
    // nao para o dia sumir da Linha do Tempo.
    if !useful || !allowed || app.state::<Monitor>().silenced(process, now) {
        return;
    }
    if !cooldown_passed(app, process) {
        return;
    }

    let reminder = PendingReminder {
        process_name: process.to_string(),
        display_name: display.clone(),
        opened,
        has_active_timer: running,
    };
    if let Ok(mut observed) = app.state::<Monitor>().observed.lock() {
        observed.pending = Some(reminder.clone());
    }

    show_reminder(app, &reminder);
    remind(
        app,
        &format!(
            "{display} {}",
            if opened { "foi aberto" } else { "foi fechado" }
        ),
        if opened {
            "Iniciar o cronometro? A janelinha do M/OS tem o botao."
        } else {
            "Encerrar o registro atual? A janelinha do M/OS tem o botao."
        },
    );
}

/// Traz a janelinha de lembrete para a frente, no canto inferior direito.
///
/// **Sem roubar o foco.** Quem esta desenhando esta com as maos no AutoCAD, e
/// uma janela que captura o teclado no meio de um comando de CAD nao e um
/// lembrete — e um acidente esperando para acontecer. Ela aparece por cima,
/// espera, e nao atrapalha.
fn show_reminder<R: Runtime>(app: &AppHandle<R>, reminder: &PendingReminder) {
    let Some(window) = app.get_webview_window("lembrete") else {
        return;
    };
    // A janela ja aberta so precisa do evento: reposicionar por baixo do dedo do
    // usuario seria pior que deixar onde esta.
    let already_visible = window.is_visible().unwrap_or(false);
    if !already_visible {
        if let Ok(Some(monitor)) = window.current_monitor() {
            let screen = monitor.size();
            let scale = monitor.scale_factor();
            if let Ok(size) = window.outer_size() {
                // Margem de 24pt do canto, convertida para fisico: a barra de
                // tarefas mora ali embaixo, e encostar nela esconde o botao.
                let margin = (24.0 * scale) as u32;
                let x = screen.width.saturating_sub(size.width + margin);
                let y = screen.height.saturating_sub(size.height + margin * 3);
                let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
            }
        }
        let _ = window.show();
        let _ = window.set_always_on_top(true);
    }
    let _ = window.emit("reminder", reminder);
}

/// Mostra a oferta de gravar, sem roubar o foco.
///
/// Uma janela que captura o teclado no instante em que alguem entra numa reuniao
/// e um acidente: a pessoa esta clicando em "entrar", e nao aqui.
///
/// So oferece UMA VEZ por abertura de microfone — `ja_ofereceu` guarda o alvo, e
/// o laco o limpa quando o processo some dos abertos.
fn oferecer<R: Runtime>(app: &AppHandle<R>, processo: &str) {
    {
        let monitor = app.state::<Monitor>();
        let Ok(mut observed) = monitor.observed.lock() else {
            return;
        };
        if observed.ja_ofereceu.as_deref() == Some(processo) {
            return;
        }
        observed.ja_ofereceu = Some(processo.to_string());
    }

    let Some(window) = app.get_webview_window("reuniao-detectada") else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        if let Ok(Some(tela)) = window.current_monitor() {
            let screen = tela.size();
            let scale = tela.scale_factor();
            if let Ok(size) = window.outer_size() {
                // Mesma margem do lembrete: a barra de tarefas mora ali embaixo,
                // e encostar nela esconde o botao.
                let margin = (24.0 * scale) as u32;
                let x = screen.width.saturating_sub(size.width + margin);
                let y = screen.height.saturating_sub(size.height + margin * 3);
                let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
            }
        }
        let _ = window.show();
        let _ = window.set_always_on_top(true);
    }
    let _ = window.emit(
        "reuniao-detectada",
        serde_json::json!({ "processo": processo, "nome": nome_amigavel(processo) }),
    );
}

/// O nome que a pessoa le.
///
/// Sem lista de reunioes conhecidas: o que se observou foi o nome do
/// executavel, e inventar "Google Meet" a partir de `chrome.exe` seria afirmar o
/// que nao se observou — exatamente o que a ADR-047 recusa ao nao ler titulo.
fn nome_amigavel(processo: &str) -> String {
    processo.strip_suffix(".exe").unwrap_or(processo).to_string()
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

/// Fecha a janelinha da oferta de gravar.
#[tauri::command]
pub fn fechar_reuniao_detectada<R: Runtime>(app: AppHandle<R>) {
    if let Some(window) = app.get_webview_window("reuniao-detectada") {
        // `hide`, e nao `close`: a janela sobrevive entre ofertas, como a do
        // lembrete. Recriar custa o tempo em que ela precisa aparecer.
        let _ = window.hide();
    }
}

/// Silencia a DETECCAO para um processo, pelo mesmo caminho do lembrete.
///
/// Silencia o aviso, e nunca a observacao — mesmo criterio do "nao lembrar
/// hoje": quem pediu silencio pediu para nao ser interrompido.
#[tauri::command]
pub fn silenciar_deteccao<R: Runtime>(app: AppHandle<R>, processo: String) {
    let ate = time::OffsetDateTime::now_utc().unix_timestamp() + 60 * 60 * 12;
    app.state::<Monitor>().suppress(&processo, ate);
    if let Some(window) = app.get_webview_window("reuniao-detectada") {
        let _ = window.hide();
    }
}

/// O que a janelinha do lembrete deve mostrar. Ela pergunta ao abrir, porque
/// pode ter nascido depois de o evento ter sido emitido.
#[tauri::command]
pub fn reminder_pending<R: Runtime>(app: AppHandle<R>) -> Option<PendingReminder> {
    app.state::<Monitor>().pending()
}

/// Fecha a janelinha sem decidir nada. "Agora nao" e uma resposta legitima.
#[tauri::command]
pub fn reminder_dismiss<R: Runtime>(app: AppHandle<R>) {
    app.state::<Monitor>().clear_pending();
    if let Some(window) = app.get_webview_window("lembrete") {
        let _ = window.hide();
    }
}

/// Os programas silenciados agora, para a tela poder mostrar e desfazer.
///
/// O silencio vive em MEMORIA e nao no banco, de proposito: "hoje" e uma
/// decisao do dia, e reiniciar o M/OS ja e o gesto mais natural de "quero tudo
/// de volta". Persisti-lo criaria um estado que sobrevive sem que ninguem
/// lembre de o ter criado.
#[tauri::command]
pub fn reminder_silenced<R: Runtime>(app: AppHandle<R>) -> Vec<SilencedApp> {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    app.state::<Monitor>()
        .silenced_now(now)
        .into_iter()
        .map(|(process_name, until)| SilencedApp {
            process_name,
            minutes_left: ((until - now) / 60).max(0),
        })
        .collect()
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SilencedApp {
    pub process_name: String,
    /// Quanto falta, em minutos. A tela nao precisa do instante — precisa dizer
    /// "volta em 3h", que e o que responde "ate quando isso vale?".
    pub minutes_left: i64,
}

#[tauri::command]
pub fn reminder_unsilence<R: Runtime>(app: AppHandle<R>, process_name: String) {
    app.state::<Monitor>().unsilence(&process_name);
}

/// Silencia um programa ate o instante dado.
///
/// O instante vem da INTERFACE, em ISO: "hoje" acaba a meia-noite LOCAL, e aqui
/// so existe UTC. Calcular o fim do dia sem saber o fuso daria meia-noite em
/// Londres, que no Brasil e nove da noite — o lembrete voltaria a incomodar
/// justamente na hora extra.
#[tauri::command]
pub fn reminder_suppress<R: Runtime>(
    app: AppHandle<R>,
    process_name: String,
    until: String,
) -> Result<(), mos_core::CoreError> {
    let moment = mos_core::parse_moment(&until)?;
    app.state::<Monitor>()
        .suppress(&process_name, moment.unix_timestamp());
    if let Some(window) = app.get_webview_window("lembrete") {
        let _ = window.hide();
    }
    Ok(())
}
