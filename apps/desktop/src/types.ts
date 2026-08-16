export type CaptureSource = "home" | "quick_capture";
export type ProcessingState = "inbox" | "processed";
export type LifecycleState = "active" | "archived" | "trashed";

export type Capture = {
  id: string;
  content: string;
  source: CaptureSource;
  capturedAt: string;
  updatedAt: string;
  processingState: ProcessingState;
  lifecycleState: LifecycleState;
};

export type Project = {
  id: string;
  name: string;
  description: string;
  /** Vazio significa sem repositorio. */
  repository: string;
  lifecycleState: LifecycleState;
  createdAt: string;
  updatedAt: string;
};

export type Workspace = {
  id: string;
  name: string;
  description: string;
  lifecycleState: LifecycleState;
  createdAt: string;
  updatedAt: string;
};

/** Uma entrada significa OCULTO — ausencia e o padrao visivel. Ver migration 0008. */
export type HiddenWidget = {
  workspaceId: string;
  widgetId: string;
};

/** A ordem e a ordem das colunas do kanban.
 *  `inbox` aqui nao e a Inbox de Captures — Capture tem processingState. */
export type TaskState = "inbox" | "backlog" | "planned" | "doing" | "review" | "done";

export type ResourceKind = "site" | "library" | "image" | "note";

export type Task = {
  id: string;
  title: string;
  description: string;
  projectId: string | null;
  sourceCaptureId: string | null;
  state: TaskState;
  lifecycleState: LifecycleState;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
};

export type AppLaunchKind = "url" | "path";

export type RegisteredApp = {
  id: string;
  name: string;
  description: string;
  sourceUrl: string | null;
  launchKind: AppLaunchKind | null;
  launchTarget: string | null;
  /** Capacidade nao declarada e capacidade que o Hermes nao tenta usar. */
  canOpen: boolean;
  canRead: boolean;
  canWrite: boolean;
  canAutomate: boolean;
  lifecycleState: LifecycleState;
  createdAt: string;
  updatedAt: string;
  lastOpenedAt: string | null;
};

export type AppCapabilities = {
  canOpen: boolean;
  canRead: boolean;
  canWrite: boolean;
  canAutomate: boolean;
};

export type Resource = {
  id: string;
  kind: ResourceKind;
  title: string;
  url: string;
  note: string;
  sourceCaptureId: string | null;
  lifecycleState: LifecycleState;
  createdAt: string;
  updatedAt: string;
};

/** Um par significa: este Resource pertence a este contexto. */
export type ResourceWorkspace = {
  resourceId: string;
  workspaceId: string;
};

export type AppCatalogEntry = {
  id: string;
  name: string;
  description: string;
  sourceUrl: string;
  launchKind: AppLaunchKind | null;
  launchTarget: string | null;
};

export type SearchItem =
  | { kind: "capture"; capture: Capture; derivedTask: Task | null; project: Project | null }
  | { kind: "task"; task: Task; project: Project | null }
  | { kind: "project"; project: Project }
  | { kind: "workspace"; workspace: Workspace }
  | { kind: "app"; app: RegisteredApp }
  | { kind: "resource"; resource: Resource };

export type FunctionCategory = "capture" | "work" | "memory" | "app" | "data" | "system";
export type FunctionRisk = "low" | "medium" | "high";
export type FunctionConfirmation = "none" | "explicit";

export type FunctionDefinition = {
  id: string;
  name: string;
  description: string;
  category: FunctionCategory;
  risk: FunctionRisk;
  confirmation: FunctionConfirmation;
};

export type AppError = {
  code: string;
  message: string;
  retryable: boolean;
};

export type AppStatus = {
  inboxCount: number;
  projectCount: number;
  taskCount: number;
  appCount: number;
  resourceCount: number;
  workspaceCount: number;
  shortcut: string;
  snapshot: string;
  storage: {
    databasePath: string;
    schemaVersion: number;
    journalMode: string;
    synchronous: string;
    integrity: string;
  };
};

export type BackupReceipt = {
  path: string;
  bytes: number;
  createdAt: string;
};

