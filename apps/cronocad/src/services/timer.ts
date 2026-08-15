/**
 * Servico do cronometro: wrappers tipados sobre os comandos Tauri.
 * A fonte da verdade e o backend (secao 21); estes wrappers apenas transportam.
 */

import type { ActiveTimer, ActivityType, TimeEntry } from "@/types/domain";
import { invokeCommand } from "./tauri";

export interface StartTimerInput {
  projectId: string;
  activityType: ActivityType;
  description?: string | null;
}

export function getActiveTimer(): Promise<ActiveTimer | null> {
  return invokeCommand<ActiveTimer | null>("get_active_timer");
}

export function startTimer(input: StartTimerInput): Promise<ActiveTimer> {
  return invokeCommand<ActiveTimer>("start_timer", { input });
}

export function pauseTimer(): Promise<ActiveTimer> {
  return invokeCommand<ActiveTimer>("pause_timer");
}

export function resumeTimer(): Promise<ActiveTimer> {
  return invokeCommand<ActiveTimer>("resume_timer");
}

export function stopTimer(): Promise<TimeEntry> {
  return invokeCommand<TimeEntry>("stop_timer");
}

export function discardTimer(): Promise<void> {
  return invokeCommand<void>("discard_timer");
}

/** Desconta tempo inativo (segundos) do cronometro ativo (secao 11). */
export function discountIdle(seconds: number): Promise<ActiveTimer> {
  return invokeCommand<ActiveTimer>("discount_idle", { seconds });
}
