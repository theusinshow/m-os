/**
 * Tipos de dominio do CronoCAD.
 *
 * Espelham o modelo de dados (secao 8 do documento). Timestamps sao strings
 * ISO 8601 em UTC produzidas pelo backend; valores monetarios sao inteiros em
 * centavos; duracoes sao inteiros em segundos.
 */

export type Uuid = string;
/** Timestamp ISO 8601 em UTC, ex.: "2026-07-11T13:34:00Z". */
export type IsoDateTime = string;
/** Valor monetario inteiro, em centavos. */
export type Cents = number;
/** Duracao inteira, em segundos. */
export type Seconds = number;

export type ProjectStatus = "active" | "paused" | "completed" | "archived";

export type TimeEntrySource = "timer" | "manual" | "reconstructed";

export type ActivityType =
  | "drawing"
  | "detailing"
  | "revision"
  | "meeting"
  | "study"
  | "other";

export type TimerStatus = "running" | "paused";

export type ActivityEventType =
  | "app_opened"
  | "app_closed"
  | "idle_started"
  | "idle_ended"
  | "timer_started"
  | "timer_paused"
  | "timer_resumed"
  | "timer_stopped";

export interface Client {
  id: Uuid;
  name: string;
  companyName: string | null;
  email: string | null;
  phone: string | null;
  notes: string | null;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
  archivedAt: IsoDateTime | null;
}

export interface Project {
  id: Uuid;
  clientId: Uuid | null;
  name: string;
  code: string | null;
  description: string | null;
  hourlyRateCents: Cents;
  /** Meta de horas do projeto, em minutos (0 = sem meta). */
  budgetMinutes: number;
  status: ProjectStatus;
  color: string | null;
  /** Bloco de anotacoes livres do projeto (null quando vazio). */
  notes: string | null;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
  archivedAt: IsoDateTime | null;
}

/**
 * Pendencia de um projeto (checklist). Sem data e sem notificacao: um lembrete
 * aqui apenas fica visivel — no modal do projeto e no Painel.
 */
export interface ProjectTodo {
  id: Uuid;
  projectId: Uuid;
  text: string;
  done: boolean;
  doneAt: IsoDateTime | null;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

/**
 * Horas e valor acumulados de um projeto ao longo de toda a sua vida.
 *
 * Calculado no backend sobre TODO o historico (nao sobre a lista carregada na
 * tela). `billableSeconds` e `amountCents` ja descontam inatividade, zeram
 * sessoes nao-faturaveis e aplicam o arredondamento das Configuracoes — sempre
 * na visualizacao/cobranca, nunca no banco.
 */
export interface ProjectBilling {
  projectId: Uuid;
  grossSeconds: Seconds;
  idleSeconds: Seconds;
  billableSeconds: Seconds;
  amountCents: Cents;
}

export interface TimeEntry {
  id: Uuid;
  projectId: Uuid;
  startedAt: IsoDateTime;
  endedAt: IsoDateTime | null;
  durationSeconds: Seconds;
  idleSeconds: Seconds;
  description: string | null;
  activityType: ActivityType;
  billable: boolean;
  hourlyRateSnapshotCents: Cents;
  source: TimeEntrySource;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
  deletedAt: IsoDateTime | null;
}

/** Estado persistente do unico cronometro ativo (ou null se nenhum). */
export interface ActiveTimer {
  id: Uuid;
  projectId: Uuid;
  startedAt: IsoDateTime;
  lastResumedAt: IsoDateTime;
  accumulatedSeconds: Seconds;
  status: TimerStatus;
  description: string | null;
  activityType: ActivityType;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface MonitoredApp {
  id: Uuid;
  displayName: string;
  processName: string;
  enabled: boolean;
  remindOnOpen: boolean;
  remindOnClose: boolean;
  createdAt: IsoDateTime;
  updatedAt: IsoDateTime;
}

export interface ActivityEvent {
  id: Uuid;
  eventType: ActivityEventType;
  processName: string | null;
  detectedAt: IsoDateTime;
  metadataJson: string | null;
  processed: boolean;
  createdAt: IsoDateTime;
}

export type RoundingMode = "nearest" | "up" | "down";

export interface Settings {
  idleDetectionEnabled: boolean;
  idleThresholdMinutes: number;
  processMonitoringEnabled: boolean;
  processCheckIntervalSeconds: number;
  remindWhenMonitoredAppOpens: boolean;
  remindWhenMonitoredAppCloses: boolean;
  roundingEnabled: boolean;
  roundingIntervalMinutes: number;
  roundingMode: RoundingMode;
  startWithWindows: boolean;
  minimizeToTray: boolean;
  closeToTray: boolean;
  currency: string;
  locale: string;
  issuerName: string;
  issuerDocument: string;
  issuerContact: string;
}
