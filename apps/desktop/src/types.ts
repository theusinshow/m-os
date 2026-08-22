export type CaptureSource = "home" | "quick_capture" | "drop" | "voice";
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

export type ResourceKind = "site" | "library" | "image" | "note" | "file";

// ===========================================================================
// Universal Drop Zone
// ===========================================================================
//
// A ingestao e o registro do que entrou, por onde, o que virou e onde parou.
// Ela nao substitui o Resource nem a Capture: ela os explica.

export type IngestionSource = "drop_file" | "drop_text" | "drop_url";

export type DetectedKind =
  | "pdf"
  | "image"
  | "text"
  | "markdown"
  | "data"
  | "code"
  | "archive"
  | "url"
  | "unknown";

export type IngestionState =
  | "receiving"
  | "preserved"
  | "completed"
  | "interrupted"
  | "failed"
  | "undone";

export type ExtractionState = "pending" | "done" | "empty" | "unsupported" | "failed";

/** De onde a pessoa estava olhando quando soltou. */
export type DropContext = {
  page: string;
  projectId: string | null;
  workspaceId: string | null;
  taskId: string | null;
};

export type Ingestion = {
  id: string;
  source: IngestionSource;
  originalName: string;
  mime: string;
  byteSize: number;
  sha256: string;
  storedPath: string;
  detectedKind: DetectedKind;
  state: IngestionState;
  failure: string;
  captureId: string | null;
  resourceId: string | null;
  duplicateOf: string | null;
  context: DropContext;
  suggestedProjectId: string | null;
  relationConfidence: number;
  relationReason: string;
  extractionState: ExtractionState;
  extractionError: string;
  pageCount: number | null;
  imageSize: { width: number; height: number } | null;
  createdAt: string;
  updatedAt: string;
};

export type IngestionReceipt = {
  ingestion: Ingestion;
  /** O conteudo ja estava no M/OS; o contexto novo foi aplicado no que existia. */
  duplicate: boolean;
  /** Rotulo curto do destino, para o recibo. */
  destination: string;
};

/** O estado de um item do lote NA TELA — nao e o estado persistido. */
export type IngestionStatus =
  | "esperando"
  | "lendo"
  | "entendendo"
  | "guardado"
  | "repetido"
  | "erro"
  | "desfeito";

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
  | { kind: "resource"; resource: Resource }
  /* O `day` viaja junto porque um objetivo sem data não se distingue de outro:
     dois dias podem ter escrito a mesma frase, e a data é o que faz o resultado
     significar alguma coisa. Ver `SearchItem::DailyObjective` no core. */
  | { kind: "daily_objective"; objective: DailyObjective; day: Day };

/** Espelha `FunctionCategory` em `crates/mos-core/src/functions.rs`. */
export type FunctionCategory = "capture" | "daily" | "work" | "time" | "attention" | "meeting" | "memory" | "app" | "data" | "system";
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
  /** O estado do registro do atalho da voz. */
  voiceShortcut: string;
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
  /**
   * Quando o Project foi pago. Vazio é o estado normal: não pago.
   *
   * Eixo próprio, e não um quinto `TrackingStatus`: o estado descreve o
   * trabalho, isto descreve o dinheiro. "Concluído e não pago" é o estado que
   * interessa cobrar, e colapsar os dois o tornaria inexprimível.
   */
  paidAt: string | null;
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
  /** Oferecer gravação quando um programa abre o microfone (ADR-047).
   *
   *  Ligada de fábrica. O M/OS observa QUAL programa abriu o microfone — nunca o
   *  título da janela, o conteúdo da tela ou o áudio. */
  meetingDetectionEnabled: boolean;
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

export type CalendarKind = "session" | "task_done" | "task_created" | "capture" | "app_opened" | "day_started" | "day_ended" | "objective_done" | "meeting";

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

/* Uma pétala fixada no leque. `workspaceId` nulo é a visão "Todos", pela mesma
   leitura da 0018, agora na 0021.

   A AUSÊNCIA de linha para um slot também significa algo — "o que o desenho
   escolheu" — e quem resolve isso é `lequePetalas.ts`, a única cópia da regra. */
/** Os enderecos do shell.
 *
 * Mora aqui, e nao no `App.tsx`, porque deixou de ser assunto so dele: o leque
 * navega por `pagina`, e importar o tipo de la criaria ciclo — o `App` ja
 * importa o `Leque`. */
export type Page =
  | "home"
  | "hermes"
  | "inbox"
  | "projects"
  | "workspaces"
  | "apps"
  | "library"
  | "tasks"
  | "tempo"
  | "calendario"
  | "finance"
  | "reunioes"
  | "settings";

