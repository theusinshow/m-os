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

export type SearchItem =
  | { kind: "capture"; capture: Capture; derivedTask: Task | null; project: Project | null }
  | { kind: "task"; task: Task; project: Project | null }
  | { kind: "project"; project: Project };

export type AppError = {
  code: string;
  message: string;
  retryable: boolean;
};

export type AppStatus = {
  inboxCount: number;
  projectCount: number;
  taskCount: number;
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
