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

export type TaskState = "backlog" | "doing" | "done";

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
  lifecycleState: LifecycleState;
  createdAt: string;
  updatedAt: string;
  lastOpenedAt: string | null;
};

export type Resource = {
  id: string;
  kind: "link";
  title: string;
  url: string;
  note: string;
  sourceCaptureId: string | null;
  lifecycleState: LifecycleState;
  createdAt: string;
  updatedAt: string;
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
