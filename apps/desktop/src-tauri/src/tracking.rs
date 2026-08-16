//! Comandos do rastreio de tempo (ADR-032).
//!
//! A importacao do CronoCAD e um caminho de mao unica que roda uma vez. Ela
//! aparece na interface e nao num script porque quem precisa conferir se as
//! horas chegaram inteiras e o usuario, na tela dele — e porque um script exige
//! que ele saiba onde o banco mora.

use std::path::PathBuf;

use mos_core::{ActiveTimer, ActivityType, CoreError, ProjectId, StartTimer, TimeEntry, Totals};
use mos_storage_sqlite::ImportReport;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::AppState;

/// Onde o CronoCAD guarda o banco, na instalacao padrao.
///
/// Sugerido e nao imposto: o usuario escolhe o arquivo no dialogo. Serve para
/// que ele nao precise saber que `com.cronocad.app` existe.
#[tauri::command]
pub fn tracking_default_cronocad_path() -> Option<String> {
    let base = std::env::var("APPDATA").ok()?;
    let path = PathBuf::from(base)
        .join("com.cronocad.app")
        .join("cronocad.sqlite");
    path.exists().then(|| path.display().to_string())
}

/// Quando o CronoCAD foi importado, se foi.
///
/// A tela pergunta ao BANCO em vez de lembrar da sessao: fechar e reabrir o app
/// nao deveria reabilitar um botao que nao pode mais ser clicado.
#[tauri::command]
pub fn tracking_cronocad_imported_at<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<String>, CoreError> {
    app.state::<AppState>().storage.cronocad_imported_at()
}

/// Importa o banco do CronoCAD. Roda uma vez.
#[tauri::command]
pub async fn tracking_import_cronocad<R: Runtime>(
    app: AppHandle<R>,
    path: String,
) -> Result<ImportReport, CoreError> {
    let report = app
        .state::<AppState>()
        .storage
        .import_cronocad(std::path::Path::new(&path))?;
    // Projects e Tasks nasceram: a Home, o Kanban e a lista precisam refletir.
    let _ = app.emit("data-changed", "cronocad-import");
    Ok(report)
}

/// Totais por Project, com o arredondamento configurado ja aplicado.
#[tauri::command]
pub fn tracking_totals<R: Runtime>(
    app: AppHandle<R>,
) -> Result<std::collections::HashMap<String, Totals>, CoreError> {
    app.state::<AppState>().tracking.totals_by_project()
}

/// Lanca tempo que o cronometro nao contou.
///
/// A pergunta que guia o CronoCAD e "isso reduz a chance de esquecer de
/// registrar o trabalho?". Esquecer de INICIAR e tao comum quanto esquecer de
/// encerrar, e sem este caminho a hora esquecida simplesmente nao existe.
#[tauri::command]
pub fn tracking_record<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    started_at: String,
    duration_seconds: i64,
    description: String,
    activity_type: String,
    billable: bool,
) -> Result<TimeEntry, CoreError> {
    let state = app.state::<AppState>();
    let project = ProjectId::parse(&project_id)?;
    // A taxa vem do Project AGORA, e vira snapshot: e a melhor aproximacao
    // disponivel para uma hora lancada depois.
    let rate = state
        .tracking
        .project_tracking()?
        .into_iter()
        .find(|entry| entry.project_id == project)
        .map(|entry| entry.hourly_rate_cents)
        .unwrap_or(0);

    let entry = state.tracking.record(mos_core::NewTimeEntry {
        project_id: project,
        started_at: mos_core::parse_moment(&started_at)?,
        ended_at: None,
        duration_seconds,
        idle_seconds: 0,
        description,
        activity_type: ActivityType::parse(&activity_type)?,
        billable,
        hourly_rate_snapshot_cents: rate,
        // `Manual` e nao `Timer`: hora lancada a mao e estimativa, e faturar
        // estimativa como medicao sem dizer seria mentir por omissao.
        source: mos_core::EntrySource::Manual,
    })?;
    let _ = app.emit("data-changed", "tracking");
    Ok(entry)
}

/// Corrige uma sessao ja gravada.
///
/// Recebe a edicao como ESTRUTURA e nao como sete parametros soltos: o tipo ja
/// existe no dominio, e repetir a lista de campos aqui criaria um segundo lugar
/// para esquecer de acrescentar um.
#[tauri::command]
pub fn tracking_edit<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    edit: mos_core::TimeEntryEdit,
) -> Result<TimeEntry, CoreError> {
    let entry = app
        .state::<AppState>()
        .tracking
        .edit(mos_core::TimeEntryId::parse(&id)?, edit)?;
    let _ = app.emit("data-changed", "tracking");
    Ok(entry)
}

#[tauri::command]
pub fn tracking_trash<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), CoreError> {
    app.state::<AppState>()
        .tracking
        .trash(mos_core::TimeEntryId::parse(&id)?)?;
    let _ = app.emit("data-changed", "tracking");
    Ok(())
}

/// O inverso de `tracking_trash`, para o recibo poder oferecer desfazer.
#[tauri::command]
pub fn tracking_restore<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), CoreError> {
    app.state::<AppState>()
        .tracking
        .restore(mos_core::TimeEntryId::parse(&id)?)?;
    let _ = app.emit("data-changed", "tracking");
    Ok(())
}

/// O cronometro em curso, se houver.
///
/// A tela pergunta e recebe `accumulated_seconds` mais `last_resumed_at`, e nao
/// um numero de segundos ja pronto: assim ela pode desenhar o relogio correndo
/// sozinha, sem que o backend precise emitir um evento por segundo.
#[tauri::command]
pub fn timer_current<R: Runtime>(app: AppHandle<R>) -> Result<Option<ActiveTimer>, CoreError> {
    app.state::<AppState>().tracking.active_timer()
}

