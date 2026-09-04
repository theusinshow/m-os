use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use mos_core::{
    AppCatalogEntry, AppLaunchKind, AppService, AttentionService, BackupInspection, BackupReceipt,
    Capture, CaptureService, ConversationService, CoreError, CreateAppInput, CreateCaptureInput,
    CreateProjectInput, CreateResourceInput, CreateTaskInput, CreateWorkspaceInput, DailyService,
    DataService, FunctionDefinition, HiddenWidget, MeetingService, MemoryService,
    MonitoringService, Project, RegisteredApp, Resource, ResourceWorkspace, SearchItem, Task,
    TaskState, TrackingService, UpdateAppInput, UpdateProjectInput, UpdateResourceInput,
    UpdateTaskInput, UpdateWorkspaceInput, VoiceService, WorkService, Workspace,
};
use mos_storage_sqlite::{SqliteStorage, StorageHealth};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Listener, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

mod academic;
mod academic_sync;
mod attention;
mod atualizacao;
mod calendar;
mod daily;
mod diagnostico;
mod finance;
mod hermes;
mod ingest;
mod jarvis;
mod meeting;
mod microfone;
mod monitor;
mod pdf;
mod stale;
mod surface;
mod sync;
mod tracking;
mod univirtus;
mod usage;
mod voice;

const DEFAULT_CAPTURE_SHORTCUT: &str = "Ctrl+Shift+Space";

/// O atalho da voz, SEGURADO enquanto se fala.
///
/// **Este numero foi medido, e nao escolhido.** A primeira escolha foi
/// `Ctrl+Alt+Space`, para ficar na familia do `Ctrl+Shift+Space` da captura. Ao
/// rodar o app, ele nao registrou: `RegisterHotKey` devolveu 1409
/// (`ERROR_HOTKEY_ALREADY_REGISTERED`) — ha outro programa nesta maquina que ja
/// o tomou. Um padrao que nao registra e uma feature que nao existe, e nada na
/// tela principal contaria isso.
///
/// `Ctrl+Alt+G` — G de gravar — foi sondado livre pela mesma API, junto de
/// `Ctrl+Alt+Q`, `Ctrl+Alt+Z` e `Ctrl+Shift+G`. As recusas:
///
/// - `Ctrl+Alt+Space` e `Ctrl+Alt+M`: ja tomados nesta maquina;
/// - `Ctrl+Shift+V` e `Ctrl+Alt+V`: roubariam "colar sem formatacao" e "colar
///   especial" de TODOS os programas — um atalho global e global;
/// - `Alt+Space`: e o menu de janela do Windows;
/// - `Ctrl+Shift+Alt+Space`: livre, mas tres modificadores para SEGURAR sao
///   ergonomia ruim justamente no gesto que precisa ser rapido.
const DEFAULT_VOICE_SHORTCUT: &str = "Ctrl+Alt+G";

pub(crate) struct AppState {
    captures: CaptureService,
    work: WorkService,
    apps: AppService,
    memory: MemoryService,
    conversations: ConversationService,
    tracking: TrackingService,
    meetings: MeetingService,
    voice: VoiceService,
    monitoring: MonitoringService,
    data: DataService,
    attention: AttentionService,
    /// A camada de intencao sobre o dia. Ver `docs/DAILY-SESSION.md`.
    daily: DailyService,
    clock: Arc<dyn mos_core::Clock>,
    storage: Arc<SqliteStorage>,
    shortcut_status: Mutex<String>,
    active_shortcut: Mutex<Option<String>>,
    voice_shortcut_status: Mutex<String>,
    /// O atalho de voz registrado agora.
    ///
    /// O plugin entrega UM handler para todos os atalhos, entao ele precisa
    /// saber qual deles disparou. Guardar a string registrada e o que permite
    /// distinguir "abrir a captura" de "segurar para falar" — sem isto, os dois
    /// fariam a mesma coisa.
    active_voice_shortcut: Mutex<Option<String>>,
    snapshot_status: Arc<Mutex<String>>,
    settings_path: PathBuf,
}