/**
 * O que a importação do CronoCAD trouxe.
 *
 * `trackedSeconds` existe para ser comparado com a tela do CronoCAD: é o número
 * que diz se as horas chegaram inteiras, e portanto se dá para desinstalar.
 */
export type ActivityType = "drawing" | "detailing" | "revision" | "meeting" | "study" | "other";

/**
 * O cronômetro em curso.
 *
 * Vem cru — acumulado mais a marca do último resume — e não como segundos já
 * prontos. É o que permite a tela desenhar o relógio correndo sozinha, sem o
 * backend precisar emitir um evento por segundo.
 */
export type ActiveTimer = {
  projectId: string;
  startedAt: string;
  lastResumedAt: string;
  accumulatedSeconds: number;
  status: "running" | "paused";
  description: string;
  activityType: ActivityType;
};

export type TimeEntry = {
  id: string;
  projectId: string;
  startedAt: string;
  endedAt: string | null;
  durationSeconds: number;
  idleSeconds: number;
  description: string;
  activityType: ActivityType;
  billable: boolean;
  hourlyRateSnapshotCents: number;
  source: "timer" | "manual" | "reconstructed";
};

/** Quem paga pelo trabalho. Um Project pessoal simplesmente não tem. */
export type Client = {
  id: string;
  name: string;
  companyName: string;
  email: string;
  phone: string;
  notes: string;
  archived: boolean;
};

export type ClientInput = {
  name: string;
  companyName: string;
  email: string;
  phone: string;
  notes: string;
};

/**
 * Um programa cuja abertura sugere trabalho.
 *
 * `processName` é a chave real: o monitoramento casa por ele, e o backend
 * normaliza para minúsculas porque o Windows não diferencia.
 */
export type MonitoredApp = {
  id: string;
  displayName: string;
  processName: string;
  enabled: boolean;
  remindOnOpen: boolean;
  remindOnClose: boolean;
};

export type ActivityKind =
  | "app_opened"
  | "app_closed"
  | "idle_started"
  | "idle_ended"
  | "timer_started"
  | "timer_paused"
  | "timer_resumed"
  | "timer_stopped";

/** O que o sistema observou. Observação não vira hora sozinha. */
export type ActivityEvent = {
  id: string;
  kind: ActivityKind;
  processName: string;
  detectedAt: string;
  /** Já virou sessão ou já foi descartado — não é reoferecido. */
  processed: boolean;
};

export type TrackingSettings = {
  rounding: { enabled: boolean; intervalMinutes: number; mode: "nearest" | "up" | "down" };
  idleThresholdMinutes: number;
};

/**
 * O que uma correção pode mudar numa sessão.
 *
 * Sem `projectId` e sem a taxa, e as duas ausências são a mesma decisão: a taxa
 * é o registro do que valia quando o trabalho aconteceu, e reescrevê-la — direto
 * ou de lado, movendo o Project — pode alterar um valor já faturado.
 */
export type TimeEntryEdit = {
  startedAt: string;
  durationSeconds: number;
  idleSeconds: number;
  description: string;
  activityType: ActivityType;
  billable: boolean;
};

/**
 * Totais de um Project.
 *
 * `grossSeconds` é o tempo REAL registrado. `billableSeconds` já passou pelo
 * desconto de inatividade e pelo arredondamento configurado — os dois números
 * existem lado a lado porque são perguntas diferentes: quanto eu trabalhei, e
 * quanto eu cobro.
 */
export type Totals = {
  grossSeconds: number;
  idleSeconds: number;
  billableSeconds: number;
  amountCents: number;
};

export type ImportReport = {
  projects: number;
  entries: number;
  tasks: number;
  trackedSeconds: number;
  monitoredApps: number;
  /** O histórico observado pelo sistema, que alimenta a Linha do Tempo. */
  activityEvents: number;
  clients: number;
};

export type BackupInspection = BackupReceipt & {
  schemaVersion: number;
  captureCount: number;
};

export type UpdateInfo = {
  currentVersion: string;
  version: string;
  date: string | null;
  body: string;
};

export type UpdateProgress = {
  downloaded: number;
  total: number | null;
};
