use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use mos_core::{
    AppCatalogEntry, AppLaunchKind, AppService, BackupInspection, BackupReceipt, Capture,
    CaptureService, CoreError, CreateAppInput, CreateCaptureInput, CreateProjectInput,
    CreateResourceInput, CreateTaskInput, CreateWorkspaceInput, DataService, FunctionDefinition,
    HiddenWidget, MemoryService, Project, RegisteredApp, Resource, ResourceWorkspace, SearchItem,
    Task, TaskState, UpdateAppInput, UpdateProjectInput, UpdateResourceInput, UpdateTaskInput,
    UpdateWorkspaceInput, WorkService, Workspace,
};
use mos_storage_sqlite::{SqliteStorage, StorageHealth};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

mod hermes;

const DEFAULT_CAPTURE_SHORTCUT: &str = "Ctrl+Shift+Space";

struct AppState {
    captures: CaptureService,
    work: WorkService,
    apps: AppService,
    memory: MemoryService,
    data: DataService,
    storage: Arc<SqliteStorage>,
    shortcut_status: Mutex<String>,
    active_shortcut: Mutex<Option<String>>,
    snapshot_status: Arc<Mutex<String>>,
    settings_path: PathBuf,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserSettings {
    capture_shortcut: String,
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
    workspace_id: &str,
    widget_id: &str,
    visible: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state
        .work
        .set_widget_hidden(workspace_id, widget_id, !visible)?;
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

fn reveal_window<R: Runtime>(app: &AppHandle<R>, label: &str) {
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
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir M/OS", true, None::<&str>)?;
    let capture = MenuItem::with_id(app, "capture", "Captura rapida", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &capture, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .tooltip("M/OS")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => reveal_window(app, "main"),
            "capture" => reveal_window(app, "quick-capture"),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

fn shortcut_error(error: tauri_plugin_global_shortcut::Error) -> CoreError {
    CoreError::new(
        mos_core::ErrorCode::InvalidInput,
        format!("Nao foi possivel registrar o atalho: {error}"),
        true,
    )
}

fn lock_error(message: String) -> CoreError {
    CoreError::new(
        mos_core::ErrorCode::StorageUnavailable,
        format!("Estado local indisponivel: {message}"),
        false,
    )
}

fn load_shortcut(path: &std::path::Path) -> String {
    fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<UserSettings>(&json).ok())
        .map(|settings| settings.capture_shortcut)
        .filter(|shortcut| !shortcut.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CAPTURE_SHORTCUT.into())
}

fn persist_shortcut(path: &std::path::Path, shortcut: &str) -> Result<(), CoreError> {
    let json = serde_json::to_vec_pretty(&UserSettings {
        capture_shortcut: shortcut.into(),
    })
    .map_err(|error| {
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
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            reveal_window(app, "main");
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        reveal_window(app, "quick-capture");
                    }
                })
                .build(),
        )
        .setup(|app| {
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            let data_directory = app.path().app_data_dir()?;
            fs::create_dir_all(&data_directory)?;
            let settings_path = data_directory.join("settings.json");
            let configured_shortcut = load_shortcut(&settings_path);
            let storage = Arc::new(
                SqliteStorage::open(
                    data_directory.join("m-os.db"),
                    data_directory.join("backups"),
                )
                .map_err(|error| std::io::Error::other(error.to_string()))?,
            );
            app.manage(hermes::HermesState::default());
            app.manage(AppState {
                captures: CaptureService::new(storage.clone()),
                work: WorkService::new(storage.clone()),
                apps: AppService::new(storage.clone()),
                memory: MemoryService::new(storage.clone()),
                data: DataService::new(storage.clone()),
                storage,
                shortcut_status: Mutex::new("Registrando...".into()),
                active_shortcut: Mutex::new(None),
                snapshot_status: Arc::new(Mutex::new("Snapshot ainda nao verificado.".into())),
                settings_path,
            });

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
            setup_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            hermes::hermes_status,
            hermes::hermes_set_credentials,
            hermes::hermes_clear_credentials,
            hermes::hermes_set_base_url,
            hermes::hermes_connect,
            hermes::hermes_send,
            hermes::hermes_interrupt,
            hermes::hermes_approve,
            hermes::hermes_disconnect,
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
            show_quick_capture,
            hide_quick_capture,
        ])
        .run(tauri::generate_context!())
        .expect("error while running M/OS");
}
