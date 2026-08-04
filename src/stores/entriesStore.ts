/**
 * Store das sessoes registradas (time_entries) — visao padrao (nao excluidas),
 * lida do backend. Compartilhado por Painel, Historico e Relatorios. As acoes
 * de escrita recarregam a lista para manter todas as telas coerentes.
 */

import { create } from "zustand";
import type { TimeEntry } from "@/types/domain";
import {
  createTimeEntry,
  deleteTimeEntry,
  listTimeEntries,
  restoreTimeEntry,
  updateTimeEntry,
  type EntryUpdateInput,
  type ManualEntryInput,
} from "@/services/timeEntries";

interface EntriesState {
  entries: TimeEntry[];
  loaded: boolean;
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  create: (input: ManualEntryInput) => Promise<void>;
  update: (id: string, input: EntryUpdateInput) => Promise<void>;
  remove: (id: string) => Promise<void>;
  restore: (id: string) => Promise<void>;
}

function messageOf(err: unknown): string {
  return typeof err === "string"
    ? err
    : err instanceof Error
      ? err.message
      : String(err);
}

export const useEntriesStore = create<EntriesState>((set, get) => ({
  entries: [],
  loaded: false,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      // Sem limite: Painel e Relatorio somam este array para calcular valores.
      const entries = await listTimeEntries(undefined, false);
      set({ entries, loading: false, loaded: true });
    } catch (err) {
      set({ loading: false, loaded: true, error: messageOf(err) });
    }
  },

  create: async (input) => {
    await createTimeEntry(input);
    await get().load();
  },
  update: async (id, input) => {
    await updateTimeEntry(id, input);
    await get().load();
  },
  remove: async (id) => {
    await deleteTimeEntry(id);
    await get().load();
  },
  restore: async (id) => {
    await restoreTimeEntry(id);
    await get().load();
  },
}));