#[tauri::command]
pub fn timer_start<R: Runtime>(
    app: AppHandle<R>,
    project_id: String,
    description: String,
    activity_type: String,
) -> Result<ActiveTimer, CoreError> {
    let timer = app.state::<AppState>().tracking.start_timer(StartTimer {
        project_id: ProjectId::parse(&project_id)?,
        description,
        activity_type: ActivityType::parse(&activity_type)?,
    })?;
    let _ = app.emit("timer-changed", "started");
    Ok(timer)
}

#[tauri::command]
pub fn timer_set_running<R: Runtime>(
    app: AppHandle<R>,
    running: bool,
) -> Result<ActiveTimer, CoreError> {
    let timer = app
        .state::<AppState>()
        .tracking
        .set_timer_running(running)?;
    let _ = app.emit("timer-changed", if running { "resumed" } else { "paused" });
    Ok(timer)
}

/// Encerra e devolve a sessao gravada.
#[tauri::command]
pub fn timer_stop<R: Runtime>(app: AppHandle<R>) -> Result<TimeEntry, CoreError> {
    let entry = app.state::<AppState>().tracking.stop_timer()?;
    let _ = app.emit("timer-changed", "stopped");
    // A sessao nasceu: quem mostra horas por Project precisa reler.
    let _ = app.emit("data-changed", "timer");
    Ok(entry)
}

/// Joga fora o cronometro sem gravar sessao.
///
/// A confirmacao e da interface, e nao daqui: e a unica operacao do rastreio
/// que descarta tempo, e um comando que so recusa sem explicar deixaria o
/// usuario sem entender por que o botao nao fez nada.
#[tauri::command]
pub fn timer_discard<R: Runtime>(app: AppHandle<R>) -> Result<(), CoreError> {
    app.state::<AppState>().tracking.discard_timer()?;
    let _ = app.emit("timer-changed", "discarded");
    Ok(())
}

#[tauri::command]
pub fn tracking_clients<R: Runtime>(
    app: AppHandle<R>,
    include_archived: bool,
) -> Result<Vec<mos_core::Client>, CoreError> {
    app.state::<AppState>().tracking.clients(include_archived)
}

/// Cria quando `id` vem vazio, atualiza quando vem preenchido.
///
/// Um comando so porque o formulario e o mesmo: separar em dois obrigaria a
/// tela a saber qual chamar, e ela ja sabe se esta editando pelo id que tem.
#[tauri::command]
pub fn tracking_save_client<R: Runtime>(
    app: AppHandle<R>,
    id: Option<String>,
    input: mos_core::ClientInput,
) -> Result<mos_core::Client, CoreError> {
    let state = app.state::<AppState>();
    let client = match id.as_deref().filter(|value| !value.is_empty()) {
        Some(id) => state.tracking.update_client(id, input)?,
        None => state.tracking.create_client(input)?,
    };
    let _ = app.emit("data-changed", "clients");
    Ok(client)
}

#[tauri::command]
pub fn tracking_set_client_archived<R: Runtime>(
    app: AppHandle<R>,
    id: String,
    archived: bool,
) -> Result<mos_core::Client, CoreError> {
    let client = app
        .state::<AppState>()
        .tracking
        .set_client_archived(&id, archived)?;
    let _ = app.emit("data-changed", "clients");
    Ok(client)
}

#[tauri::command]
pub fn monitoring_apps<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Vec<mos_core::MonitoredApp>, CoreError> {
    app.state::<AppState>().monitoring.apps()
}

#[tauri::command]
pub fn monitoring_save_app<R: Runtime>(
    app: AppHandle<R>,
    entry: mos_core::MonitoredApp,
) -> Result<mos_core::MonitoredApp, CoreError> {
    let saved = app.state::<AppState>().monitoring.save_app(entry)?;
    let _ = app.emit("data-changed", "monitoring");
    Ok(saved)
}

#[tauri::command]
pub fn monitoring_delete_app<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), CoreError> {
    app.state::<AppState>().monitoring.delete_app(&id)?;
    let _ = app.emit("data-changed", "monitoring");
    Ok(())
}

/// Os eventos de uma janela, do mais antigo para o mais novo.
///
/// A janela e obrigatoria e nao tem padrao: a Linha do Tempo e sempre sobre um
/// periodo, e devolver "tudo" por omissao traria 321 eventos para desenhar um
/// dia.
#[tauri::command]
pub fn monitoring_events<R: Runtime>(
    app: AppHandle<R>,
    since: String,
    until: String,
) -> Result<Vec<mos_core::ActivityEvent>, CoreError> {
    app.state::<AppState>().monitoring.events(
        mos_core::parse_moment(&since)?,
        mos_core::parse_moment(&until)?,
    )
}

/// Marca o evento como resolvido, para o periodo nao ser reoferecido amanha.
#[tauri::command]
pub fn monitoring_mark_processed<R: Runtime>(
    app: AppHandle<R>,
    id: String,
) -> Result<(), CoreError> {
    app.state::<AppState>().monitoring.mark_processed(&id)
}

/// As sessoes, em tempo REAL — sem arredondar e sem descontar inatividade.
#[tauri::command]
pub fn tracking_entries<R: Runtime>(
    app: AppHandle<R>,
    project_id: Option<String>,
) -> Result<Vec<TimeEntry>, CoreError> {
    let project = project_id.as_deref().map(ProjectId::parse).transpose()?;
    app.state::<AppState>().tracking.entries(project)
}
