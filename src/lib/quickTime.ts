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
  /**
   * Ancora do bloco novo: quando presente, e o **fim** do bloco (na pratica,
   * o `startedAt` da sessao ancora), para o ajuste entrar logo antes dela em
   * vez de aninhado dentro do intervalo original.
   */
  anchorAtIso?: string | null;
  /**
   * Sessoes ja existentes (a funcao filtra as do dia pedido). Normalmente vem
   * do `entriesStore`, que carrega no maximo 200 sessoes recentes — para um
   * dia antigo fora dessa janela a lista chega vazia aqui, e a regra das
   * 18:00 assume.
   */
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

/**
 * Fim da ultima sessao encerrada naquele dia local, ou null.
 *
 * Atribui a sessao ao dia em que ela **comecou** (`startedAt`), nao ao dia em
 * que terminou. Uma sessao que comeca 23:50 de D-1 e termina 00:10 de D nao
 * conta como "ultima sessao de D" — defensavel e de baixo impacto, mas fica
 * documentado para nao ser re-litigado.
 */
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

/**
 * Aplica a primeira regra de ancoragem que casar (a ordem importa):
 * 1. ancorado: o bloco termina onde a sessao ancora comeca (`anchorAtIso`).
 * 2. dia = hoje: termina agora (`now`).
 * 3. dia passado, com sessoes: termina no fim da ultima sessao do dia.
 * 4. dia passado, vazio: termina as 18:00 locais.
 */
function resolveEnd(input: QuickEntryWindowInput): Date {
  const { day, anchorAtIso, dayEntries, now } = input;

  if (anchorAtIso) return new Date(anchorAtIso);
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