impl AppState {
    /// Os servicos que a camada de acao do agente usa, num tipo so.
    ///
    /// Existe para o executor do Hermes NAO precisar de `AppHandle`: ele pedia
    /// os servicos via `app.state::<AppState>()`, e isso amarrava a logica de
    /// dominio a uma janela do Tauri — a superficie de bolso nao tinha como
    /// reaproveita-la sem arrastar o Tauri junto.
    ///
    /// Sao clones baratos: cada servico guarda `Arc`, e nada e duplicado.
    ///
    /// # Por que ele ainda nao tem chamador
    ///
    /// Porque a metade que falta nao foi feita: o `run_action` do Hermes ainda
    /// depende de `app.emit`, de `attention::poke` e do `daily.rs`, que sao do
    /// desktop. Este metodo e a parte do desacoplamento que JA foi medida e
    /// resolvida — era o maior dos acoplamentos, e o unico espalhado por todo o
    /// `jarvis.rs`. O caminho que falta esta escrito no `apps/mos-web/README.md`.
    ///
    /// Apagar por estar sem uso desfaria trabalho concluido para refaze-lo
    /// depois; o `allow` diz que a ausencia de chamador e esperada, e nao um
    /// esquecimento.
    #[allow(dead_code)]
    pub(crate) fn servicos(&self) -> mos_core::Servicos {
        mos_core::Servicos {
            captures: self.captures.clone(),
            work: self.work.clone(),
            memory: self.memory.clone(),
            conversations: self.conversations.clone(),
            tracking: self.tracking.clone(),
            meetings: self.meetings.clone(),
            attention: self.attention.clone(),
            daily: self.daily.clone(),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserSettings {
    capture_shortcut: String,
    /// O atalho da voz. Vazio significa o padrao.
    #[serde(default)]
    voice_shortcut: String,
    /// Preferencia NOSSA, e nao do sistema.
    ///
    /// Diferente de "iniciar com o Windows", que vive no registro e e lido
    /// de la (ADR-043), esta o Windows nao conhece: ele so sabe iniciar o
    /// programa, nao com que cara. Por isso ela pode morar aqui sem criar a
    /// segunda fonte de verdade que aquela ADR existe para evitar.
    #[serde(default)]
    start_minimized: bool,
    /// A faixa de uso desligada de vez, pelo tray.
    ///
    /// `oculta` e nao `visivel` porque `#[serde(default)]` de um booleano e
    /// `false`, e o padrao que se quer e a faixa APARECENDO. Um campo chamado
    /// `visivel` precisaria de um `default` proprio so para nao nascer
    /// desligado no primeiro `settings.json` que ainda nao o conhece.
    #[serde(default)]
    faixa_oculta: bool,
    /// A faixa recolhida na lingueta. Mesma logica de nome do campo acima.
    #[serde(default)]
    faixa_recolhida: bool,
    /// Provedores de IA ALEM do Claude Code, cada um com o comando que sabe
    /// dizer a propria cota. Ver [`usage::FonteExterna`] e a ADR-063.
    ///
    /// Editado a mao neste arquivo, e sem tela de Settings. Nao e esquecimento:
    /// quem tem um segundo agente de codigo com um comando de cota em JSON e
    /// alguem que edita JSON, e uma tela para isso custaria mais do que ela
    /// valeria hoje. Quando houver um segundo interessado, ela nasce.
    #[serde(default)]
    pub(crate) faixa_fontes: Vec<usage::FonteExterna>,
    /// Caminhos do transcritor local.
    ///
    /// Preferencia NOSSA, e nao do sistema, entao ela mora aqui — diferente de
    /// "iniciar com o Windows", que vive no registro e e lida de la (ADR-043).
    /// Os caminhos ficam em Settings em vez de embutidos porque a decisao D-7
    /// (qual modelo, CPU ou Vulkan) e do usuario: trocar o binario nao pode
    /// exigir recompilar o M/OS.
    #[serde(default)]
    whisper: mos_transcribe::WhisperConfig,
    /// Quando o envio da transcricao ao Hermes foi autorizado, em RFC3339.
    ///
    /// Vazio significa nunca. E um INSTANTE e nao um booleano porque a ADR-027
    /// pede que "o que saiu e quando" tenha resposta — e a data em que a pessoa
    /// autorizou faz parte dessa resposta.
    #[serde(default)]
    analysis_consent_at: String,
    /// Onde o hub de sincronizacao esta. Vazio significa "nao configurado", e
    /// e o estado normal ate alguem ligar isto — o M/OS funciona inteiro sem.
    ///
    /// O ENDERECO mora aqui; o SEGREDO mora no Credential Manager. Guardar os
    /// dois juntos poria um token em texto claro num arquivo que o backup
    /// carrega e o export copia.
    #[serde(default)]
    sync_endpoint: String,
    /// O dia em que o resumo da sincronizacao foi mostrado pela ultima vez.
    ///
    /// Data civil (`YYYY-MM-DD`), e nao instante: "primeira abertura do dia" e a
    /// mesma regua da Daily Session, e um segundo conceito de dia dentro do
    /// mesmo app seria uma divergencia esperando acontecer.
    ///
    /// Mora AQUI e nao no React porque, como estado da tela, sair da Home e
    /// voltar traria a faixa de novo no mesmo dia.
    #[serde(default)]
    pub(crate) sync_ultimo_resumo_em: String,
    /// O que se sabe sobre atualizacao, gravado em vez de guardado na tela.
    ///
    /// O painel respondia "estou atualizado?" apenas nos segundos seguintes ao
    /// clique: o estado morava em `useState`, e sair de Settings apagava a prova
    /// de que a verificacao tinha acontecido. Pior, "nao ha versao nova" e "nao
    /// consegui verificar" ficavam com a mesma cara — nenhuma —, que e de onde
    /// sai a impressao de que a atualizacao "as vezes nao funciona".
    ///
    /// Ficam aqui, e nao no banco, porque sao fatos DESTE aparelho: a versao
    /// instalada nao sincroniza, e mandar isto para o hub faria o celular achar
    /// que tambem esta desatualizado. Ver `atualizacao.rs`.
    #[serde(default)]
    pub(crate) atualizacao_verificada_em: String,
    #[serde(default)]
    pub(crate) atualizacao_disponivel: String,
    #[serde(default)]
    pub(crate) atualizacao_publicada_em: String,
    #[serde(default)]
    pub(crate) atualizacao_falha: String,
    #[serde(default)]
    pub(crate) atualizacao_falha_em: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppStatus {
    inbox_count: usize,
    project_count: usize,
    task_count: usize,
    app_count: usize,
    resource_count: usize,
    workspace_count: usize,
    shortcut: String,
    voice_shortcut: String,
    snapshot: String,
    storage: StorageHealth,
}

#[tauri::command]
fn create_capture(
    input: CreateCaptureInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.create(input)?;
    let _ = app.emit_to("main", "capture-changed", capture.id.to_string());
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(capture)
}

#[tauri::command]
fn get_capture(id: &str, state: tauri::State<'_, AppState>) -> Result<Capture, CoreError> {
    state.captures.get(id)
}

#[tauri::command]
fn list_recent(state: tauri::State<'_, AppState>) -> Result<Vec<Capture>, CoreError> {
    state.captures.recent(8)
}

#[tauri::command]
fn list_inbox(state: tauri::State<'_, AppState>) -> Result<Vec<Capture>, CoreError> {
    state.captures.inbox(200)
}

#[tauri::command]
fn list_archived(state: tauri::State<'_, AppState>) -> Result<Vec<Capture>, CoreError> {
    state.captures.archived(200)
}

#[tauri::command]
fn list_trashed(state: tauri::State<'_, AppState>) -> Result<Vec<Capture>, CoreError> {
    state.captures.trashed(200)
}

#[tauri::command]
fn search_captures(
    query: &str,
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Capture>, CoreError> {
    state.captures.search(query, include_archived, 50)
}

#[tauri::command]
fn search_all(
    query: &str,
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchItem>, CoreError> {
    let mut items = state.work.search(query, include_archived)?;
    items.extend(
        state
            .apps
            .search(query, include_archived, 100)?
            .into_iter()
            .map(|app| SearchItem::App { app }),
    );
    // Os objetivos do dia entram DEPOIS dos Apps, e nao antes das Tasks: a
    // busca do Command procura coisa para abrir, e o dia e contexto. Ele
    // responde "o que eu estava fazendo terca?", que e uma pergunta que se faz
    // depois de nao achar o que se procurava.
    items.extend(
        state
            .daily
            .search(query, 20)?
            .into_iter()
            .map(|(objective, day)| SearchItem::DailyObjective { objective, day }),
    );
    // A faculdade entra por ultimo, junto do dia: quem digita no Command procura
    // coisa para abrir, e disciplina e contexto. Uma prova achada depois da Task
    // de exercicios e a ordem certa de leitura.
    items.extend(mos_core::AcademicRepository::search_academic(
        state.storage.as_ref(),
        mos_core::SearchRequest {
            query: query.to_owned(),
            include_archived,
            limit: 20,
        },
    )?);
    items.truncate(100);
    Ok(items)
}

#[tauri::command]
fn list_functions() -> Vec<FunctionDefinition> {
    mos_core::function_registry()
}

#[tauri::command]
fn search_functions(query: &str) -> Vec<FunctionDefinition> {
    mos_core::search_functions(query, 50)
}

#[tauri::command]
fn mark_capture_processed(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.mark_processed(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn move_capture_to_inbox(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.move_to_inbox(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn archive_capture(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.archive(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn trash_capture(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.trash(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn restore_capture(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Capture, CoreError> {
    let capture = state.captures.restore(id)?;
    notify_capture_changed(&app, id);
    Ok(capture)
}

#[tauri::command]
fn rebuild_search(state: tauri::State<'_, AppState>) -> Result<usize, CoreError> {
    Ok(state.work.rebuild_search()?
        + state.apps.rebuild_search()?
        + state.memory.rebuild_search()?)
}

#[tauri::command]
fn create_resource(
    input: CreateResourceInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Resource, CoreError> {
    let resource = state.memory.create_resource(input)?;
    notify_data_changed(&app, "resource-created");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(resource)
}

#[tauri::command]
fn update_resource(
    input: UpdateResourceInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Resource, CoreError> {
    let resource = state.memory.update_resource(input)?;
    notify_data_changed(&app, "resource-updated");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(resource)
}

#[tauri::command]
fn get_resource(id: &str, state: tauri::State<'_, AppState>) -> Result<Resource, CoreError> {
    state.memory.resource(id)
}

#[tauri::command]
fn list_resources(
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Resource>, CoreError> {
    state.memory.resources(include_archived)
}

#[tauri::command]
fn list_trashed_resources(state: tauri::State<'_, AppState>) -> Result<Vec<Resource>, CoreError> {
    state.memory.trashed_resources()
}

#[tauri::command]
fn search_resources(
    query: &str,
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Resource>, CoreError> {
    state.memory.search(query, include_archived, 50)
}

#[tauri::command]
fn list_resource_workspaces(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ResourceWorkspace>, CoreError> {
    state.memory.resource_workspaces()
}

#[tauri::command]
fn set_resource_workspace(
    resource_id: &str,
    workspace_id: &str,
    linked: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state
        .memory
        .set_resource_workspace(resource_id, workspace_id, linked)?;
    notify_data_changed(&app, "resource-workspace");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn set_resource_archived(
    id: &str,
    archived: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Resource, CoreError> {
    let lifecycle = if archived {
        mos_core::LifecycleState::Archived
    } else {
        mos_core::LifecycleState::Active
    };
    let resource = state.memory.set_resource_lifecycle(id, lifecycle)?;
    notify_data_changed(&app, "resource-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(resource)
}

#[tauri::command]
fn trash_resource(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Resource, CoreError> {
    let resource = state
        .memory
        .set_resource_lifecycle(id, mos_core::LifecycleState::Trashed)?;
    notify_data_changed(&app, "resource-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(resource)
}

#[tauri::command]
fn restore_resource(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Resource, CoreError> {
    let resource = state
        .memory
        .set_resource_lifecycle(id, mos_core::LifecycleState::Active)?;
    notify_data_changed(&app, "resource-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(resource)
}

/// Abre um link que veio dentro de uma resposta do Hermes.
///
/// Passa pelo mesmo caminho nativo dos Resources, e nao pelo WebView: navegar
/// dentro da janela transformaria o M/OS num navegador com privilegio de
/// aplicacao. `open_external_target` ja recusa o que nao for http(s), o que
/// importa aqui mais do que em qualquer outro chamador — este alvo foi escrito
/// por um modelo, e `ShellExecuteW` abriria feliz um caminho local.
#[tauri::command]
fn open_external_url(url: String) -> Result<(), CoreError> {
    open_external_target(AppLaunchKind::Url, url.trim())
}

#[tauri::command]
fn open_resource(id: &str, state: tauri::State<'_, AppState>) -> Result<(), CoreError> {
    let resource = state.memory.resource(id)?;
    if resource.lifecycle_state != mos_core::LifecycleState::Active {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidTransition,
            "Somente um Resource ativo pode ser aberto.",
            false,
        ));
    }
    open_external_target(AppLaunchKind::Url, &resource.url)
}

#[tauri::command]
fn create_workspace(
    input: CreateWorkspaceInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Workspace, CoreError> {
    let workspace = state.work.create_workspace(input)?;
    notify_data_changed(&app, "workspace-created");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(workspace)
}

#[tauri::command]
fn update_workspace(
    input: UpdateWorkspaceInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Workspace, CoreError> {
    let workspace = state.work.update_workspace(input)?;
    notify_data_changed(&app, "workspace-updated");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(workspace)
}

#[tauri::command]
fn get_workspace(id: &str, state: tauri::State<'_, AppState>) -> Result<Workspace, CoreError> {
    state.work.workspace(id)
}

#[tauri::command]
fn list_workspaces(
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Workspace>, CoreError> {
    state.work.workspaces(include_archived)
}

#[tauri::command]
fn set_workspace_archived(
    id: &str,
    archived: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Workspace, CoreError> {
    let workspace = state.work.set_workspace_archived(id, archived)?;
    notify_data_changed(&app, "workspace-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(workspace)
}

#[tauri::command]
fn list_workspace_projects(
    id: &str,
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Project>, CoreError> {
    state.work.workspace_projects(id, include_archived)
}

#[tauri::command]
fn list_workspace_apps(
    id: &str,
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RegisteredApp>, CoreError> {
    state.work.workspace_apps(id, include_archived)
}

#[tauri::command]
fn list_project_workspaces(
    id: &str,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Workspace>, CoreError> {
    state.work.project_workspaces(id)
}

#[tauri::command]
fn list_app_workspaces(
    id: &str,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Workspace>, CoreError> {
    state.work.app_workspaces(id)
}

#[tauri::command]
fn set_project_workspace(
    project_id: &str,
    workspace_id: &str,
    linked: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state
        .work
        .set_project_workspace(project_id, workspace_id, linked)?;
    notify_data_changed(&app, "project-workspace");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn set_app_workspace(
    app_id: &str,
    workspace_id: &str,
    linked: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state.work.set_app_workspace(app_id, workspace_id, linked)?;
    notify_data_changed(&app, "app-workspace");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

/* Exclusao definitiva. Seis comandos com a mesma forma, e a mesma regra imposta
no repositorio: so apaga o que ja esta arquivado ou na lixeira.

Nao existe undo aqui, e por isso a confirmacao mora na interface. Depois de
apagar, o snapshot agendado nao serve de socorro: ele e posterior ao
apagamento. O socorro e o backup anterior, em DADOS E PORTABILIDADE. */
#[tauri::command]
fn delete_capture(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state.captures.delete_capture(id)?;
    notify_data_changed(&app, "capture-deleted");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn delete_task(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state.work.delete_task(id)?;
    notify_data_changed(&app, "task-deleted");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn delete_project(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state.work.delete_project(id)?;
    notify_data_changed(&app, "project-deleted");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn delete_workspace(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state.work.delete_workspace(id)?;
    notify_data_changed(&app, "workspace-deleted");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn delete_registered_app(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state.apps.delete_app(id)?;
    notify_data_changed(&app, "app-deleted");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn delete_resource(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state.memory.delete_resource(id)?;
    notify_data_changed(&app, "resource-deleted");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn list_hidden_widgets(state: tauri::State<'_, AppState>) -> Result<Vec<HiddenWidget>, CoreError> {
    state.work.hidden_widgets()
}

/// A interface fala em visivel; a tabela guarda o oculto. A inversao acontece
/// aqui, num lugar so — espalha-la pelos componentes seria garantir que um dia
/// dois deles discordem sobre o que a ausencia de linha significa.
#[tauri::command]
fn set_workspace_widget(
    workspace_id: Option<String>,
    widget_id: &str,
    visible: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state
        .work
        .set_widget_hidden(workspace_id.as_deref(), widget_id, !visible)?;
    notify_data_changed(&app, "workspace-widget");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(())
}

#[tauri::command]
fn create_project(
    input: CreateProjectInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Project, CoreError> {
    let project = state.work.create_project(input)?;
    notify_data_changed(&app, "project-created");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(project)
}

#[tauri::command]
fn update_project(
    input: UpdateProjectInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Project, CoreError> {
    let project = state.work.update_project(input)?;
    notify_data_changed(&app, "project-updated");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(project)
}

#[tauri::command]
fn get_project(id: &str, state: tauri::State<'_, AppState>) -> Result<Project, CoreError> {
    state.work.project(id)
}

#[tauri::command]
fn list_projects(
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Project>, CoreError> {
    state.work.projects(include_archived)
}

#[tauri::command]
fn set_project_archived(
    id: &str,
    archived: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Project, CoreError> {
    let project = state.work.set_project_archived(id, archived)?;
    notify_data_changed(&app, "project-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(project)
}

#[tauri::command]
fn create_task(
    input: CreateTaskInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Task, CoreError> {
    let task = state.work.create_task(input)?;
    notify_data_changed(&app, "task-created");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(task)
}

#[tauri::command]
fn update_task(
    input: UpdateTaskInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Task, CoreError> {
    let task = state.work.update_task(input)?;
    notify_data_changed(&app, "task-updated");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(task)
}

#[tauri::command]
fn get_task(id: &str, state: tauri::State<'_, AppState>) -> Result<Task, CoreError> {
    state.work.task(id)
}

#[tauri::command]
fn list_tasks(
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Task>, CoreError> {
    state.work.tasks(include_archived)
}

#[tauri::command]
fn set_task_state(
    id: &str,
    task_state: TaskState,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Task, CoreError> {
    let task = state.work.set_task_state(id, task_state)?;
    notify_data_changed(&app, "task-state");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(task)
}

#[tauri::command]
fn set_task_archived(
    id: &str,
    archived: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Task, CoreError> {
    let task = state.work.set_task_archived(id, archived)?;
    notify_data_changed(&app, "task-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(task)
}

#[tauri::command]
fn create_registered_app(
    input: CreateAppInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<RegisteredApp, CoreError> {
    let registered_app = state.apps.create_app(input)?;
    notify_data_changed(&app, "app-created");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(registered_app)
}

#[tauri::command]
fn list_app_catalog(state: tauri::State<'_, AppState>) -> Vec<AppCatalogEntry> {
    state.apps.catalog()
}

#[tauri::command]
fn register_app_catalog(
    ids: Vec<String>,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RegisteredApp>, CoreError> {
    let registered = state.apps.register_catalog(&ids)?;
    notify_data_changed(&app, "app-catalog-registered");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(registered)
}

#[tauri::command]
fn update_registered_app(
    input: UpdateAppInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<RegisteredApp, CoreError> {
    let registered_app = state.apps.update_app(input)?;
    notify_data_changed(&app, "app-updated");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(registered_app)
}

#[tauri::command]
fn get_registered_app(
    id: &str,
    state: tauri::State<'_, AppState>,
) -> Result<RegisteredApp, CoreError> {
    state.apps.app(id)
}

#[tauri::command]
fn list_registered_apps(
    include_archived: bool,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<RegisteredApp>, CoreError> {
    state.apps.apps(include_archived)
}

#[tauri::command]
fn set_registered_app_archived(
    id: &str,
    archived: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<RegisteredApp, CoreError> {
    let registered_app = state.apps.set_app_archived(id, archived)?;
    notify_data_changed(&app, "app-lifecycle");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(registered_app)
}

#[tauri::command]
fn mark_registered_app_opened(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<RegisteredApp, CoreError> {
    let registered_app = state.apps.mark_app_opened(id)?;
    notify_data_changed(&app, "app-opened");
    Ok(registered_app)
}

#[tauri::command]
fn open_registered_app(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<RegisteredApp, CoreError> {
    let registered_app = state.apps.app(id)?;
    if registered_app.lifecycle_state != mos_core::LifecycleState::Active {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidTransition,
            "App arquivado nao pode ser aberto.",
            false,
        ));
    }
    let launch_kind = registered_app.launch_kind.ok_or_else(|| {
        CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Este App nao possui alvo de abertura.",
            false,
        )
    })?;
    let launch_target = registered_app.launch_target.as_deref().ok_or_else(|| {
        CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Este App nao possui alvo de abertura.",
            false,
        )
    })?;
    open_external_target(launch_kind, launch_target)?;
    let opened = state.apps.mark_app_opened(id)?;
    notify_data_changed(&app, "app-opened");
    schedule_snapshot(&state.data, &state.snapshot_status, &app);
    Ok(opened)
}

#[tauri::command]
fn create_backup(
    path: &str,
    state: tauri::State<'_, AppState>,
) -> Result<BackupReceipt, CoreError> {
    state.data.create_backup(PathBuf::from(path).as_path())
}

#[tauri::command]
fn inspect_backup(
    path: &str,
    state: tauri::State<'_, AppState>,
) -> Result<BackupInspection, CoreError> {
    state.data.inspect_backup(PathBuf::from(path).as_path())
}

#[tauri::command]
fn restore_backup(
    path: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<BackupReceipt, CoreError> {
    let receipt = state.data.restore_backup(PathBuf::from(path).as_path())?;
    let _ = app.emit_to("main", "dataset-restored", ());
    Ok(receipt)
}

#[tauri::command]
fn export_json(path: &str, state: tauri::State<'_, AppState>) -> Result<BackupReceipt, CoreError> {
    state.data.export_json(PathBuf::from(path).as_path())
}

#[tauri::command]
fn get_app_status(state: tauri::State<'_, AppState>) -> Result<AppStatus, CoreError> {
    Ok(AppStatus {
        inbox_count: state.captures.inbox(200)?.len(),
        project_count: state.work.projects(false)?.len(),
        task_count: state.work.tasks(false)?.len(),
        app_count: state.apps.apps(false)?.len(),
        resource_count: state.memory.resources(false)?.len(),
        workspace_count: state.work.workspaces(false)?.len(),
        shortcut: state
            .shortcut_status
            .lock()
            .map_err(|error| lock_error(error.to_string()))?
            .clone(),
        voice_shortcut: state
            .voice_shortcut_status
            .lock()
            .map_err(|error| lock_error(error.to_string()))?
            .clone(),
        snapshot: state
            .snapshot_status
            .lock()
            .map_err(|error| lock_error(error.to_string()))?
            .clone(),
        storage: state.storage.health()?,
    })
}

#[tauri::command]
fn set_capture_shortcut(
    shortcut: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, CoreError> {
    let requested = shortcut.trim();
    if requested.is_empty() {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Informe um atalho.",
            false,
        ));
    }

    let mut active = state
        .active_shortcut
        .lock()
        .map_err(|error| lock_error(error.to_string()))?;
    if active.as_deref() == Some(requested) {
        return Ok(format!("Registrado: {requested}"));
    }
    let previous = active.take();
    if let Some(previous) = &previous {
        app.global_shortcut()
            .unregister(previous.as_str())
            .map_err(shortcut_error)?;
    }

    let result = match app.global_shortcut().register(requested) {
        Ok(()) => match persist_shortcut(&state.settings_path, requested) {
            Ok(()) => {
                *active = Some(requested.into());
                Ok(format!("Registrado: {requested}"))
            }
            Err(error) => {
                let _ = app.global_shortcut().unregister(requested);
                if let Some(previous) = &previous {
                    if app.global_shortcut().register(previous.as_str()).is_ok() {
                        *active = Some(previous.clone());
                    }
                }
                Err(error)
            }
        },
        Err(error) => {
            let mut message = format!("Nao foi possivel registrar {requested}: {error}");
            if let Some(previous) = previous {
                if app.global_shortcut().register(previous.as_str()).is_ok() {
                    *active = Some(previous.clone());
                    message.push_str(&format!(". {previous} continua ativo."));
                }
            }
            Err(CoreError::new(
                mos_core::ErrorCode::InvalidInput,
                message,
                true,
            ))
        }
    };
    let status = match &result {
        Ok(message) => message.clone(),
        Err(error) => error.message.clone(),
    };
    *state
        .shortcut_status
        .lock()
        .map_err(|error| lock_error(error.to_string()))? = status;
    result
}

/// Troca o atalho da voz, com rollback.
///
/// Mesma cerimonia do `set_capture_shortcut`, e a razao e a mesma: um atalho
/// que falha ao registrar nao pode deixar o usuario SEM atalho nenhum — o
/// anterior volta, e a mensagem diz que ele continua ativo.
#[tauri::command]
fn set_voice_shortcut(
    shortcut: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, CoreError> {
    let requested = shortcut.trim();
    if requested.is_empty() {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Informe um atalho.",
            false,
        ));
    }
    if state
        .active_shortcut
        .lock()
        .map_err(|error| lock_error(error.to_string()))?
        .as_deref()
        == Some(requested)
    {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Este atalho ja abre a Captura rapida.",
            false,
        ));
    }

    let mut active = state
        .active_voice_shortcut
        .lock()
        .map_err(|error| lock_error(error.to_string()))?;
    if active.as_deref() == Some(requested) {
        return Ok(format!("Registrado: {requested}"));
    }
    let previous = active.take();
    if let Some(previous) = &previous {
        app.global_shortcut()
            .unregister(previous.as_str())
            .map_err(shortcut_error)?;
    }

    let result = match app.global_shortcut().register(requested) {
        Ok(()) => match persist_voice_shortcut(&state.settings_path, requested) {
            Ok(()) => {
                *active = Some(requested.into());
                Ok(format!("Registrado: {requested}"))
            }
            Err(error) => {
                let _ = app.global_shortcut().unregister(requested);
                if let Some(previous) = &previous {
                    if app.global_shortcut().register(previous.as_str()).is_ok() {
                        *active = Some(previous.clone());
                    }
                }
                Err(error)
            }
        },
        Err(error) => {
            let mut message = format!("Nao foi possivel registrar {requested}: {error}");
            if let Some(previous) = previous {
                if app.global_shortcut().register(previous.as_str()).is_ok() {
                    *active = Some(previous.clone());
                    message.push_str(&format!(". {previous} continua ativo."));
                }
            }
            Err(CoreError::new(
                mos_core::ErrorCode::InvalidInput,
                message,
                true,
            ))
        }
    };
    let status = match &result {
        Ok(message) => message.clone(),
        Err(error) => error.message.clone(),
    };
    *state
        .voice_shortcut_status
        .lock()
        .map_err(|error| lock_error(error.to_string()))? = status;
    result
}

#[tauri::command]
fn show_quick_capture(app: AppHandle) {
    reveal_window(&app, "quick-capture");
}

#[tauri::command]
fn hide_quick_capture(app: AppHandle) {
    if let Some(window) = app.get_webview_window("quick-capture") {
        let _ = window.hide();
    }
}

fn notify_capture_changed(app: &AppHandle, id: &str) {
    let _ = app.emit_to("main", "capture-changed", id);
}

fn notify_data_changed(app: &AppHandle, reason: &str) {
    let _ = app.emit_to("main", "data-changed", reason);
}

/// De quanto em quanto tempo o M/OS confere se o dia ja tem snapshot.
///
/// Trinta minutos e barato: `ensure_daily_snapshot` so olha se o arquivo do dia
/// existe, e volta sem fazer nada quando existe. O que ele compra e o app que
/// fica aberto atravessando a meia-noite — com `startMinimized`, isso e a
/// regra, e nao a excecao.
const PULSO_DO_SNAPSHOT: Duration = Duration::from_secs(30 * 60);

/// Garante o snapshot do dia enquanto o app estiver de pe.
///
/// # Por que existe
///
/// Ate 2026-08-22 o snapshot diario so acontecia como efeito colateral de UMA
/// mutacao — criar Capture, Task, Project, Resource, App ou Workspace. Trinta e
/// duas chamadas espalhadas por trinta e dois comandos do `lib.rs`, e nenhuma
/// nos modulos que vieram depois: `daily.rs`, `meeting.rs`, `tracking.rs`,
/// `finance.rs`, `voice.rs`.
///
/// A consequencia apareceu no disco: os backups pararam em 2026-08-20. Nos dias
/// 21 e 22 o M/OS foi usado — Daily Session, Weekly Review, duas reunioes —, e
/// nada disso passa por um dos comandos que dispara. **Os dados mais novos eram
/// justamente os que ficavam sem copia.**
///
/// A correcao nao e acrescentar a chamada nos modulos que faltam: seria a mesma
/// armadilha esperando o proximo modulo. O backup do dia passa a depender do
/// app estar ABERTO, que e a unica condicao que todo uso tem em comum.
fn manter_snapshot_do_dia(
    data: &DataService,
    snapshot_status: &Arc<Mutex<String>>,
    app: &AppHandle,
) {
    let data = data.clone();
    let snapshot_status = snapshot_status.clone();
    let app = app.clone();
    std::thread::spawn(move || loop {
        let message = match data.ensure_daily_snapshot() {
            Ok(Some(_)) => "Snapshot diario criado.".to_owned(),
            Ok(None) => "Snapshot diario ja existe.".to_owned(),
            Err(error) => format!("Falha no snapshot diario: {}", error.message),
        };
        if let Ok(mut status) = snapshot_status.lock() {
            *status = message.clone();
        }
        let _ = app.emit_to("main", "snapshot-status-changed", message);
        std::thread::sleep(PULSO_DO_SNAPSHOT);
    });
}

fn schedule_snapshot(data: &DataService, snapshot_status: &Arc<Mutex<String>>, app: &AppHandle) {
    let data = data.clone();
    let snapshot_status = snapshot_status.clone();
    let app = app.clone();
    std::thread::spawn(move || {
        let message = match data.ensure_daily_snapshot() {
            Ok(Some(_)) => "Snapshot diario criado.".to_owned(),
            Ok(None) => "Snapshot diario ja existe.".to_owned(),
            Err(error) => format!("Falha no snapshot diario: {}", error.message),
        };
        if let Ok(mut status) = snapshot_status.lock() {
            *status = message.clone();
        }
        let _ = app.emit_to("main", "snapshot-status-changed", message);
    });
}

pub(crate) fn reveal_window<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        if label == "quick-capture" {
            if let (Ok(Some(monitor)), Ok(size)) = (window.current_monitor(), window.outer_size()) {
                let monitor_size = monitor.size();
                let monitor_position = monitor.position();
                let x =
                    monitor_position.x + (monitor_size.width.saturating_sub(size.width) / 2) as i32;
                let y = monitor_position.y
                    + ((monitor_size.height as f64 * 0.34) as i32 - size.height as i32 / 2);
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }
        }
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.emit("window-revealed", ());
        // Voltar ao primeiro plano e o gatilho que o fluxo casa > trabalho >
        // celular mais usa: sentar na mesa e trazer o M/OS para frente.
        sync::acordar(&window.app_handle().clone());
    }
}

/// O tray, e os dois menus que ele alterna.
///
/// **Dois menus, e nao itens escondidos.** `MenuItem` do Tauri 2 nao tem
/// `set_visible`, e um item permanente dizendo "Meeting Notes: parado" seria
/// ruido no unico lugar do sistema que o usuario olha de canto de olho. A troca
/// acontece so na TRANSICAO — o relogio de cada segundo e `set_text`, que nao
/// reconstroi nada.
pub struct TrayHandles {
    pub tray: tauri::tray::TrayIcon<tauri::Wry>,
    /// O item que carrega o relogio. Vive dentro de `live`.
    pub clock: tauri::menu::MenuItem<tauri::Wry>,
    /// Os DOIS itens da faixa: um item so pertence a um menu, e o tray troca de
    /// menu quando uma gravacao comeca. Marcar so um deixaria a marca errada
    /// metade do tempo.
    pub faixa: [CheckMenuItem<tauri::Wry>; 2],
    pub idle: Menu<tauri::Wry>,
    pub live: Menu<tauri::Wry>,
    /// Qual menu esta montado agora.
    pub live_shown: std::sync::atomic::AtomicBool,
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir M/OS", true, None::<&str>)?;
    let capture = MenuItem::with_id(app, "capture", "Captura rapida", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    // Marcado por padrao, e corrigido pela preferencia gravada assim que o laco
    // de uso comeca: aqui o `AppState` ainda nao existe, e com ele nao existe o
    // caminho do `settings.json`.
    let faixa = CheckMenuItem::with_id(app, "faixa", "Faixa de uso", true, true, None::<&str>)?;
    let idle = Menu::with_items(app, &[&open, &capture, &faixa, &quit])?;

    // Um item so pertence a um menu, entao o menu de gravacao tem instancias
    // proprias. Os ids sao os mesmos: quem trata o evento nao precisa saber
    // qual menu estava montado.
    let clock = MenuItem::with_id(app, "meeting_open", "Meeting Notes", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "meeting_stop", "Parar gravacao", true, None::<&str>)?;
    let open_live = MenuItem::with_id(app, "open", "Abrir M/OS", true, None::<&str>)?;
    let capture_live = MenuItem::with_id(app, "capture", "Captura rapida", true, None::<&str>)?;
    let quit_live = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let faixa_live =
        CheckMenuItem::with_id(app, "faixa", "Faixa de uso", true, true, None::<&str>)?;
    let live = Menu::with_items(
        app,
        &[
            &clock,
            &stop,
            &open_live,
            &capture_live,
            &faixa_live,
            &quit_live,
        ],
    )?;

    let mut tray = TrayIconBuilder::new()
        .tooltip("M/OS")
        .menu(&idle)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" | "meeting_open" => reveal_window(app, "main"),
            "capture" => reveal_window(app, "quick-capture"),
            // Parar pelo tray existe porque a janela pode estar escondida — e e
            // exatamente nessa situacao que a pessoa precisa parar sem procurar
            // o aplicativo atras do Meet.
            "meeting_stop" => meeting::stop_from_tray(app),
            "faixa" => usage::alternar_pela_bandeja(app),
            "quit" => app.exit(0),
            _ => {}
        });
    #[cfg(windows)]
    {
        tray = tray.icon(icone_da_bandeja());
    }
    #[cfg(not(windows))]
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    let tray = tray.build(app)?;
    app.manage(TrayHandles {
        tray,
        clock,
        faixa: [faixa, faixa_live],
        idle,
        live,
        live_shown: std::sync::atomic::AtomicBool::new(false),
    });
    Ok(())
}

fn shortcut_error(error: tauri_plugin_global_shortcut::Error) -> CoreError {
    CoreError::new(
        mos_core::ErrorCode::InvalidInput,
        format!("Nao foi possivel registrar o atalho: {error}"),
        true,
    )
}

/// Os servicos, ou um erro — nunca um panico.
///
/// `Manager::state()` **entra em panico** quando o tipo ainda nao foi
/// gerenciado, e um panico no `main` aborta o processo inteiro. Isso nao e
/// hipotetico: o Tauri cria as janelas declaradas em `tauri.conf.json` ANTES de
/// chamar o `setup`, e a webview ja pode emitir IPC enquanto o `setup` ainda
/// esta abrindo o banco. Medido nesta maquina, com backtrace:
///
/// ```text
/// mos_desktop_lib::attention::attention_count
///   -> AppHandle::state::<AppState>
///   -> panicked: state() called before manage() for AppState
/// thread caused non-unwinding panic. aborting.
/// ```
///
/// Quem recebe `tauri::State<'_, AppState>` por PARAMETRO nao corre esse risco:
/// o Tauri devolve erro de IPC. O risco e de quem chama `state()` a mao, com o
/// `AppHandle` — e e por isso que este helper existe.
pub(crate) fn services<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<tauri::State<'_, AppState>, CoreError> {
    app.try_state::<AppState>().ok_or_else(|| {
        CoreError::new(
            mos_core::ErrorCode::StorageUnavailable,
            "O M/OS ainda esta abrindo.",
            true,
        )
    })
}

fn lock_error(message: String) -> CoreError {
    CoreError::new(
        mos_core::ErrorCode::StorageUnavailable,
        format!("Estado local indisponivel: {message}"),
        false,
    )
}

/// Marca que quem abriu o M/OS foi o Windows, no logon.
const AUTOSTART_FLAG: &str = "--autostarted";

/// Se o M/OS deve nascer escondido nesta abertura.
///
/// Exige as DUAS coisas: ter sido aberto pelo sistema E a preferencia estar
/// ligada. Abrir a mao sempre mostra a janela — quem clicou no icone quer ver
/// o programa, e esconde-lo seria o app decidindo contra o gesto.
fn should_start_hidden(settings: &UserSettings) -> bool {
    settings.start_minimized && std::env::args().any(|arg| arg == AUTOSTART_FLAG)
}

/// Se o M/OS inicia com o Windows.
///
/// Pergunta ao SISTEMA a cada chamada, e nao a uma configuracao nossa. O
/// `auto-launch` tambem escreve na chave que o Gerenciador de Tarefas usa, e
/// o usuario pode desligar por la sem nos avisar; espelhar isso num booleano
/// nosso faria a tela afirmar "ligado" sobre algo desligado (ADR-043).
#[tauri::command]
fn autostart_enabled<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<bool, CoreError> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|error| {
        CoreError::new(
            mos_core::ErrorCode::Io,
            format!("Nao consegui ler a inicializacao do Windows: {error}"),
            false,
        )
    })
}

/// Liga ou desliga a entrada do M/OS na inicializacao do Windows.
///
/// # A recusa de 2026-08-25
///
/// **Um build de desenvolvimento nao pode se registrar no logon.**
///
/// O `tauri-plugin-autostart` grava no registro o caminho do executavel EM
/// EXECUCAO. Ligar o interruptor rodando `npm run tauri dev` grava
/// `target\debug\mos-desktop.exe` — e esse binario carrega o `devUrl`,
/// `http://localhost:1420`. No logon o Vite nao existe, entao TODA janela do
/// M/OS abre com a pagina de erro do WebView2 ("Nao consigo chegar a esta
/// pagina", `ERR_CONNECTION_REFUSED`), inclusive a janelinha do canto que
/// aparece sobre o AutoCAD.
///
/// Aconteceu de verdade, e ficou assim por dias. O que tornou o estrago
/// silencioso foi a combinacao com `autostart_enabled`, que pergunta ao SISTEMA
/// se existe a chave: existia. Settings dizia "ligado", com toda razao, sobre um
/// caminho que nunca poderia funcionar.
///
/// Desligar continua valendo em dev — quem esta com o registro envenenado
/// precisa de um jeito de limpa-lo, e ele e este.
#[tauri::command]
fn autostart_set<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    enabled: bool,
) -> Result<bool, CoreError> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled && tauri::is_dev() {
        return Err(CoreError::new(
            mos_core::ErrorCode::InvalidTransition,
            "Este e um build de desenvolvimento: ele so abre com o servidor do Vite de pe.              Registra-lo no logon faria o M/OS abrir numa tela de erro toda vez que voce              ligasse o computador. Ligue a inicializacao pelo M/OS instalado.",
            false,
        ));
    }
    let manager = app.autolaunch();
    let outcome = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    outcome.map_err(|error| {
        CoreError::new(
            mos_core::ErrorCode::Io,
            format!("Nao consegui mudar a inicializacao do Windows: {error}"),
            false,
        )
    })?;
    // Devolve o que o SISTEMA passou a dizer, e nao o que foi pedido: se a
    // gravacao no registro nao pegou, a tela precisa mostrar a verdade.
    autostart_enabled(app)
}

#[tauri::command]
fn start_minimized(state: tauri::State<'_, AppState>) -> bool {
    load_settings(&state.settings_path).start_minimized
}

#[tauri::command]
fn set_start_minimized(state: tauri::State<'_, AppState>, value: bool) -> Result<bool, CoreError> {
    let mut settings = load_settings(&state.settings_path);
    settings.start_minimized = value;
    save_settings(&state.settings_path, &settings)?;
    Ok(value)
}
#[tauri::command]
fn widget_placements(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<mos_core::WidgetPlacement>, CoreError> {
    state.work.widget_placements()
}

#[tauri::command]
fn radial_pins(state: tauri::State<'_, AppState>) -> Result<Vec<mos_core::RadialPin>, CoreError> {
    state.work.radial_pins()
}

/// `workspace_id` ausente e a visao "Todos", como no arranjo da Home
/// (migration 0021).
#[tauri::command]
fn set_radial_pin(
    state: tauri::State<'_, AppState>,
    workspace_id: Option<String>,
    pin: mos_core::RadialPinInput,
) -> Result<Vec<mos_core::RadialPin>, CoreError> {
    state.work.set_radial_pin(workspace_id.as_deref(), pin)
}

#[tauri::command]
fn clear_radial_pin(
    state: tauri::State<'_, AppState>,
    workspace_id: Option<String>,
    slot: i64,
) -> Result<Vec<mos_core::RadialPin>, CoreError> {
    state.work.clear_radial_pin(workspace_id.as_deref(), slot)
}

/// `workspace_id` ausente e a visao "Todos", que arruma a propria Home
/// (migration 0018). O front manda `null` quando nenhum Workspace esta
/// selecionado.
#[tauri::command]
fn set_widget_layout(
    state: tauri::State<'_, AppState>,
    workspace_id: Option<String>,
    placements: Vec<mos_core::WidgetPlacementInput>,
) -> Result<Vec<mos_core::WidgetPlacement>, CoreError> {
    state
        .work
        .set_widget_layout(workspace_id.as_deref(), &placements)
}

#[tauri::command]
fn reset_widget_layout(
    state: tauri::State<'_, AppState>,
    workspace_id: Option<String>,
) -> Result<Vec<mos_core::WidgetPlacement>, CoreError> {
    state.work.reset_widget_layout(workspace_id.as_deref())
}
pub(crate) fn load_settings(path: &std::path::Path) -> UserSettings {
    let mut settings = fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<UserSettings>(&json).ok())
        .unwrap_or_default();
    if settings.voice_shortcut.trim().is_empty() {
        settings.voice_shortcut = DEFAULT_VOICE_SHORTCUT.into();
    }
    if settings.capture_shortcut.trim().is_empty() {
        settings.capture_shortcut = DEFAULT_CAPTURE_SHORTCUT.into();
    }
    settings
}

fn load_shortcut(path: &std::path::Path) -> String {
    load_settings(path).capture_shortcut
}

fn load_voice_shortcut(path: &std::path::Path) -> String {
    load_settings(path).voice_shortcut
}

/// Grava o arquivo inteiro a partir do que ja estava la.
///
/// Ler-modificar-gravar e nao reconstruir do zero: uma versao anterior desta
/// funcao montava `UserSettings` so com o atalho, e qualquer campo novo seria
/// apagado no proximo clique em Aplicar — sem erro nenhum.
pub(crate) fn save_settings(
    path: &std::path::Path,
    settings: &UserSettings,
) -> Result<(), CoreError> {
    let json = serde_json::to_vec_pretty(settings).map_err(|error| {
        CoreError::new(
            mos_core::ErrorCode::Io,
            format!("Nao foi possivel salvar a configuracao: {error}"),
            false,
        )
    })?;
    fs::write(path, json).map_err(|error| {
        CoreError::new(
            mos_core::ErrorCode::Io,
            format!("Nao foi possivel salvar a configuracao: {error}"),
            false,
        )
    })
}

/// Le a configuracao do transcritor local.
///
/// Ler do arquivo a cada consulta, e nao guardar em memoria, e a mesma regra da
/// ADR-043: o arquivo e a fonte de verdade, e um espelho em memoria divergiria
/// no primeiro caminho que o gravasse sem avisar.
fn whisper_config(path: &std::path::Path) -> mos_transcribe::WhisperConfig {
    load_settings(path).whisper
}

fn set_whisper_config(
    path: &std::path::Path,
    whisper: mos_transcribe::WhisperConfig,
) -> Result<(), CoreError> {
    // Ler-modificar-gravar, e nao reconstruir: um `UserSettings` montado so com
    // este campo apagaria o atalho e o `start_minimized` no proximo clique em
    // Aplicar — sem erro nenhum.
    let mut settings = load_settings(path);
    settings.whisper = whisper;
    save_settings(path, &settings)
}

fn analysis_consent(path: &std::path::Path) -> String {
    load_settings(path).analysis_consent_at
}

fn set_analysis_consent(path: &std::path::Path, at: &str) -> Result<(), CoreError> {
    let mut settings = load_settings(path);
    settings.analysis_consent_at = at.to_owned();
    save_settings(path, &settings)
}

fn persist_voice_shortcut(path: &std::path::Path, shortcut: &str) -> Result<(), CoreError> {
    let mut settings = load_settings(path);
    settings.voice_shortcut = shortcut.into();
    save_settings(path, &settings)
}

fn persist_shortcut(path: &std::path::Path, shortcut: &str) -> Result<(), CoreError> {
    let mut settings = load_settings(path);
    settings.capture_shortcut = shortcut.into();
    save_settings(path, &settings)
}

fn open_external_target(kind: AppLaunchKind, target: &str) -> Result<(), CoreError> {
    match kind {
        AppLaunchKind::Url => {
            if !(target.starts_with("https://") || target.starts_with("http://")) {
                return Err(CoreError::new(
                    mos_core::ErrorCode::InvalidInput,
                    "URL de App deve comecar com http:// ou https://.",
                    false,
                ));
            }
        }
        AppLaunchKind::Path => {
            if !std::path::Path::new(target).exists() {
                return Err(CoreError::new(
                    mos_core::ErrorCode::InvalidInput,
                    "Alvo local do App nao foi encontrado.",
                    false,
                ));
            }
        }
    }
    open_target_with_os(target)
}

/// Abre um original guardado, pelo programa padrao do Windows.
///
/// Recebe um caminho JA validado como filho da area de drops. A validacao mora
/// em `ingest::stored_file`, e nao aqui, porque e la que existe o `FileStore`
/// que sabe onde a area comeca.
pub(crate) fn open_stored_path(path: &std::path::Path) -> Result<(), CoreError> {
    let target = path.to_str().ok_or_else(|| {
        CoreError::new(
            mos_core::ErrorCode::InvalidInput,
            "Caminho do original nao e representavel.",
            false,
        )
    })?;
    open_target_with_os(target)
}

/// Mostra o original na pasta, selecionado, sem abri-lo.
#[cfg(windows)]
pub(crate) fn reveal_stored_path(path: &std::path::Path) -> Result<(), CoreError> {
    // `explorer /select,<caminho>` e o caminho documentado da Microsoft para
    // isto. O argumento vai como argumento, e nao concatenado numa linha de
    // shell: nada do que o usuario escreveu chega a um interpretador.
    std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            CoreError::new(
                mos_core::ErrorCode::Io,
                format!("Nao foi possivel abrir a pasta: {error}"),
                true,
            )
        })
}

#[cfg(not(windows))]
pub(crate) fn reveal_stored_path(_path: &std::path::Path) -> Result<(), CoreError> {
    Err(CoreError::new(
        mos_core::ErrorCode::InvalidTransition,
        "Mostrar na pasta esta disponivel apenas no Windows nesta versao.",
        false,
    ))
}

#[cfg(windows)]
fn open_target_with_os(target: &str) -> Result<(), CoreError> {
    use std::ptr;
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

    let operation = wide_null("open");
    let target = wide_null(target);
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result <= 32 {
        return Err(CoreError::new(
            mos_core::ErrorCode::Io,
            "O Windows nao aceitou abrir este App.",
            true,
        ));
    }
    Ok(())
}

/// O icone da bandeja, no tamanho que o Windows vai mesmo desenhar.
///
/// `default_window_icon()` e o PRIMEIRO quadro do `icon.ico` — o de 16x16, porque
/// `tauri-codegen` faz literalmente `icon_dir.entries()[0]` e o arquivo e escrito
/// em ordem crescente. A 100% isso acerta por acidente, ja que a bandeja pede 16.
/// A 125% ela pede 20 e a 150% pede 24, e o mesmo 16x16 sobe esticado — a mesma
/// falha que a barra de tarefas tinha, so que sem `WM_SETICON` para corrigir,
/// porque a bandeja recebe a imagem e nao o grupo de icones.
///
/// Os `.rgba` sao despejos crus do `gerar-icones.py`, sem cabecalho e sem
/// compressao, porque `Image::new` quer exatamente isso: nenhum decodificador
/// entra na arvore de dependencias so para desenhar um icone de 16 pixels.
#[cfg(windows)]
fn icone_da_bandeja() -> tauri::image::Image<'static> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};

    const B16: &[u8] = include_bytes!("../icons/bandeja-16.rgba");
    const B20: &[u8] = include_bytes!("../icons/bandeja-20.rgba");
    const B24: &[u8] = include_bytes!("../icons/bandeja-24.rgba");
    const B32: &[u8] = include_bytes!("../icons/bandeja-32.rgba");

    // As faixas sao largas de proposito: o que importa e nunca AMPLIAR. Entre
    // dois quadros, escolher o menor faria voltar o esticamento que esta funcao
    // existe para acabar.
    let (bytes, lado) = match unsafe { GetSystemMetrics(SM_CXSMICON) } {
        ..=16 => (B16, 16u32),
        17..=20 => (B20, 20),
        21..=24 => (B24, 24),
        _ => (B32, 32),
    };
    tauri::image::Image::new(bytes, lado, lado)
}

/// Da ao Windows o icone GRANDE, que ele nunca recebeu.
///
/// O Tauri poe na janela um icone unico de 16x16 — o primeiro quadro do
/// `icon.ico`. Isso preenche `ICON_SMALL`, que e o da barra de titulo, e deixa
/// `ICON_BIG` VAZIO. Sem `ICON_BIG` e sem icone de classe, a barra de tarefas e
/// o Alt+Tab pedem 24, 32 ou 48px, nao acham nada, e esticam o 16x16.
///
/// Foi medido, e nao suposto: o icone desenhado na barra de tarefas batia com um
/// 16px ampliado por bilinear (erro medio 3,7) e nao com o quadro de 24px que o
/// `.ico` ja trazia pronto (erro 8,5). Toda a nitidez que `gerar-icones.py`
/// produz desenhando cada tamanho separado morria neste ponto.
///
/// `ExtractIconExW` le o grupo de icones do proprio executavel e escolhe o
/// melhor quadro para cada tamanho de sistema — que e exatamente o servico que
/// faltava. Os dois `HICON` vivem enquanto o app viver; nao ha `DestroyIcon`
/// porque destrui-los enquanto a janela os usa e que seria o erro.
#[cfg(windows)]
fn assentar_icones_da_janela(window: &tauri::WebviewWindow) {
    use std::ptr;
    use windows_sys::Win32::UI::Shell::ExtractIconExW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageW, HICON, ICON_BIG, ICON_SMALL, WM_SETICON,
    };

    let Ok(hwnd) = window.hwnd() else { return };
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let caminho = wide_null(&exe.to_string_lossy());

    let mut grande: HICON = ptr::null_mut();
    let mut pequeno: HICON = ptr::null_mut();
    // 1 grupo, o do indice 0: o icone do proprio aplicativo.
    if unsafe { ExtractIconExW(caminho.as_ptr(), 0, &mut grande, &mut pequeno, 1) } == 0 {
        return;
    }

    let alvo = hwnd.0 as _;
    for (slot, icone) in [(ICON_BIG, grande), (ICON_SMALL, pequeno)] {
        if !icone.is_null() {
            unsafe { SendMessageW(alvo, WM_SETICON, slot as usize, icone as isize) };
        }
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

#[cfg(not(windows))]
fn open_target_with_os(_target: &str) -> Result<(), CoreError> {
    Err(CoreError::new(
        mos_core::ErrorCode::InvalidTransition,
        "Abertura de Apps esta disponivel apenas no Windows nesta versao.",
        false,
    ))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        // O argumento nao liga nada: ele so registra o FATO de que quem
        // abriu foi o sistema, no logon. O que fazer com esse fato —
        // aparecer ou ficar no tray — e preferencia nossa, guardada em
        // settings.json. Sem ele, um M/OS aberto a mao no logon seria
        // indistinguivel de um aberto pelo Windows.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_FLAG]),
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            diagnostico::escrever(
                diagnostico::Nivel::Aviso,
                "abertura",
                "segunda instancia pediu para revelar a janela.",
            );
            reveal_window(app, "main");
        }))
        .plugin(tauri_plugin_dialog::init())
        // O lembrete de "abriu o CAD sem cronometro" precisa aparecer com o
        // M/OS atras do AutoCAD — que e exatamente quando isso acontece.
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                // UM handler para os dois atalhos, entao ele precisa saber qual
                // disparou. A comparacao e contra o `Shortcut` PARSEADO, e nao
                // contra a string: "Ctrl+Alt+Space" e "CommandOrControl+Alt+Space"
                // sao o mesmo atalho escrito de dois jeitos, e comparar texto
                // faria o segundo cair no ramo errado em silencio.
                .with_handler(|app, shortcut, event| {
                    let voice = app
                        .try_state::<AppState>()
                        .and_then(|state| {
                            state
                                .active_voice_shortcut
                                .lock()
                                .ok()
                                .and_then(|guard| guard.clone())
                        })
                        .and_then(|registered| {
                            registered
                                .parse::<tauri_plugin_global_shortcut::Shortcut>()
                                .ok()
                        })
                        .is_some_and(|parsed| &parsed == shortcut);

                    if voice {
                        // Segurar, e nao alternar (design system, §Voz). O
                        // auto-repeat do Windows repete `Pressed` enquanto a
                        // tecla esta afundada; a guarda mora do outro lado.
                        match event.state {
                            ShortcutState::Pressed => voice::shortcut_pressed(app),
                            ShortcutState::Released => voice::shortcut_released(app),
                        }
                        return;
                    }
                    // A faixa vem ANTES da Captura rapida porque o ramo dela e
                    // o fallback: qualquer atalho registrado que nao seja o da
                    // voz cai la, e sem esta guarda o atalho da faixa abriria a
                    // Captura.
                    let faixa = usage::ATALHO
                        .parse::<tauri_plugin_global_shortcut::Shortcut>()
                        .is_ok_and(|parsed| &parsed == shortcut);
                    if faixa {
                        if event.state == ShortcutState::Pressed {
                            usage::alternar_pela_bandeja(app);
                        }
                        return;
                    }
                    if event.state == ShortcutState::Pressed {
                        reveal_window(app, "quick-capture");
                    }
                })
                .build(),
        )
        .setup(|app| {
            // A decisao de esconder a janela, o mais cedo possivel.
            //
            // # A tentativa que falhou, e por que ela nao volta
            //
            // Em 2026-08-25 isto virou o contrario por um dia: `main` passou a
            // nascer `visible: false` no `tauri.conf.json`, e aqui ela era
            // MOSTRADA. A ideia era boa no papel — esconder depois de mostrar
            // nao e esconder, e piscar — e o resultado foi pior que o problema:
            // **a janela passou a abrir MINIMIZADA**, todas as vezes. Medido,
            // com o mesmo teste nos dois binarios:
            //
            // ```text
            // build de 24/08 (visible: true):  1196x799  visivel=True   minimizada=False
            // build de 25/08 (visible: false):  160x28   visivel=True   minimizada=True
            // ```
            //
            // Mostrar uma janela que nunca foi mostrada, de dentro do `setup` —
            // antes de o laco de eventos existir —, nao termina onde deveria no
            // Windows. Nao vale gastar mais tentativas nisso: o pisca-pisca que
            // se queria remover so acontece com `start_minimized` LIGADO, e
            // abrir minimizada acontecia sempre.
            //
            // O que sobrou de util foi a POSICAO: esconder e a primeira coisa
            // do `setup`, e nao mais depois do updater e do `app_data_dir`.
            // Quanto menos codigo entre criar e esconder, menor a piscada.
            //
            // Na duvida, DEIXA VISIVEL: qualquer leitura que nao deu certo cai
            // no ramo que nao esconde, porque um app que nao aparece nao tem
            // como pedir ajuda.
            let nascer_escondido = app
                .path()
                .app_data_dir()
                .ok()
                .map(|dir| dir.join("settings.json"))
                .is_some_and(|path| should_start_hidden(&load_settings(&path)));
            if nascer_escondido {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            let data_directory = app.path().app_data_dir()?;
            fs::create_dir_all(&data_directory)?;
            // O caderno de ocorrencias abre antes do banco de proposito: se a
            // abertura do banco for o que quebra, o panico precisa ter onde
            // cair. Ver `diagnostico.rs`.
            diagnostico::instalar(&data_directory);
            diagnostico::vigiar_janelas(app.handle());
            let settings_path = data_directory.join("settings.json");
            let configured_shortcut = load_shortcut(&settings_path);
            let configured_voice_shortcut = load_voice_shortcut(&settings_path);
            let storage = Arc::new(
                SqliteStorage::open(
                    data_directory.join("m-os.db"),
                    data_directory.join("backups"),
                )
                .map_err(|error| std::io::Error::other(error.to_string()))?,
            );
            // A identidade deste dispositivo e a emissao de operacoes.
            //
            // Registrar e idempotente: abrir o M/OS todo dia nao cria um
            // dispositivo por dia. Ligar a emissao e o que faz cada mudanca
            // deixar rastro na fila de saida — hoje so a Capture emite, e as
            // outras entidades entram uma de cada vez. Ver `docs/SYNC.md`.
            //
            // Falhar aqui NAO impede o M/OS de abrir: sincronizacao e camada
            // por cima, e um sistema que se recusa a funcionar porque nao
            // conseguiu se identificar seria pior que um sem sincronizacao.
            {
                use mos_sync::DeviceRepository;

                let nome = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Este PC".to_owned());
                match storage.este_dispositivo(&nome, "windows", env!("CARGO_PKG_VERSION")) {
                    Ok(device) => {
                        if let Err(causa) = storage.habilitar_sync(device.id) {
                            eprintln!("[sync] emissao desligada: {causa}");
                        } else {
                            // A passagem unica do que ja existia antes do sync.
                            //
                            // Depois de `habilitar_sync` porque precisa do
                            // relogio, e aqui e nao numa migration porque uma
                            // migration roda antes de o dispositivo ter
                            // identidade. Ela se marca como feita e as proximas
                            // aberturas devolvem zero sem varrer nada.
                            //
                            // Falhar aqui tambem nao impede o M/OS de abrir,
                            // pelo mesmo motivo do bloco acima.
                            match storage.backfill_do_sync() {
                                Ok(0) => {}
                                Ok(quantas) => {
                                    eprintln!("[sync] backfill inicial: {quantas} operacoes")
                                }
                                Err(causa) => eprintln!("[sync] backfill nao passou: {causa}"),
                            }

                            // A VARREDURA DE REPARO.
                            //
                            // Depois do backfill, e nao antes: o backfill mexe
                            // na fila de saida, e o reparo olha o banco. Ela
                            // existe porque uma entidade podia chegar, falhar ao
                            // virar linha e sumir da fila quando o app fechasse
                            // — ficando viva no `sync_state` e invisivel na
                            // tela, para sempre.
                            //
                            // Falhar aqui tambem nao impede o M/OS de abrir.
                            match storage.reparar_materializacao() {
                                Ok(reparo) if reparo.reparadas > 0 => eprintln!(
                                    "[sync] reparo: {} de {} entidades voltaram a aparecer",
                                    reparo.reparadas, reparo.examinadas
                                ),
                                Ok(reparo) if !reparo.falharam.is_empty() => eprintln!(
                                    "[sync] reparo: {} dependem de algo que nao chegou: {:?}",
                                    reparo.falharam.len(),
                                    reparo.falharam
                                ),
                                Ok(_) => {}
                                Err(causa) => eprintln!("[sync] reparo nao rodou: {causa}"),
                            }
                        }
                    }
                    Err(causa) => eprintln!("[sync] dispositivo nao registrado: {causa}"),
                }
            }

            app.manage(hermes::HermesState::default());
            // Um relogio so para o processo inteiro. O agendador e o
            // servico precisam do MESMO: dois relogios discordariam sobre o
            // que "agora" significa, e o sono deixaria de ser detectavel.
            let clock: Arc<dyn mos_core::Clock> = Arc::new(mos_core::SystemClock);

            app.manage(AppState {
                captures: CaptureService::new(storage.clone()),
                work: WorkService::new(storage.clone()),
                apps: AppService::new(storage.clone()),
                memory: MemoryService::new(storage.clone()),
                conversations: ConversationService::new(storage.clone()),
                tracking: TrackingService::new(storage.clone()),
                meetings: MeetingService::new(storage.clone(), clock.clone()),
                voice: VoiceService::new(storage.clone(), clock.clone()),
                monitoring: MonitoringService::new(storage.clone()),
                data: DataService::new(storage.clone()),
                attention: AttentionService::new(storage.clone(), clock.clone()),
                daily: DailyService::new(storage.clone(), clock.clone()),
                clock,
                storage,
                shortcut_status: Mutex::new("Registrando...".into()),
                active_shortcut: Mutex::new(None),
                voice_shortcut_status: Mutex::new("Registrando...".into()),
                active_voice_shortcut: Mutex::new(None),
                snapshot_status: Arc::new(Mutex::new("Snapshot ainda nao verificado.".into())),
                settings_path,
            });

            app.manage(sync::SyncRuntime::default());
            // O sync automatico. Ele espera a tela dizer que abriu antes da
            // primeira rodada — ver `sync::iniciar_daemon`.
            sync::iniciar_daemon(app.handle().clone());

            // A mutacao e OUVIDA, e nao emitida. `data-changed` ja sai de 25
            // lugares; tocar os 25 seriam 25 chances de esquecer um, e o
            // esquecido nao daria erro — daria uma entidade que so sai deste
            // aparelho no proximo quarto de hora.
            {
                let handle = app.handle().clone();
                app.listen_any("data-changed", move |_| {
                    let handle = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(sync::DEBOUNCE_DA_MUTACAO).await;
                        sync::acordar(&handle);
                    });
                });
            }

            // Reparo de abertura: o app pode ter sido fechado no meio de um
            // turno, e uma mensagem gravada como `streaming` voltaria
            // eternamente em curso na tela.
            let _ = app.state::<AppState>().conversations.settle_unfinished();

            // O backup do dia nao pode depender de a pessoa ter criado uma
            // Capture: ver `manter_snapshot_do_dia`.
            {
                let state = app.state::<AppState>();
                manter_snapshot_do_dia(&state.data, &state.snapshot_status, &app.handle().clone());
            }

            // A faculdade, uma vez por abertura — e so quando ja ha sessao
            // guardada. Dado academico muda algumas vezes por semana; um
            // polling contra o portal da faculdade seria uma gentileza que
            // ninguem pediu. O resto e o botao "Sincronizar agora".
            academic_sync::sincronizar_na_abertura(app.handle().clone());

            // A Drop Zone precisa do disco antes da primeira janela: a
            // reconciliacao roda na abertura, e ela e quem transforma uma
            // transferencia morta pela metade num fato visivel em vez de um
            // arquivo pela metade guardado como se fosse o original.
            let store = mos_ingest::FileStore::new(&data_directory)
                .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(ingest::IngestState::new(
                mos_ingest::FileStore::new(&data_directory)
                    .map_err(|error| std::io::Error::other(error.message))?,
            ));
            match ingest::reconcile_on_open(app.handle(), &store) {
                Ok(recovered) if recovered > 0 => {
                    let _ = app.emit("ingestion-recovered", recovered);
                }
                Ok(_) => {}
                Err(error) => {
                    eprintln!("Reconciliacao de ingestoes falhou: {}", error.message);
                }
            }
            // As leituras de conteudo que ficaram pendentes retomam em segundo
            // plano. Elas nunca foram condicao para nada: o arquivo ja esta
            // guardado e ja aparece na Library desde o drop.
            ingest::resume_extractions(app.handle(), std::sync::Arc::new(store));

            app.manage(meeting::RecordingState::default());
            app.manage(voice::VoiceRuntime::default());
            // O contexto ambiente do M/OS: a tela aberta e o fuso de quem esta
            // nela. Lido pela voz e pelo Hermes, publicado pelo renderer.
            app.manage(surface::SurfaceRuntime::default());
            // O que o processo anterior deixou pelo caminho. Uma nota em
            // `recording` num processo recem-nascido significa que o anterior
            // morreu sem terminar — e o audio dela pode estar inteiro em disco.
            voice::reconcile(app.handle());
            // A reconciliacao roda ANTES da limpeza, e a ordem e a garantia:
            // uma reuniao interrompida precisa virar `interrupted` antes que
            // qualquer rotina olhe para o disco dela. Invertida, a limpeza veria
            // uma reuniao ainda em `recording` — que ela nao apaga, mas o
            // acoplamento seria por sorte e nao por desenho.
            match meeting::reconcile_on_open(app.handle()) {
                Ok(recovered) if !recovered.is_empty() => {
                    // O aviso e emitido, e nao imposto: quem decide entre
                    // processar e descartar e a pessoa.
                    let _ = app.emit("meeting-interrupted", recovered.len());
                }
                Ok(_) => {}
                Err(error) => {
                    // Falhar aqui NAO pode impedir o app de abrir. Uma reuniao
                    // que ficou por reconciliar continua no banco e sera vista
                    // na proxima abertura; um M/OS que nao abre perde tudo.
                    eprintln!("meeting: reconciliacao de abertura falhou: {error}");
                }
            }
            if let Err(error) = meeting::clean_expired_audio(app.handle()) {
                eprintln!("meeting: limpeza de audio falhou: {error}");
            }

            let shortcut_status = match app.global_shortcut().register(configured_shortcut.as_str())
            {
                Ok(()) => {
                    *app.state::<AppState>()
                        .active_shortcut
                        .lock()
                        .map_err(|error| std::io::Error::other(error.to_string()))? =
                        Some(configured_shortcut.clone());
                    format!("Registrado: {configured_shortcut}")
                }
                Err(error) => format!("Atalho indisponivel: {error}"),
            };
            *app.state::<AppState>()
                .shortcut_status
                .lock()
                .map_err(|error| std::io::Error::other(error.to_string()))? = shortcut_status;

            // O atalho da voz e registrado DEPOIS, e a falha dele nao derruba o
            // outro: quem perde a voz continua tendo a captura por texto, que e
            // o caminho que sempre funciona.
            // O atalho da faixa e registrado por ULTIMO e a falha dele nao
            // derruba nada: quem o perde continua com o item do tray e com o
            // clique na lingueta. E o terceiro caminho, nao o unico.
            if usage::ATALHO != configured_shortcut && usage::ATALHO != configured_voice_shortcut {
                if let Err(causa) = app.global_shortcut().register(usage::ATALHO) {
                    diagnostico::escrever(
                        diagnostico::Nivel::Aviso,
                        "faixa",
                        &format!("o atalho {} nao registrou: {causa}", usage::ATALHO),
                    );
                }
            } else {
                diagnostico::escrever(
                    diagnostico::Nivel::Aviso,
                    "faixa",
                    &format!("o atalho {} ja pertence a outro gesto", usage::ATALHO),
                );
            }

            let voice_status = if configured_voice_shortcut == configured_shortcut {
                "Conflito com o atalho da Captura rapida.".to_owned()
            } else {
                match app
                    .global_shortcut()
                    .register(configured_voice_shortcut.as_str())
                {
                    Ok(()) => {
                        *app.state::<AppState>()
                            .active_voice_shortcut
                            .lock()
                            .map_err(|error| std::io::Error::other(error.to_string()))? =
                            Some(configured_voice_shortcut.clone());
                        format!("Registrado: {configured_voice_shortcut}")
                    }
                    Err(error) => format!("Atalho indisponivel: {error}"),
                }
            };
            *app.state::<AppState>()
                .voice_shortcut_status
                .lock()
                .map_err(|error| std::io::Error::other(error.to_string()))? = voice_status;
            setup_tray(app)?;

            // Depois do tray porque so aqui a janela ja existe de fato.
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                assentar_icones_da_janela(&window);
            }

            // O laco de observacao roda em tarefa propria e nunca na thread da
            // interface: uma varredura de processos leva dezenas de
            // milissegundos, e no fio da janela isso e um engasgo visivel a
            // cada cinco segundos.
            app.manage(monitor::Monitor::default());
            app.manage(usage::Uso::default());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(monitor::run(handle));
            tauri::async_runtime::spawn(attention::run(app.handle().clone()));
            tauri::async_runtime::spawn(meeting::run(app.handle().clone()));
            tauri::async_runtime::spawn(meeting::run_levels(app.handle().clone()));
            tauri::async_runtime::spawn(usage::run(app.handle().clone()));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        // O portao de abertura, e por que ele existe num lugar so.
        //
        // O Tauri cria as janelas declaradas em `tauri.conf.json` ANTES de
        // chamar o `setup`, e a webview ja emite IPC enquanto o `setup` ainda
        // esta abrindo o banco. Um comando que chame `AppHandle::state()` nesse
        // instante nao devolve erro: ele ENTRA EM PANICO, e panico no `main`
        // aborta o processo. Medido nesta maquina, com backtrace:
        //
        // ```text
        // mos_desktop_lib::attention::attention_count -> state::<AppState>
        // panicked: state() called before manage() for AppState
        // thread caused non-unwinding panic. aborting.
        // ```
        //
        // E ele nao e teorico para quem ja usa o M/OS: a janela e larga na
        // proporcao do que o `setup` tem para fazer, e a migration 0022 roda
        // exatamente ali, na primeira abertura depois desta versao.
        //
        // A guarda vive AQUI, e nao nos oitenta e quatro lugares que chamam
        // `state()`, porque aqui ela cobre tambem o comando que alguem
        // escrever amanha. Nenhum comando roda antes de o app estar pronto, e
        // quem chamou cedo recebe um erro que a interface sabe ler.
        .invoke_handler({
            let comandos: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> =
                Box::new(tauri::generate_handler![
                    finance::finance_set_action_secret,
                    finance::finance_clear_action_secret,
                    finance::finance_action_secret_configured,
                    hermes::hermes_status,
                    sync::sync_status,
                    sync::sync_set_endpoint,
                    sync::sync_set_token,
                    sync::sync_clear_token,
                    sync::sync_now,
                    sync::sync_malha,
                    sync::sync_reparar,
                    sync::sync_app_pronto,
                    sync::sync_dispensar_resumo,
                    hermes::hermes_set_credentials,
                    hermes::hermes_clear_credentials,
                    hermes::hermes_set_base_url,
                    hermes::hermes_connect,
                    hermes::hermes_send,
                    hermes::hermes_interrupt,
                    hermes::hermes_approve,
                    hermes::hermes_clarify,
                    hermes::hermes_clarify_cancel,
                    meeting::meeting_start,
                    meeting::meeting_stop,
                    meeting::meeting_pause,
                    meeting::meeting_resume,
                    meeting::meeting_recording,
                    meeting::meeting_list,
                    meeting::meeting_get,
                    meeting::meeting_transcript,
                    meeting::meeting_analysis,
                    meeting::meeting_insights,
                    meeting::meeting_set_project,
                    meeting::meeting_set_title,
                    meeting::meeting_set_notes,
                    meeting::meeting_set_archived,
                    meeting::meeting_process_recovered,
                    meeting::meeting_discard,
                    meeting::meeting_delete,
                    meeting::meeting_interrupted,
                    meeting::meeting_open_commitments,
                    meeting::meeting_transcribe,
                    meeting::meeting_retry,
                    meeting::meeting_transcriber_status,
                    meeting::meeting_set_transcriber,
                    meeting::meeting_analyze,
                    meeting::meeting_analysis_consent,
                    meeting::meeting_set_analysis_consent,
                    meeting::meeting_previews,
                    meeting::meeting_accept_insight,
                    meeting::meeting_dismiss_insight,
                    jarvis::action_undo,
                    calendar::calendar_window,
                    academic::academic_dashboard,
                    academic::academic_today,
                    academic::academic_semesters,
                    academic::academic_create_semester,
                    academic::academic_update_semester,
                    academic::academic_archive_semester,
                    academic::academic_subjects,
                    academic::academic_create_subject,
                    academic::academic_update_subject,
                    academic::academic_archive_subject,
                    academic::academic_assignments,
                    academic::academic_create_assignment,
                    academic::academic_update_assignment,
                    academic::academic_set_assignment_status,
                    academic::academic_archive_assignment,
                    academic::academic_create_task,
                    academic::academic_unlink_task,
                    academic::academic_exams,
                    academic::academic_create_exam,
                    academic::academic_update_exam,
                    academic::academic_archive_exam,
                    academic::academic_materials,
                    academic::academic_link_material,
                    academic::academic_study_sessions,
                    academic::academic_start_study,
                    academic::academic_finish_study,
                    academic::academic_discard_study,
                    academic::academic_set_assignment_decision,
                    academic::academic_set_exam_decision,
                    academic::academic_plan_assignment,
                    academic::academic_plan_exam,
                    academic_sync::univirtus_status,
                    academic_sync::univirtus_connect,
                    academic_sync::univirtus_sync,
                    academic_sync::univirtus_disconnect,
                    academic_sync::univirtus_subject_facts,
                    academic_sync::univirtus_material_url,
                    stale::stale_list,
                    tracking::tracking_default_cronocad_path,
                    tracking::tracking_import_cronocad,
                    tracking::tracking_cronocad_imported_at,
                    tracking::tracking_totals,
                    tracking::tracking_entries,
                    tracking::tracking_record,
                    tracking::tracking_edit,
                    tracking::tracking_trash,
                    tracking::tracking_restore,
                    tracking::timer_current,
                    tracking::timer_start,
                    tracking::timer_set_running,
                    tracking::timer_stop,
                    tracking::timer_discard,
                    tracking::tracking_settings,
                    tracking::tracking_set_settings,
                    tracking::tracking_aplicar_tarifa_padrao,
                    tracking::tracking_report,
                    tracking::tracking_issuer,
                    tracking::tracking_set_issuer,
                    tracking::tracking_export_report_pdf,
                    tracking::tracking_export_invoice_pdf,
                    tracking::tracking_export_csv,
                    tracking::tracking_trashed,
                    tracking::tracking_project_tracking,
                    tracking::tracking_set_project_tracking,
                    tracking::tracking_clients,
                    tracking::tracking_save_client,
                    tracking::tracking_set_client_archived,
                    widget_placements,
                    set_widget_layout,
                    reset_widget_layout,
                    radial_pins,
                    set_radial_pin,
                    clear_radial_pin,
                    autostart_enabled,
                    autostart_set,
                    start_minimized,
                    set_start_minimized,
                    attention::attention_create,
                    attention::attention_list,
                    attention::attention_count,
                    attention::attention_snooze,
                    attention::attention_complete,
                    attention::attention_acknowledge,
                    attention::attention_cancel,
                    attention::attention_archive,
                    usage::usage_faixa,
                    usage::faixa_painel_alternar,
                    usage::faixa_painel_fechar,
                    usage::faixa_recolher,
                    usage::faixa_abrir_app,
                    usage::faixa_zona,
                    daily::daily_today,
                    daily::daily_context,
                    daily::daily_history,
                    daily::daily_session,
                    daily::daily_start,
                    daily::daily_add_objective,
                    daily::daily_update_objective,
                    daily::daily_set_objective_status,
                    daily::daily_set_main,
                    daily::daily_remove_objective,
                    daily::daily_reorder,
                    daily::daily_end,
                    daily::daily_reopen,
                    daily::weekly_week,
                    daily::weekly_pending,
                    daily::weekly_close,
                    monitor::fechar_reuniao_detectada,
                    monitor::silenciar_deteccao,
                    monitor::reminder_pending,
                    monitor::reminder_dismiss,
                    monitor::reminder_suppress,
                    monitor::reminder_silenced,
                    monitor::reminder_unsilence,
                    tracking::monitoring_settings,
                    tracking::monitoring_set_settings,
                    tracking::monitoring_apps,
                    tracking::monitoring_save_app,
                    tracking::monitoring_delete_app,
                    tracking::monitoring_events,
                    tracking::monitoring_timeline,
                    tracking::tracking_record_from_timeline,
                    tracking::monitoring_mark_processed,
                    hermes::hermes_select_conversation,
                    hermes::hermes_load_history,
                    hermes::hermes_disconnect,
                    jarvis::conversation_list,
                    jarvis::conversation_current,
                    jarvis::conversation_create,
                    jarvis::conversation_messages,
                    jarvis::conversation_rename,
                    jarvis::conversation_set_archived,
                    jarvis::conversation_delete,
                    jarvis::conversation_search,
                    jarvis::conversation_truncate,
                    jarvis::action_resolve,
                    open_external_url,
                    ingest::ingest_begin,
                    ingest::ingest_chunk,
                    ingest::ingest_finish,
                    ingest::ingest_abort,
                    ingest::ingest_text,
                    ingest::ingest_url,
                    ingest::ingest_undo,
                    ingest::ingest_accept_suggestion,
                    ingest::list_ingestions,
                    ingest::open_ingested_file,
                    ingest::reveal_ingested_file,
                    create_capture,
                    get_capture,
                    list_recent,
                    list_inbox,
                    list_archived,
                    list_trashed,
                    search_captures,
                    search_all,
                    list_functions,
                    search_functions,
                    mark_capture_processed,
                    move_capture_to_inbox,
                    archive_capture,
                    trash_capture,
                    restore_capture,
                    rebuild_search,
                    create_resource,
                    update_resource,
                    get_resource,
                    list_resources,
                    list_trashed_resources,
                    list_resource_workspaces,
                    set_resource_workspace,
                    search_resources,
                    set_resource_archived,
                    trash_resource,
                    restore_resource,
                    open_resource,
                    create_workspace,
                    update_workspace,
                    get_workspace,
                    list_workspaces,
                    set_workspace_archived,
                    list_workspace_projects,
                    list_workspace_apps,
                    list_project_workspaces,
                    list_app_workspaces,
                    set_project_workspace,
                    set_app_workspace,
                    set_workspace_widget,
                    list_hidden_widgets,
                    delete_capture,
                    delete_task,
                    delete_project,
                    delete_workspace,
                    delete_registered_app,
                    delete_resource,
                    create_project,
                    update_project,
                    get_project,
                    list_projects,
                    set_project_archived,
                    create_task,
                    update_task,
                    get_task,
                    list_tasks,
                    set_task_state,
                    set_task_archived,
                    create_registered_app,
                    list_app_catalog,
                    register_app_catalog,
                    update_registered_app,
                    get_registered_app,
                    list_registered_apps,
                    set_registered_app_archived,
                    mark_registered_app_opened,
                    open_registered_app,
                    create_backup,
                    inspect_backup,
                    restore_backup,
                    export_json,
                    get_app_status,
                    set_capture_shortcut,
                    set_voice_shortcut,
                    show_quick_capture,
                    hide_quick_capture,
                    voice::voice_start,
                    voice::voice_stop,
                    voice::voice_cancel,
                    voice::voice_recording,
                    voice::voice_pending,
                    voice::voice_retry,
                    voice::voice_discard,
                    voice::voice_act,
                    surface::surface_set_context,
                    surface::surface_set_locale,
                    diagnostico::diagnostico_janela_viva,
                    diagnostico::diagnostico_registrar,
                    diagnostico::diagnostico_recente,
                    diagnostico::diagnostico_caminho,
                    atualizacao::atualizacao_estado,
                    atualizacao::atualizacao_anotar_verificacao,
                    atualizacao::atualizacao_anotar_falha,
                ]);
            move |invoke| {
                // O caderno de ocorrencias atravessa o portao.
                //
                // Ele existe justamente para os instantes em que o `AppState`
                // ainda nao subiu — ou nunca vai subir. Barra-lo aqui seria
                // apagar a luz exatamente onde esta escuro: a janela que morre
                // antes do `manage()` e a que mais precisa deixar rastro.
                //
                // Seguro porque nenhum comando de `diagnostico.rs` toca banco,
                // cofre ou estado gerenciado: os quatro leem e escrevem um
                // arquivo de texto cujo caminho vive num `OnceLock` proprio.
                let do_diagnostico = invoke.message.command().starts_with("diagnostico_");
                if !do_diagnostico
                    && invoke
                        .message
                        .webview_ref()
                        .try_state::<AppState>()
                        .is_none()
                {
                    // Erro ESTRUTURADO, e nao string crua. Sem o `retryable`, a
                    // tela nao conseguia separar "cheguei cedo demais" — que
                    // passa sozinho em menos de um segundo — de "o banco
                    // quebrou", e tratava as duas como falha definitiva.
                    invoke.resolver.reject(CoreError::new(
                        mos_core::ErrorCode::StorageUnavailable,
                        "O M/OS ainda esta abrindo.",
                        true,
                    ));
                    return true;
                }
                comandos(invoke)
            }
        })
        .build(tauri::generate_context!())
        .expect("error while running M/OS")
        .run(|app, event| {
            // O laco de observacao precisa saber que acabou. Sem isto ele
            // continua acordando a cada cinco segundos depois de o usuario ter
            // saido, e o processo nao morre.
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                app.state::<monitor::Monitor>().stop();
                // Uma gravacao em curso no `Quit` precisa FECHAR, e nao ser
                // abandonada. Sem isto, o ultimo chunk fica sem `sync_all` e a
                // proxima abertura recupera a reuniao como interrompida — o que
                // seria verdade, mas seria uma interrupcao causada por nos.
                meeting::shutdown(app);
            }
        });
}
