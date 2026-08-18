import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import type { Reminder, ReminderTarget, ActiveTimer, ActivityEvent, ActivityType, AppCapabilities, CalendarItem, Client, ClientInput, InvoiceData, Issuer, MonitoredApp, MonitoringSettings, PendingReminder, Period, ProjectTracking, ReportLine, ReportPdfData, SilencedApp, TrackingSettings, AppCatalogEntry, AppLaunchKind, AppStatus, BackupInspection, BackupReceipt, Capture, CaptureSource, FunctionDefinition, HiddenWidget, ImportReport, Project, RegisteredApp, TimeEntry, Resource, ResourceKind, ResourceWorkspace, SearchItem, Task, TaskState, TimeEntryEdit, Totals, UpdateInfo, UpdateProgress, Workspace } from "./types";

let pendingUpdate: Update | null = null;

export const api = {
  // --- Attention System ---
  //
  // `at` e `until` viajam como instante RFC 3339 ja resolvido. O calculo de
  // "amanha de manha" acontece AQUI de proposito: a interface e o unico lado
  // que conhece o fuso de quem clicou. Mesmo padrao que o lembrete do monitor
  // ja usa para o fim do dia.
  createReminder(title: string, body: string, at: Date, target?: ReminderTarget) {
    return invoke<Reminder>("attention_create", {
      input: {
        title,
        body,
        at: at.toISOString(),
        targetType: target?.type,
        targetId: target?.id,
      },
    });
  },
  reminders() {
    return invoke<Reminder[]>("attention_list");
  },
  attentionCount() {
    return invoke<number>("attention_count");
  },
  snoozeReminder(id: string, until: Date) {
    return invoke<Reminder>("attention_snooze", { id, until: until.toISOString() });
  },
  completeReminder(id: string) {
    return invoke<Reminder>("attention_complete", { id });
  },
  acknowledgeReminder(id: string) {
    return invoke<Reminder>("attention_acknowledge", { id });
  },
  cancelReminder(id: string) {
    return invoke<Reminder>("attention_cancel", { id });
  },
  archiveReminder(id: string) {
    return invoke<Reminder>("attention_archive", { id });
  },
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
  /**
   * Tudo o que o M/OS registrou entre dois instantes.
   *
   * A janela vai como instante e não como data: quem decide onde um dia começa
   * é esta ponta, que conhece o fuso de quem está olhando.
   */
  calendarWindow(since: string, until: string) {
    return invoke<CalendarItem[]>("calendar_window", { since, until });
  },
  /** Totais por Project, com o arredondamento configurado já aplicado. */
  trackingTotals() {
    return invoke<Record<string, Totals>>("tracking_totals");
  },
  /** As sessões, em tempo REAL — sem arredondar e sem descontar inatividade. */
  trackingEntries(projectId?: string) {
    return invoke<TimeEntry[]>("tracking_entries", { projectId: projectId ?? null });
  },
  /** Lança tempo que o cronômetro não contou. Fica marcado como `manual`. */
  trackingRecord(input: { projectId: string; startedAt: string; durationSeconds: number; description: string; activityType: ActivityType; billable: boolean }) {
    return invoke<TimeEntry>("tracking_record", input);
  },
  trackingEdit(id: string, edit: TimeEntryEdit) {
    return invoke<TimeEntry>("tracking_edit", { id, edit });
  },
  trackingTrash(id: string) {
    return invoke<void>("tracking_trash", { id });
  },
  trackingRestore(id: string) {
    return invoke<void>("tracking_restore", { id });
  },
  /** A lixeira: o que foi removido continua no banco e volta por aqui. */
  trackingTrashed() {
    return invoke<TimeEntry[]>("tracking_trashed");
  },
  /**
   * As sessões de um período, cada uma com o que vale.
   *
   * Data ausente é "sem borda deste lado". O cálculo vem do backend porque é lá
   * que o arredondamento vive — repetir a conta aqui daria um total de tela
   * diferente do total da fatura, e ninguém saberia qual acreditar.
   */
  trackingReport(since: string | null, until: string | null) {
    return invoke<ReportLine[]>("tracking_report", { since, until });
  },
  trackingIssuer() {
    return invoke<Issuer>("tracking_issuer");
  },
  trackingSetIssuer(issuer: Issuer) {
    return invoke<Issuer>("tracking_set_issuer", { issuer });
  },
  /** `false` quando o usuário fecha o diálogo sem escolher onde salvar. */
  exportReportPdf(report: ReportPdfData, suggestedName: string) {
    return invoke<boolean>("tracking_export_report_pdf", { report, suggestedName });
  },
  exportInvoicePdf(invoice: InvoiceData, suggestedName: string) {
    return invoke<boolean>("tracking_export_invoice_pdf", { invoice, suggestedName });
  },
  exportCsv(contents: string, suggestedName: string) {
    return invoke<boolean>("tracking_export_csv", { contents, suggestedName });
  },
  trackingSettings() {
    return invoke<TrackingSettings>("tracking_settings");
  },
  trackingSetSettings(settings: TrackingSettings) {
    return invoke<TrackingSettings>("tracking_set_settings", { settings });
  },
  /** Os dados de cobrança de todo Project que tem algum. */
  projectTracking() {
    return invoke<ProjectTracking[]>("tracking_project_tracking");
  },
  /**
   * Grava valor/hora, código, cliente e meta.
   *
   * Vale do momento da gravação em diante: cada sessão já guarda a taxa que
   * valia quando o trabalho aconteceu, e reajustar aqui não reescreve o passado.
   */
  setProjectTracking(tracking: ProjectTracking) {
    return invoke<ProjectTracking>("tracking_set_project_tracking", { tracking });
  },
  clients(includeArchived = false) {
    return invoke<Client[]>("tracking_clients", { includeArchived });
  },
  /** Cria quando `id` vem vazio, atualiza quando vem preenchido. */
  saveClient(id: string | null, input: ClientInput) {
    return invoke<Client>("tracking_save_client", { id, input });
  },
  setClientArchived(id: string, archived: boolean) {
    return invoke<Client>("tracking_set_client_archived", { id, archived });
  },
  /** O lembrete que a janelinha deve mostrar, se houver. */
  reminderPending() {
    return invoke<PendingReminder | null>("reminder_pending");
  },
  /** Fecha sem decidir nada. "Agora não" é uma resposta legítima. */
  reminderDismiss() {
    return invoke<void>("reminder_dismiss");
  },
  /**
   * Silencia um programa até o instante dado.
   *
   * O instante é calculado aqui porque "hoje" acaba à meia-noite LOCAL, e o
   * backend só conhece UTC.
   */
  reminderSuppress(processName: string, until: string) {
    return invoke<void>("reminder_suppress", { processName, until });
  },
  /** Os programas silenciados agora — para a tela poder mostrar e desfazer. */
  reminderSilenced() {
    return invoke<SilencedApp[]>("reminder_silenced");
  },
  reminderUnsilence(processName: string) {
    return invoke<void>("reminder_unsilence", { processName });
  },
  monitoringSettings() {
    return invoke<MonitoringSettings>("monitoring_settings");
  },
  /** Vale na próxima passada do laço — desligar tem efeito em segundos. */
  monitoringSetSettings(settings: MonitoringSettings) {
    return invoke<MonitoringSettings>("monitoring_set_settings", { settings });
  },
  monitoredApps() {
    return invoke<MonitoredApp[]>("monitoring_apps");
  },
  saveMonitoredApp(entry: MonitoredApp) {
    return invoke<MonitoredApp>("monitoring_save_app", { entry });
  },
  deleteMonitoredApp(id: string) {
    return invoke<void>("monitoring_delete_app", { id });
  },
  /** A janela é obrigatória: a Linha do Tempo é sempre sobre um período. */
  activityEvents(since: string, until: string) {
    return invoke<ActivityEvent[]>("monitoring_events", { since, until });
  },
  /** Só os períodos SEM sessão registrada — o resto seria contar duas vezes. */
  monitoringTimeline(since: string, until: string) {
    return invoke<Period[]>("monitoring_timeline", { since, until });
  },
  /** Entra como `reconstructed`: proposta pelo sistema, aceita por você. */
  recordFromTimeline(projectId: string, since: string, until: string, activityType: ActivityType) {
    return invoke<TimeEntry>("tracking_record_from_timeline", { projectId, since, until, activityType });
  },
  markActivityProcessed(id: string) {
    return invoke<void>("monitoring_mark_processed", { id });
  },
  timerDiscard() {
    return invoke<void>("timer_discard");
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
