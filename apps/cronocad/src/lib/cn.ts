/**
 * Helper minimo para compor classes condicionalmente, sem dependencia extra.
 * Aceita strings, falsos (ignorados) e retorna uma unica string.
 */
export function cn(
  ...values: Array<string | false | null | undefined>
): string {
  return values.filter(Boolean).join(" ");
}
