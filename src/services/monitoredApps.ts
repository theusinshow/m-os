/**
 * Servico de programas monitorados (secao 10): CRUD e supressao de lembrete.
 */

import type { MonitoredApp } from "@/types/domain";
import { invokeCommand } from "./tauri";

export interface MonitoredAppInput {
  displayName: string;
  processName: string;
  enabled: boolean;
  remindOnOpen: boolean;
  remindOnClose: boolean;
}

export function listMonitoredApps(): Promise<MonitoredApp[]> {
  return invokeCommand<MonitoredApp[]>("list_monitored_apps");
}

export function createMonitoredApp(
  input: MonitoredAppInput,
): Promise<MonitoredApp> {
  return invokeCommand<MonitoredApp>("create_monitored_app", { input });
}

export function updateMonitoredApp(
  id: string,
  input: MonitoredAppInput,
): Promise<MonitoredApp> {
  return invokeCommand<MonitoredApp>("update_monitored_app", { id, input });
}

export function deleteMonitoredApp(id: string): Promise<void> {
  return invokeCommand<void>("delete_monitored_app", { id });
}

/** Suprime o lembrete de abertura de um processo pelo resto do dia. */
export function suppressAppReminderToday(processName: string): Promise<void> {
  return invokeCommand<void>("suppress_app_reminder_today", { processName });
}
