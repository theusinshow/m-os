/**
 * Conversao entre ISO UTC (backend) e o valor de `input[type=datetime-local]`
 * (horario local). Centralizado para edicao/criacao manual de sessoes.
 */

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

/** ISO UTC -> "YYYY-MM-DDTHH:MM" no horario local (para datetime-local). */
export function isoToLocalInput(iso: string): string {
  const d = new Date(iso);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
    d.getHours(),
  )}:${pad(d.getMinutes())}`;
}

/** "YYYY-MM-DDTHH:MM" (horario local) -> ISO UTC. */
export function localInputToIso(local: string): string {
  return new Date(local).toISOString();
}

/** Data ISO -> "YYYY-MM-DD" no horario local (para input[type=date]). */
export function isoToDateInput(iso: string): string {
  const d = new Date(iso);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** True se `iso` cai no dia local de `ref` (comparacao por data local). */
export function isSameLocalDay(iso: string, ref: Date): boolean {
  return new Date(iso).toDateString() === ref.toDateString();
}
