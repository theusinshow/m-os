/**
 * Store do cronometro.
 *
 * O backend e a fonte da verdade: cada acao chama um comando, e o estado local
 * reflete o resultado. O estado tambem e re-sincronizado ao receber o evento
 * `timer-state-changed` (ex.: acoes disparadas pela bandeja).
 *
 * Recuperacao (secao 9): ao iniciar o app, se houver um cronometro remanescente,
 * marca-se `recoveryPending` para que a UI ofereca manter/encerrar/descartar —
 * nunca decidindo silenciosamente.
 */

import { create } from "zustand";
import type { ActiveTimer } from "@/types/domain";
import {
  discardTimer,
  discountIdle,
  getActiveTimer,
  pauseTimer,
  resumeTimer,
  startTimer,
  stopTimer,
  type StartTimerInput,
} from "@/services/timer";
import { useEntriesStore } from "./entriesStore";

interface TimerState {
  activeTimer: ActiveTimer | null;
  loaded: boolean;
  error: string | null;
  recoveryPending: boolean;

  loadActive: (isStartup?: boolean) => Promise<void>;
  start: (input: StartTimerInput) => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  stop: () => Promise<void>;
  discard: () => Promise<void>;
  discountIdle: (seconds: number) => Promise<void>;
  dismissRecovery: () => void;
}

function messageOf(err: unknown): string {
  return typeof err === "string"
    ? err
    : err instanceof Error
      ? err.message
      : String(err);
}

export const useTimerStore = create<TimerState>((set) => ({
  activeTimer: null,
  loaded: false,
  error: null,
  recoveryPending: false,

  loadActive: async (isStartup = false) => {
    try {
      const activeTimer = await getActiveTimer();
      set({
        activeTimer,
        loaded: true,
        error: null,
        // So oferece recuperacao quando o app abre ja com um cronometro.
        recoveryPending: isStartup && activeTimer !== null,
      });
    } catch (err) {
      set({ loaded: true, error: messageOf(err) });
    }
  },

  start: async (input) => {
    const activeTimer = await startTimer(input);
    set({ activeTimer, error: null });
  },

  pause: async () => {
    const activeTimer = await pauseTimer();
    set({ activeTimer });
  },

  resume: async () => {
    const activeTimer = await resumeTimer();
    set({ activeTimer });
  },

  stop: async () => {
    await stopTimer();
    set({ activeTimer: null, recoveryPending: false });
    await useEntriesStore.getState().load();
  },

  discard: async () => {
    await discardTimer();
    set({ activeTimer: null, recoveryPending: false });
  },

  discountIdle: async (seconds) => {
    const activeTimer = await discountIdle(seconds);
    set({ activeTimer });
  },

  dismissRecovery: () => set({ recoveryPending: false }),
}));