/**
 * O nome que cada página tem quando alguém fala dela.
 *
 * Existe para o Hermes: o preâmbulo diz "Tela aberta: Kanban", e é isso que
 * permite a ele entender que "essa coluna" e "esse quadro" são o Kanban do
 * M/OS. Mandar o valor cru — `tasks` — ensinaria o nome interno, que não é o
 * nome que o usuário usa.
 *
 * Mora aqui, junto de `Page`, pelo mesmo motivo que `Page` mora aqui: quem
 * acrescentar uma página tem de ver que ela precisa de um nome falado.
 */
export const SCREEN_LABEL: Record<Page, string> = {
  home: "Painel",
  hermes: "Hermes",
  inbox: "Inbox",
  projects: "Projects",
  workspaces: "Workspaces",
  apps: "Apps",
  library: "Library",
  tasks: "Kanban",
  tempo: "CronoCAD",
  calendario: "Calendário",
  finance: "Finance",
  reunioes: "Reuniões",
  settings: "Configurações",
};

export type RadialPin = {
  workspaceId: string | null;
  slot: number;
  kind: string;
  target: string;
};

export type RadialPinInput = {
  slot: number;
  kind: string;
  target: string;
};

// ===========================================================================
// Meeting Agent
// ===========================================================================

/** A ordem em que o pipeline anda. `failed` carrega o estágio em `failure`. */
export type MeetingStatus =
  | "recording" | "paused" | "stopping" | "interrupted" | "recorded"
  | "transcribing" | "transcribed" | "analyzing" | "ready"
  | "failed" | "cancelled";

export type FailedStage = "audio" | "transcription" | "analysis";

/**
 * O destino de um canal.
 *
 * Três variantes e não um booleano: "nunca abriu" e "abriu e caiu aos 32:10"
 * pedem frases diferentes na tela e preservam quantidades diferentes de áudio.
 */
export type ChannelOutcome =
  | { state: "capturing" }
  | { state: "captured" }
  | { state: "unavailable"; reason: string }
  | { state: "lost"; atMs: number; reason: string };

export type AudioRetention = "delete_after_processing" | "keep_24h" | "keep";

export type Meeting = {
  id: string;
  title: string;
  status: MeetingStatus;
  lifecycleState: LifecycleState;
  source: "manual" | "calendar" | "detected";
  startedAt: string;
  endedAt: string | null;
  /** Medida em frames gravados, nunca por diferença de relógio. */
  durationMs: number;
  projectId: string | null;
  audioDir: string;
  retention: AudioRetention;
  audioDeletedAt: string | null;
  mic: ChannelOutcome;
  system: ChannelOutcome;
  failure: { stage: FailedStage; message: string } | null;
  createdAt: string;
  updatedAt: string;
  cancelledAt: string | null;
  /** O que quem gravou escreveu durante a reunião.
   *
   *  Vazio significa "ninguém escreveu", e não "falta dado". Sobe ao Hermes como
   *  contexto e não gera item: o prompt exige `segment` por item, e uma nota não
   *  foi dita, foi escrita. */
  notes: string;
};

/** MIC é quem gravou; SYSTEM são os outros. É a distinção que a V1 protege. */
export type MeetingChannel = "mic" | "system";

export type TranscriptSegment = {
  id: string;
  meetingId: string;
  seq: number;
  startMs: number;
  endMs: number;
  channel: MeetingChannel;
  text: string;
  speaker: string | null;
  confidence: number | null;
};

export type InsightKind =
  | "decision" | "my_action" | "other_action" | "deadline"
  | "follow_up" | "open_question" | "risk" | "topic";

export type Confidence = "high" | "medium" | "low";
export type InsightStatus = "proposed" | "accepted" | "dismissed";

/** Referência a um trecho. O texto não é copiado: ele É o segmento. */
export type MeetingEvidence = {
  segmentId: string;
  seq: number;
  charStart: number | null;
  charEnd: number | null;
};

export type MeetingInsight = {
  id: string;
  meetingId: string;
  kind: InsightKind;
  seq: number;
  text: string;
  owner: string | null;
  /** O prazo COMO FOI DITO. Quem resolve para um instante é esta tela. */
  dueHint: string | null;
  confidence: Confidence;
  status: InsightStatus;
  createdTaskId: string | null;
  createdReminderId: string | null;
  evidence: MeetingEvidence[];
};

export type MeetingAnalysis = {
  meetingId: string;
  summary: string;
  model: string;
  producedAt: string;
  /** Quantas janelas de transcrição foram enviadas. Aparece na tela. */
  windows: number;
};

export type InsightPreview = {
  insightId: string;
  kind: InsightKind;
  title: string;
  owner: string | null;
  dueHint: string | null;
  confidence: Confidence;
  evidenceCount: number;
  eligibleForBulk: boolean;
  /** Vazio quando não há bloqueio. Nunca um beco sem saída. */
  blockedReason: string;
};

