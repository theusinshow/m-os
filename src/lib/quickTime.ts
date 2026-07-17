/**
 * Converte "duracao + dia" na janela (inicio/fim) que o banco exige.
 *
 * O usuario que esqueceu de ligar o cronometro lembra da duracao ("umas duas
 * horas"), nao do horario exato. Exigir horario e o que faz o registro ser
 * adiado — e adiado vira esquecido. Entao ancoramos o bloco num ponto plausivel
 * e derivamos o inicio de tras para frente.
 *
 * `now` e sempre injetado para as regras serem testaveis.
 */

import type { TimeEntry } from "@/types/domain";
import { isoToDateInput } from "./datetime";

/** Teto por lancamento. Acima disso e quase certamente engano. */
export const MAX_QUICK_SECONDS = 24 * 3600;

/** Hora do fim para um dia passado sem nenhuma sessao registrada. */
const FALLBACK_END_HOUR = 18;

/** Incrementos oferecidos pelo modal, em segundos. */
export const QUICK_INCREMENTS: ReadonlyArray<{
  label: string;
  seconds: number;
}> = [
  { label: "+15min", seconds: 15 * 60 },
  { label: "+30min", seconds: 30 * 60 },
  { label: "+1h", seconds: 3600 },
  { label: "+2h", seconds: 2 * 3600 },
];

export interface QuickEntryWindowInput {
  durationSeconds: number;
  /** "YYYY-MM-DD" no horario local. */
  day: string;
  /** Fim da sessao ancora, quando o tempo e um ajuste de sessao existente. */
  anchorEndIso?: string | null;
  /** Sessoes ja existentes (a funcao filtra as do dia pedido). */
  dayEntries: TimeEntry[];
  now: Date;
}

export interface QuickEntryWindow {
  /** ISO UTC. */
  startedAt: string;
  /** ISO UTC. */
  endedAt: string;
}

/** Mantem a duracao entre 0 e o teto de 24h. */
export function clampQuickSeconds(seconds: number): number {
  if (!Number.isFinite(seconds)) return 0;
  return Math.min(Math.max(0, Math.floor(seconds)), MAX_QUICK_SECONDS);
}

/** Fim da ultima sessao encerrada naquele dia local, ou null. */
function lastEndOfDay(entries: TimeEntry[], day: string): Date | null {
  let latest: Date | null = null;
  for (const e of entries) {
    if (e.endedAt === null) continue;
    if (isoToDateInput(e.startedAt) !== day) continue;
    const end = new Date(e.endedAt);
    if (latest === null || end.getTime() > latest.getTime()) latest = end;
  }
  return latest;
}

/** Aplica a primeira regra de ancoragem que casar (a ordem importa). */
function resolveEnd(input: QuickEntryWindowInput): Date {
  const { day, anchorEndIso, dayEntries, now } = input;

  if (anchorEndIso) return new Date(anchorEndIso);
  if (isoToDateInput(now.toISOString()) === day) return now;

  const last = lastEndOfDay(dayEntries, day);
  if (last) return last;

  return new Date(`${day}T${String(FALLBACK_END_HOUR).padStart(2, "0")}:00:00`);
}

export function resolveQuickEntryWindow(
  input: QuickEntryWindowInput,
): QuickEntryWindow {
  const end = resolveEnd(input);
  const seconds = clampQuickSeconds(input.durationSeconds);
  const start = new Date(end.getTime() - seconds * 1000);
  return { startedAt: start.toISOString(), endedAt: end.toISOString() };
}
