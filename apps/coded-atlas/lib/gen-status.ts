export const GEN_STATUS_KEY = "atlas:gen-status";
export const GEN_STATUS_EVENT = "atlas:gen-status-update";

export interface GenStatus {
  state: "generating" | "done" | "error";
  slug: string;
  name: string;
  projectUrl?: string;
  errorMessage?: string;
  startedAt: number;
}

export function emitGenStatus(status: GenStatus | null): void {
  try {
    if (status) {
      localStorage.setItem(GEN_STATUS_KEY, JSON.stringify(status));
    } else {
      localStorage.removeItem(GEN_STATUS_KEY);
    }
    window.dispatchEvent(
      new CustomEvent<GenStatus | null>(GEN_STATUS_EVENT, { detail: status })
    );
  } catch {}
}

export function readGenStatus(): GenStatus | null {
  try {
    const raw = localStorage.getItem(GEN_STATUS_KEY);
    return raw ? (JSON.parse(raw) as GenStatus) : null;
  } catch {
    return null;
  }
}
