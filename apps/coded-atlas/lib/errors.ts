import type { AtlasErrorCode } from "./types";

export class AtlasError extends Error {
  constructor(
    public code: AtlasErrorCode,
    public userMessage: string,
    public detail?: string
  ) {
    super(detail ?? userMessage);
    this.name = "AtlasError";
  }
}
