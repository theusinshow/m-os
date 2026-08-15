import type { ZodError } from "zod";

export type FormState = {
  status: "idle" | "success" | "error";
  message?: string;
  fieldErrors?: Record<string, string>;
};

export const initialFormState: FormState = { status: "idle" };

/**
 * Maps a ZodError to a flat { field: message } object using the first issue
 * per field. `rename` lets actions expose errors under the input's name when
 * the schema key differs (e.g. amountCents -> amount).
 */
export function fieldErrorsFromZod(
  error: ZodError,
  rename: Record<string, string> = {},
): Record<string, string> {
  const result: Record<string, string> = {};
  for (const issue of error.issues) {
    const rawKey = issue.path[0];
    if (typeof rawKey !== "string") continue;
    const key = rename[rawKey] ?? rawKey;
    if (!result[key]) {
      result[key] = issue.message;
    }
  }
  return result;
}

export function successState(message: string): FormState {
  return { status: "success", message };
}

export function errorState(
  message: string,
  fieldErrors?: Record<string, string>,
): FormState {
  return { status: "error", message, fieldErrors };
}
