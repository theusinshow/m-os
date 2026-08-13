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

export type AppError = {
  code: string;
  message: string;
  retryable: boolean;
};

export type AppStatus = {
  inboxCount: number;
  shortcut: string;
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
