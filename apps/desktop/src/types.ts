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
/* A LINHA SIGNIFICA OCULTO — ausencia dela significa visivel. E `workspaceId`
   nulo e a visao "Todos", que esconde os proprios widgets desde a migration
   0019, e nao um dado faltando. */
export type HiddenWidget = {
  workspaceId: string | null;
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

export type FunctionCategory = "capture" | "work" | "time" | "attention" | "memory" | "app" | "data" | "system";
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

/**
 * Os dados de cobrança de um Project.
 *
 * Separado de `Project` porque a maioria dos Projects do M/OS nunca vai ter
 * valor/hora nem cliente — e um campo nulo em toda linha ensina que ele é
 * opcional quando na verdade ele pertence a outro domínio.
 *
 * `trackingStatus` tem quatro estados e o `lifecycleState` do M/OS tem três,
 * porque nenhum dos três significa "concluído". Um projeto que terminou não é a
 * mesma coisa que um projeto arquivado por desuso, e a diferença importa para
 * quem fatura.
 */
export type TrackingStatus = "active" | "paused" | "completed" | "archived";

export type ProjectTracking = {
  projectId: string;
  hourlyRateCents: number;
  code: string;
  color: string;
  trackingStatus: TrackingStatus;
  clientId: string | null;
  /** Meta de horas, em minutos. Zero significa "sem meta". */
  budgetMinutes: number;
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

/**
 * Como o sistema observa.
 *
 * Separada de `TrackingSettings` porque responde outra pergunta: uma diz como o
 * tempo vira dinheiro, esta diz o quanto o aplicativo olha por cima do ombro.
 */
export type MonitoringSettings = {
  processMonitoringEnabled: boolean;
  checkIntervalSeconds: number;
  idleDetectionEnabled: boolean;
  idleThresholdMinutes: number;
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

/**
 * O que a janelinha de lembrete precisa saber para se desenhar.
 *
 * `opened` distingue as duas perguntas que ela faz: abriu o CAD sem cronômetro
 * ("começo a contar?") ou fechou com o cronômetro rodando ("paro de contar?").
 */
export type PendingReminder = {
  processName: string;
  displayName: string;
  opened: boolean;
  hasActiveTimer: boolean;
};

/**
 * Um programa cujo lembrete está silenciado agora.
 *
 * O silêncio vive em memória: reiniciar o M/OS devolve todos os lembretes. É a
 * escolha certa para uma decisão que se chama "hoje".
 */
export type SilencedApp = {
  processName: string;
  minutesLeft: number;
};

/** Um intervalo fechado. A Linha do Tempo devolve os que não têm sessão. */
export type Period = { start: string; end: string };

/**
 * O limiar de inatividade mora em `MonitoringSettings`, e não aqui: ele decide
 * quando o sistema considera que você parou, não quanto disso é cobrado. Com os
 * dois tipos gravando na mesma coluna, salvar um desfazia o outro.
 */
export type TrackingSettings = {
  rounding: { enabled: boolean; intervalMinutes: number; mode: "nearest" | "up" | "down" };
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

/**
 * Uma linha do relatório: a sessão, mais o que ela vale.
 *
 * `totals` chega calculado pelo backend — mesmo desconto de inatividade, mesmo
 * arredondamento que o Painel usa. A tela agrupa e formata; não recalcula.
 */
export type ReportLine = {
  entryId: string;
  projectId: string;
  startedAt: string;
  activityType: ActivityType;
  source: "timer" | "manual" | "reconstructed";
  billable: boolean;
  description: string;
  hourlyRateSnapshotCents: number;
  totals: Totals;
  /**
   * O valor do tempo REAL, sem arredondar.
   *
   * O Histórico mostra este; o Relatório mostra `totals.amountCents`. Com
   * arredondamento ligado os dois divergem de propósito — e é o registro real
   * que o histórico existe para guardar.
   */
  rawAmountCents: number;
};

/** Quem está cobrando. Sai no cabeçalho da fatura, e em nenhum outro lugar. */
export type Issuer = {
  name: string;
  document: string;
  contact: string;
};

/**
 * O PDF recebe as células JÁ FORMATADAS.
 *
 * Formatar em dois lugares acabaria com a fatura dizendo um número e a tela
 * dizendo outro — e quem descobriria seria o cliente.
 */
export type ReportPdfData = {
  title: string;
  period: string;
  totals: [string, string][];
  columns: [string, string, string, string];
  rows: [string, string, string, string][];
};

export type InvoiceData = {
  issuerName: string;
  issuerDocument: string;
  issuerContact: string;
  clientName: string;
  period: string;
  columns: [string, string, string, string];
  rows: [string, string, string, string][];
  totalLabel: string;
  totalValue: string;
};

export type CalendarKind = "session" | "task_done" | "task_created" | "capture" | "app_opened";

/**
 * Um item de calendário: algo que o M/OS registrou, com hora.
 *
 * `at` vem em **UTC**. Que dia isso é decide-se no renderer, que é o único dos
 * dois lados que conhece o fuso de quem está olhando.
 */
export type CalendarItem = {
  kind: CalendarKind;
  at: string;
  endsAt: string | null;
  title: string;
  projectId: string | null;
  seconds: number;
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

export type ReminderStatus =
  | "scheduled"
  | "due"
  | "delivered"
  | "acknowledged"
  | "snoozed"
  | "completed"
  | "cancelled"
  | "missed"
  | "expired";

export type ReminderPriority = "low" | "normal" | "high" | "urgent";

export type ReminderTarget = {
  type: "task" | "project" | "capture" | "resource" | "conversation" | "app";
  id: string;
};

export type Reminder = {
  id: string;
  title: string;
  body: string;
  target: ReminderTarget | null;
  trigger: { kind: "at"; instant: string };
  priority: ReminderPriority;
  status: ReminderStatus;
  policy: { snoozeAllowed: boolean; privacy: "show_content" | "title_only" | "hidden" };
  source: "user" | "hermes" | "capture" | "system";
  nextDueAt: string | null;
  snoozeCount: number;
  deliveredCount: number;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  lifecycleState: "active" | "archived" | "trashed";
};

/** O que o agendador manda quando algo precisa aparecer. */
export type DeliveryEvent = {
  reminderId: string;
  title: string;
  body: string;
  missed: boolean;
  overdueSeconds: number;
  level: string;
};

/* Onde um widget foi posto na Home de um Workspace.

   `section` e `span` sao anulaveis, e o `null` e a parte que importa: dentro de
   uma linha que existe, campo vazio significa "o que o desenho escolheu". Ver a
   migration 0017 — mandar o valor efetivo em toda reordenacao petrificaria o
   desenho de hoje no primeiro arrasto. */
export type WidgetPlacement = {
  /* Nulo e a visao "Todos", e nao um dado faltando: ela e um contexto de
     verdade — o unico de quem nunca criou Workspace — e arruma a propria Home
     desde a migration 0018. */
  workspaceId: string | null;
  widgetId: string;
  position: number;
  section: string | null;
  span: number | null;
};

/* O que se manda para gravar. Sem o Workspace, que vem por fora porque a
   escrita inteira e de um so. */
export type WidgetPlacementInput = {
  widgetId: string;
  position: number;
  section: string | null;
  span: number | null;
};
