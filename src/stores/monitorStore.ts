/**
 * Store dos lembretes de monitoramento (secao 10).
 *
 * Alimentado pelos eventos `monitored-app-opened`/`monitored-app-closed`. A UI
 * decide a acao (nunca vincula projeto nem encerra silenciosamente).
 */

import { create } from "zustand";

export interface MonitorPrompt {
  processName: string;
  displayName: string;
}

interface MonitorState {
  /** Programa monitorado fechado com cronometro ativo, aguardando decisao. */
  closeApp: MonitorPrompt | null;
  /** Periodo de inatividade detectado, aguardando decisao (segundos). */
  idleSeconds: number | null;
  /** Pedido de saida com cronometro ativo, aguardando confirmacao. */
  quitRequested: boolean;
  setClose: (prompt: MonitorPrompt) => void;
  setIdle: (idleSeconds: number) => void;
  requestQuit: () => void;
  clearClose: () => void;
  clearIdle: () => void;
  clearQuit: () => void;
}

export const useMonitorStore = create<MonitorState>((set) => ({
  closeApp: null,
  idleSeconds: null,
  quitRequested: false,
  setClose: (closeApp) => set({ closeApp }),
  setIdle: (idleSeconds) => set({ idleSeconds }),
  requestQuit: () => set({ quitRequested: true }),
  clearClose: () => set({ closeApp: null }),
  clearIdle: () => set({ idleSeconds: null }),
  clearQuit: () => set({ quitRequested: false }),
}));