/** O nível cru, a 15 Hz — um a cada 66 ms. Dois números e nada mais.
 *
 *  Evento separado do `MeetingTick` porque as duas coisas mudam em ritmos
 *  diferentes: mandar o tick inteiro quinze vezes por segundo seria repetir um
 *  objeto que mudou zero. */
export type MeetingLevel = {
  mic: number;
  system: number;
};

/** O que chega uma vez por segundo enquanto grava. Nunca PCM. */
export type MeetingTick = {
  meetingId: string;
  durationMs: number;
  mic: ChannelOutcome;
  system: ChannelOutcome;
  /** RMS em milésimos, já reduzido no Rust. */
  micLevel: number;
  systemLevel: number;
  /** Vem do átomo da sessão, e não do banco: a barra precisa parar de pulsar no
   *  MESMO instante em que o áudio para. */
  paused: boolean;
};

export type TranscriberStatus = {
  configured: boolean;
  ready: boolean;
  problem: string;
  name: string;
  binary: string;
  model: string;
  /** Caminho do modelo Silero. Vazio é resposta válida: significa VAD desligado. */
  vadModel: string;
  threads: number;
};

export type AnalysisConsent = {
  granted: boolean;
  grantedAt: string;
};

// =============================================================================
// Voice Inbox
// =============================================================================
//
// Nenhum destes carrega audio: a captura inteira vive no Rust, e o que sobe
// para ca e estado. `level` e `peak` sao RMS ja reduzidos a `0..1000` dentro da
// thread de captura — nao existe caminho de PCM ate aqui.

export type VoiceNoteStatus =
  | "recording"
  | "recorded"
  | "transcribing"
  | "captured"
  | "failed"
  | "cancelled";

export type VoiceNote = {
  id: string;
  status: VoiceNoteStatus;
  audioDir: string;
  durationMs: number;
  peakLevel: number;
  /** A transcricao ORIGINAL. Ela nao e reescrita em lugar nenhum. */
  transcript: string;
  provider: string;
  captureId: string | null;
  contextProjectId: string | null;
  contextTaskId: string | null;
  failureMessage: string;
  audioDeletedAt: string | null;
  startedAt: string;
  updatedAt: string;
};

export type VoiceTick = {
  noteId: string;
  durationMs: number;
  level: number;
  peak: number;
  /** Vazio quando o microfone esta bem. */
  problem: string;
};

/** O desfecho de soltar a tecla. As duas recusas nao persistiram nada. */
export type VoiceStopped =
  /** Nao havia gravacao. Nao e erro: os dois caminhos de parada correm juntos. */
  | { outcome: "notRecording" }
  | { outcome: "tooShort" }
  | { outcome: "tooQuiet" }
  | { outcome: "transcribing"; noteId: string };

export type VoiceAction = "keep" | "create_task" | "create_task_with_reminder";

export type VoiceFailed = {
  noteId: string;
  message: string;
  /** Ha audio em disco esperando um retry. */
  retryable: boolean;
};

// ===========================================================================
// Daily Session — a camada de intenção sobre o dia
// ===========================================================================
//
// Espelha `crates/mos-core/src/daily.rs`. Os nomes de estado atravessam a
// ponte: renomear um deles de um lado só faz a tela deixar de reconhecer o
// dado, sem erro de compilação de nenhum dos dois.

/** `AAAA-MM-DD`, na data civil de quem estava na frente da tela. */
export type Day = string;

/**
 * `not_started` **nunca vem do banco** — ele é o nome que a interface dá à
 * ausência de sessão. Ver o comentário de `SessionStatus` no domínio.
 */
export type SessionStatus = "not_started" | "active" | "completed";

export type ObjectivePriority = "main" | "secondary";

export type ObjectiveStatus = "pending" | "completed" | "carried_over" | "dropped";

export type DayMood = "productive" | "normal" | "blocked";

export type LinkKind = "task" | "project" | "capture" | "resource" | "meeting";

/** O par (tipo, id) que liga um objetivo a algo que já existe. */
export type ObjectiveLink = {
  kind: LinkKind;
  id: string;
};

