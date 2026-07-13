/**
 * Contratos de eventos entre backend Tauri e frontend (secao 21).
 *
 * Os nomes dos eventos sao centralizados para evitar strings soltas. Os
 * payloads sao tipados para garantir consistencia na camada de servicos.
 *
 * Nesta fundacao, apenas os contratos existem; a emissao real sera ligada
 * nas fases de cronometro, monitoramento e inatividade.
 */

import type { ActivityType, TimerStatus } from "@/types/domain";

export const TauriEvent = {
  timerStateChanged: "timer-state-changed",
  monitoredAppOpened: "monitored-app-opened",
  monitoredAppClosed: "monitored-app-closed",
  idleStarted: "idle-started",
  idleEnded: "idle-ended",
  databaseUpdated: "database-updated",
  requestQuit: "request-quit",
} as const;

export type TauriEventName = (typeof TauriEvent)[keyof typeof TauriEvent];

export interface TimerStateChangedPayload {
  active: boolean;
  projectId: string | null;
  status: TimerStatus | null;
  activityType: ActivityType | null;
}

export interface MonitoredAppPayload {
  processName: string;
  displayName: string;
  hasActiveTimer: boolean;
}

export interface IdleStartedPayload {
  idleSeconds: number;
}

export interface IdleEndedPayload {
  idleSeconds: number;
  hasActiveTimer: boolean;
}

/** Mapa nome-do-evento -> tipo-do-payload, para tipagem do listener. */
export interface TauriEventPayloadMap {
  [TauriEvent.timerStateChanged]: TimerStateChangedPayload;
  [TauriEvent.monitoredAppOpened]: MonitoredAppPayload;
  [TauriEvent.monitoredAppClosed]: MonitoredAppPayload;
  [TauriEvent.idleStarted]: IdleStartedPayload;
  [TauriEvent.idleEnded]: IdleEndedPayload;
  [TauriEvent.databaseUpdated]: { table: string };
  [TauriEvent.requestQuit]: null;
}
