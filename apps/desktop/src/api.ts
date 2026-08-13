import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import type { AppLaunchKind, AppStatus, BackupInspection, BackupReceipt, Capture, CaptureSource, Project, RegisteredApp, SearchItem, Task, TaskState, UpdateInfo, UpdateProgress, Workspace } from "./types";

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
  projects(includeArchived = false) {
    return invoke<Project[]>("list_projects", { includeArchived });
  },
  createProject(name: string, description: string) {
    return invoke<Project>("create_project", { input: { name, description } });
  },
  updateProject(id: string, name: string, description: string) {
    return invoke<Project>("update_project", { input: { id, name, description } });
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
  createRegisteredApp(name: string, description: string, launchKind: AppLaunchKind | null, launchTarget: string | null) {
    return invoke<RegisteredApp>("create_registered_app", { input: { name, description, launchKind, launchTarget } });
  },
  updateRegisteredApp(id: string, name: string, description: string, launchKind: AppLaunchKind | null, launchTarget: string | null) {
    return invoke<RegisteredApp>("update_registered_app", { input: { id, name, description, launchKind, launchTarget } });
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
