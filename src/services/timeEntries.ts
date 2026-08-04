/**
 * Servico de sessoes (time_entries): leitura, criacao manual, edicao, exclusao
 * reversivel (soft delete), restauracao e exportacao de texto (CSV).
 */

import type { ActivityType, TimeEntry } from "@/types/domain";
import { invokeCommand } from "./tauri";

export interface ManualEntryInput {
  projectId: string;
  startedAt: string;
  endedAt: string;
  description?: string | null;
  activityType: ActivityType;
  billable: boolean;
  idleSeconds: number;
  /** "manual" (padrao) ou "reconstructed" (linha do tempo). */
  source?: "manual" | "reconstructed";
}

export interface EntryUpdateInput {
  startedAt: string;
  endedAt: string;
  description?: string | null;
  activityType: ActivityType;
  billable: boolean;
  idleSeconds: number;
}

/**
 * Sessoes da mais nova para a antiga. `limit` omitido = historico inteiro:
 * telas que somam valores (Painel, Relatorio, Historico) nao podem ter teto,
 * senao o total exibido fica menor que a realidade sem nenhum aviso.
 */
export function listTimeEntries(
  limit?: number,
  includeDeleted = false,
): Promise<TimeEntry[]> {
  return invokeCommand<TimeEntry[]>("list_time_entries", {
    limit,
    includeDeleted,
  });
}

export function createTimeEntry(input: ManualEntryInput): Promise<TimeEntry> {
  return invokeCommand<TimeEntry>("create_time_entry", { input });
}

export function updateTimeEntry(
  id: string,
  input: EntryUpdateInput,
): Promise<TimeEntry> {
  return invokeCommand<TimeEntry>("update_time_entry", { id, input });
}

export function deleteTimeEntry(id: string): Promise<void> {
  return invokeCommand<void>("delete_time_entry", { id });
}

export function restoreTimeEntry(id: string): Promise<TimeEntry> {
  return invokeCommand<TimeEntry>("restore_time_entry", { id });
}

/** Abre o dialogo nativo para salvar um texto (CSV). Retorna true se salvou. */
export function saveTextFile(
  content: string,
  suggestedName: string,
): Promise<boolean> {
  return invokeCommand<boolean>("save_text_file", { content, suggestedName });
}
