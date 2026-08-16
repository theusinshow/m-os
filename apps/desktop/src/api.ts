import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import type { ActiveTimer, ActivityType, AppCapabilities, AppCatalogEntry, AppLaunchKind, AppStatus, BackupInspection, BackupReceipt, Capture, CaptureSource, FunctionDefinition, HiddenWidget, ImportReport, Project, RegisteredApp, TimeEntry, Resource, ResourceKind, ResourceWorkspace, SearchItem, Task, TaskState, Totals, UpdateInfo, UpdateProgress, Workspace } from "./types";

let pendingUpdate: Update | null = null;

export const api = {
  createCapture(content: string, source: CaptureSource) {
    return invoke<Capture>("create_capture", { input: { content, source } });
  },
  getCapture(id: string) {
    return invoke<Capture>("get_capture", { id });
  },
  recent() {
    return invoke<Capture[]>("list_recent");
  },
  inbox() {
    return invoke<Capture[]>("list_inbox");
  },
  archived() {
    return invoke<Capture[]>("list_archived");
  },
  trashed() {
    return invoke<Capture[]>("list_trashed");
  },
  search(query: string, includeArchived: boolean) {
    return invoke<SearchItem[]>("search_all", { query, includeArchived });
  },
  functions() {
    return invoke<FunctionDefinition[]>("list_functions");
  },
  searchFunctions(query: string) {
    return invoke<FunctionDefinition[]>("search_functions", { query });
  },
  markProcessed(id: string) {
    return invoke<Capture>("mark_capture_processed", { id });
  },
  moveToInbox(id: string) {
    return invoke<Capture>("move_capture_to_inbox", { id });
  },
  archive(id: string) {
    return invoke<Capture>("archive_capture", { id });
  },
  trash(id: string) {
    return invoke<Capture>("trash_capture", { id });
  },
  restore(id: string) {
    return invoke<Capture>("restore_capture", { id });
  },
  resources(includeArchived = false) {
    return invoke<Resource[]>("list_resources", { includeArchived });
  },
  trashedResources() {
    return invoke<Resource[]>("list_trashed_resources");
  },
  resource(id: string) {
    return invoke<Resource>("get_resource", { id });
  },
  searchResources(query: string, includeArchived = false) {
    return invoke<Resource[]>("search_resources", { query, includeArchived });
  },
  createResource(kind: ResourceKind, title: string, url: string, note: string, sourceCaptureId: string | null = null) {
    return invoke<Resource>("create_resource", { input: { kind, title, url, note, sourceCaptureId } });
  },
  updateResource(id: string, kind: ResourceKind, title: string, url: string, note: string) {
    return invoke<Resource>("update_resource", { input: { id, kind, title, url, note } });
  },
  setResourceArchived(id: string, archived: boolean) {
    return invoke<Resource>("set_resource_archived", { id, archived });
  },
  trashResource(id: string) {
    return invoke<Resource>("trash_resource", { id });
  },
  restoreResource(id: string) {
    return invoke<Resource>("restore_resource", { id });
  },
  openResource(id: string) {
    return invoke<void>("open_resource", { id });
  },
  /** Link vindo de uma resposta do Hermes. O backend recusa o que nao for http(s). */
  openExternalUrl(url: string) {
    return invoke<void>("open_external_url", { url });
  },
  deleteCapture(id: string) {
    return invoke<void>("delete_capture", { id });
  },
  deleteTask(id: string) {
    return invoke<void>("delete_task", { id });
  },
  deleteProject(id: string) {
    return invoke<void>("delete_project", { id });
  },
  deleteWorkspace(id: string) {
    return invoke<void>("delete_workspace", { id });
  },
  deleteRegisteredApp(id: string) {
    return invoke<void>("delete_registered_app", { id });
  },
  deleteResource(id: string) {
    return invoke<void>("delete_resource", { id });
  },
  resourceWorkspaces() {
    return invoke<ResourceWorkspace[]>("list_resource_workspaces");
  },
  setResourceWorkspace(resourceId: string, workspaceId: string, linked: boolean) {
    return invoke<void>("set_resource_workspace", { resourceId, workspaceId, linked });
  },
  projects(includeArchived = false) {
    return invoke<Project[]>("list_projects", { includeArchived });
  },
  createProject(name: string, description: string, repository = "") {
    return invoke<Project>("create_project", { input: { name, description, repository } });
  },
  updateProject(id: string, name: string, description: string, repository = "") {
    return invoke<Project>("update_project", { input: { id, name, description, repository } });
  },
  setProjectArchived(id: string, archived: boolean) {
    return invoke<Project>("set_project_archived", { id, archived });
  },
  tasks(includeArchived = false) {
    return invoke<Task[]>("list_tasks", { includeArchived });
  },
  createTask(title: string, description: string, projectId: string | null, sourceCaptureId: string | null = null) {
    return invoke<Task>("create_task", { input: { title, description, projectId, sourceCaptureId } });
  },
  updateTask(id: string, title: string, description: string, projectId: string | null) {
    return invoke<Task>("update_task", { input: { id, title, description, projectId } });
  },
  setTaskState(id: string, taskState: TaskState) {
    return invoke<Task>("set_task_state", { id, taskState });
  },
  setTaskArchived(id: string, archived: boolean) {
    return invoke<Task>("set_task_archived", { id, archived });
  },
  registeredApps(includeArchived = false) {
    return invoke<RegisteredApp[]>("list_registered_apps", { includeArchived });
  },
  createRegisteredApp(name: string, description: string, sourceUrl: string | null, launchKind: AppLaunchKind | null, launchTarget: string | null) {
    return invoke<RegisteredApp>("create_registered_app", { input: { name, description, sourceUrl, launchKind, launchTarget } });
  },
  updateRegisteredApp(id: string, name: string, description: string, sourceUrl: string | null, launchKind: AppLaunchKind | null, launchTarget: string | null, capabilities: AppCapabilities) {
    return invoke<RegisteredApp>("update_registered_app", { input: { id, name, description, sourceUrl, launchKind, launchTarget, ...capabilities } });
  },
  appCatalog() {
    return invoke<AppCatalogEntry[]>("list_app_catalog");
  },
  registerAppCatalog(ids: string[]) {
    return invoke<RegisteredApp[]>("register_app_catalog", { ids });
  },
  setRegisteredAppArchived(id: string, archived: boolean) {
    return invoke<RegisteredApp>("set_registered_app_archived", { id, archived });
  },
  workspaces(includeArchived = false) {
    return invoke<Workspace[]>("list_workspaces", { includeArchived });
  },
  createWorkspace(name: string, description: string) {
    return invoke<Workspace>("create_workspace", { input: { name, description } });
  },
  updateWorkspace(id: string, name: string, description: string) {
    return invoke<Workspace>("update_workspace", { input: { id, name, description } });
  },
  setWorkspaceArchived(id: string, archived: boolean) {
    return invoke<Workspace>("set_workspace_archived", { id, archived });
  },
  workspaceProjects(id: string, includeArchived = false) {
    return invoke<Project[]>("list_workspace_projects", { id, includeArchived });
  },
  workspaceApps(id: string, includeArchived = false) {
    return invoke<RegisteredApp[]>("list_workspace_apps", { id, includeArchived });
  },
  projectWorkspaces(id: string) {
    return invoke<Workspace[]>("list_project_workspaces", { id });
  },
  appWorkspaces(id: string) {
    return invoke<Workspace[]>("list_app_workspaces", { id });
  },
  setProjectWorkspace(projectId: string, workspaceId: string, linked: boolean) {
    return invoke<void>("set_project_workspace", { projectId, workspaceId, linked });
  },
  setAppWorkspace(appId: string, workspaceId: string, linked: boolean) {
    return invoke<void>("set_app_workspace", { appId, workspaceId, linked });
  },
  hiddenWidgets() {
    return invoke<HiddenWidget[]>("list_hidden_widgets");
  },
  setWorkspaceWidget(widgetId: string, workspaceId: string, visible: boolean) {
    return invoke<void>("set_workspace_widget", { widgetId, workspaceId, visible });
  },
  markRegisteredAppOpened(id: string) {
    return invoke<RegisteredApp>("mark_registered_app_opened", { id });
  },
  openRegisteredApp(id: string) {
    return invoke<RegisteredApp>("open_registered_app", { id });
  },
  rebuildSearch() {
    return invoke<number>("rebuild_search");
  },
  status() {
    return invoke<AppStatus>("get_app_status");
  },
  setShortcut(shortcut: string) {
    return invoke<string>("set_capture_shortcut", { shortcut });
  },
  showQuickCapture() {
    return invoke<void>("show_quick_capture");
  },
  hideQuickCapture() {
    return invoke<void>("hide_quick_capture");
  },
  /** Totais por Project, com o arredondamento configurado já aplicado. */
  trackingTotals() {
    return invoke<Record<string, Totals>>("tracking_totals");
  },
  /** As sessões, em tempo REAL — sem arredondar e sem descontar inatividade. */
  trackingEntries(projectId?: string) {
    return invoke<TimeEntry[]>("tracking_entries", { projectId: projectId ?? null });
  },
  timerCurrent() {
    return invoke<ActiveTimer | null>("timer_current");
  },
  timerStart(projectId: string, description: string, activityType: ActivityType) {
    return invoke<ActiveTimer>("timer_start", { projectId, description, activityType });
  },
  timerSetRunning(running: boolean) {
    return invoke<ActiveTimer>("timer_set_running", { running });
  },
  /** Encerra e devolve a sessão gravada. */
  timerStop() {
    return invoke<TimeEntry>("timer_stop");
  },
  /** Onde o CronoCAD guarda o banco, se ele estiver instalado. */
  defaultCronocadPath() {
    return invoke<string | null>("tracking_default_cronocad_path");
  },
  /** Quando o CronoCAD foi importado, se foi. Pergunta ao banco, não à sessão. */
  cronocadImportedAt() {
    return invoke<string | null>("tracking_cronocad_imported_at");
  },
  /** Caminho de mão única, roda uma vez. A origem é aberta só para leitura. */
  importCronocad(path: string) {
    return invoke<ImportReport>("tracking_import_cronocad", { path });
  },
  createBackup(path: string) {
    return invoke<BackupReceipt>("create_backup", { path });
  },
  inspectBackup(path: string) {
    return invoke<BackupInspection>("inspect_backup", { path });
  },
  restoreBackup(path: string) {
    return invoke<BackupReceipt>("restore_backup", { path });
  },
  exportJson(path: string) {
    return invoke<BackupReceipt>("export_json", { path });
  },
  async checkForUpdate() {
    const update = await check({ timeout: 30_000 });
    pendingUpdate = update;
    if (!update) return null;
    return {
      currentVersion: update.currentVersion,
      version: update.version,
      date: update.date ?? null,
      body: update.body ?? "",
    } satisfies UpdateInfo;
  },
  async installUpdate(onProgress: (progress: UpdateProgress) => void) {
    if (!pendingUpdate) throw new Error("Nenhuma atualizacao pendente.");
    let downloaded = 0;
    let total: number | null = null;
    await pendingUpdate.downloadAndInstall((event) => {
      if (event.event === "Started") {
        downloaded = 0;
        total = event.data.contentLength ?? null;
      }
      if (event.event === "Progress") downloaded += event.data.chunkLength;
      if (event.event === "Finished") downloaded = total ?? downloaded;
      onProgress({ downloaded, total });
    });
    pendingUpdate = null;
    await relaunch();
  },
};

export function appError(error: unknown): { message: string; retryable: boolean } {
  if (error && typeof error === "object" && "message" in error) {
    const candidate = error as { message: unknown; retryable?: unknown };
    return {
      message: String(candidate.message),
      retryable: candidate.retryable === true,
    };
  }
  return { message: String(error), retryable: false };
}
