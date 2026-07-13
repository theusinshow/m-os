/**
 * Store de configuracoes e programas monitorados.
 */

import { create } from "zustand";
import type { MonitoredApp, Settings } from "@/types/domain";
import { getSettings, updateSettings } from "@/services/settings";
import {
  createMonitoredApp,
  deleteMonitoredApp,
  listMonitoredApps,
  updateMonitoredApp,
  type MonitoredAppInput,
} from "@/services/monitoredApps";

interface SettingsState {
  settings: Settings | null;
  apps: MonitoredApp[];
  loaded: boolean;
  error: string | null;

  load: () => Promise<void>;
  saveSettings: (settings: Settings) => Promise<void>;
  addApp: (input: MonitoredAppInput) => Promise<void>;
  editApp: (id: string, input: MonitoredAppInput) => Promise<void>;
  removeApp: (id: string) => Promise<void>;
}

function messageOf(err: unknown): string {
  return typeof err === "string"
    ? err
    : err instanceof Error
      ? err.message
      : String(err);
}

const byName = (a: MonitoredApp, b: MonitoredApp) =>
  a.displayName.localeCompare(b.displayName, "pt-BR");

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  apps: [],
  loaded: false,
  error: null,

  load: async () => {
    try {
      const [settings, apps] = await Promise.all([
        getSettings(),
        listMonitoredApps(),
      ]);
      set({ settings, apps: [...apps].sort(byName), loaded: true, error: null });
    } catch (err) {
      set({ loaded: true, error: messageOf(err) });
    }
  },

  saveSettings: async (settings) => {
    const saved = await updateSettings(settings);
    set({ settings: saved });
  },

  addApp: async (input) => {
    const app = await createMonitoredApp(input);
    set({ apps: [...get().apps, app].sort(byName) });
  },

  editApp: async (id, input) => {
    const updated = await updateMonitoredApp(id, input);
    set({
      apps: get()
        .apps.map((a) => (a.id === id ? updated : a))
        .sort(byName),
    });
  },

  removeApp: async (id) => {
    await deleteMonitoredApp(id);
    set({ apps: get().apps.filter((a) => a.id !== id) });
  },
}));