export type DailySession = {
  id: string;
  day: Day;
  status: Exclude<SessionStatus, "not_started">;
  /** Vazio significa nenhuma. Hoje só o Hermes escreve. */
  note: string;
  startedAt: string;
  endedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type DailyObjective = {
  id: string;
  sessionId: string;
  title: string;
  description: string;
  /** `null` é intenção livre. O TIPO do objetivo é isto — não há campo `type`. */
  link: ObjectiveLink | null;
  priority: ObjectivePriority;
  status: ObjectiveStatus;
  position: number;
  /** O objetivo de que este veio, quando veio de um carry-over. */
  carriedFrom: string | null;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
};

export type DailyReflection = {
  sessionId: string;
  mood: DayMood | null;
  summary: string;
  createdAt: string;
  updatedAt: string;
};

/** O dia inteiro numa chamada: a Home precisa dos três na primeira pintura. */
export type DailyToday = {
  day: Day;
  status: SessionStatus;
  session: DailySession | null;
  objectives: DailyObjective[];
  reflection: DailyReflection | null;
  /** A sessão de um dia ANTERIOR que ficou aberta. */
  stale: DailySession | null;
  staleObjectives: DailyObjective[];
};

export type DailySessionSummary = {
  session: DailySession;
  done: number;
  total: number;
  /** Vazio quando o dia não teve principal. */
  mainTitle: string;
  mood: DayMood | null;
};

export type TaskSuggestion = {
  id: string;
  title: string;
  state: TaskState;
  /** Vazio quando a Task não tem Project. */
  project: string;
};

export type ProjectSuggestion = {
  id: string;
  name: string;
  openTasks: number;
};

export type CarryOver = {
  objectiveId: string;
  title: string;
  link: ObjectiveLink | null;
  /** Quantas vezes esta corrente já foi carregada. */
  timesCarried: number;
};

/**
 * O que o M/OS já sabe sobre hoje, antes de a pessoa escolher qualquer coisa.
 *
 * Três coisas que o pedido listou **não estão aqui, e não é esquecimento**:
 * Task não tem prazo no M/OS (decisão D-1), não existe entidade Event (D-4) e
 * não existe Waiting For. O que faz as vezes de prazo é o Reminder apontado
 * para a Task — e é ele que `dueToday` e `overdue` contam.
 */
export type DailyContext = {
  dueToday: number;
  overdue: number;
  highPriority: number;
  meetingsToday: number;
  inbox: number;
  freshCaptures: number;
  projects: number;
  doing: number;
  openTasks: number;
  suggestedTasks: TaskSuggestion[];
  suggestedProjects: ProjectSuggestion[];
  carryOver: CarryOver[];
  /** Vazia quando não há carry-over. */
  carryOverDay: string;
};

/** Um objetivo como a interface o descreve, antes de ele existir. */
export type ObjectiveDraft = {
  title: string;
  description?: string;
  /** Vazio nos dois é intenção livre. Metade preenchida é recusada. */
  linkKind?: LinkKind | "";
  linkId?: string;
  /** O objetivo de ontem de que este veio. */
  carriedFrom?: string;
};

export type StartDayInput = {
  main: ObjectiveDraft | null;
  secondaries: ObjectiveDraft[];
  note?: string;
};

export type ObjectiveResolution = {
  objectiveId: string;
  status: ObjectiveStatus;
};

export type EndDayInput = {
  /**
   * Objetivo pendente que NÃO aparecer aqui fica pendente — e reaparece no
   * carry-over do próximo Start My Day. Não decidir é uma resposta válida.
   */
  resolutions: ObjectiveResolution[];
  mood?: DayMood | "";
  summary?: string;
};

// ===========================================================================
// Weekly Review — o fecho da semana
// ===========================================================================
//
// Espelha `crates/mos-core/src/weekly.rs`.

/** A data da SEGUNDA-FEIRA da semana, `AAAA-MM-DD`. Nunca número ISO. */
export type Week = string;

export type WeeklyReview = {
  id: string;
  week: Week;
  /** Vazio é legítimo: fechar a semana é o gesto, escrever é opcional. */
  summary: string;
  closedAt: string;
  createdAt: string;
  updatedAt: string;
};

export type Dominant = {
  label: string;
  /** Em quantos dias isto foi o objetivo principal. */
  mainDays: number;
  days: number;
};

export type Recurring = {
  title: string;
  timesCarried: number;
};

/**
 * A semana em narrativa. **Nenhum placar** — não existe `X de Y` aqui, e a
 * ausência é a decisão: `ATTENTION-SYSTEM.md` §19 proíbe resumo de
 * produtividade em digest semanal. `daysWithSession` é fato sobre o uso do
 * sistema, e não sobre o trabalho.
 */
export type WeekSummary = {
  week: Week;
  daysWithSession: number;
  dominated: Dominant[];
  recurring: Recurring[];
  dropped: string[];
  blockedDays: Day[];
  review: WeeklyReview | null;
  /** Nenhuma sessão na semana. A tela usa isto para NÃO oferecer o fecho. */
  empty: boolean;
};

/** O que está parado há tempo demais. Vem pronto de `mos-core::stale`. */
export type Parada = {
  kind: "task" | "project";
  id: string;
  title: string;
  /** Nome do Project, para Task. "N tasks abertas", para Project. */
  context: string;
  /** A coluna do Kanban, para Task. Vazio para Project. */
  state: string;
  days: number;
};

/** Quando um Project foi mexido de verdade — e não quando foi renomeado. */
export type ProjectActivity = { projectId: string; lastActivity: string };

export type StaleView = { paradas: Parada[]; activity: ProjectActivity[] };
