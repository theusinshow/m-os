/**
 * Reconstrucao do dia (secao 14): a partir dos eventos de atividade, pareia as
 * sessoes de cada programa (aberto -> fechado) e identifica lacunas — periodos
 * com programa aberto sem sessao de trabalho registrada. Funcoes puras e
 * testaveis; a decisao de transformar uma lacuna em registro cabe ao usuario.
 */

import type { ActivityEvent, ActivityEventType, TimeEntry } from "@/types/domain";

export interface AppInterval {
  processName: string;
  /** ISO de abertura. */
  start: string;
  /** ISO de fechamento, ou null se ainda estava aberto ao fim do periodo. */
  end: string | null;
}

/**
 * Pareia eventos `app_opened`/`app_closed` por processo, em ordem cronologica.
 * Aberturas sem fechamento correspondente resultam em `end: null`.
 */
export function pairAppSessions(events: ActivityEvent[]): AppInterval[] {
  const open = new Map<string, string>();
  const intervals: AppInterval[] = [];

  for (const ev of events) {
    if (!ev.processName) continue;
    if (ev.eventType === "app_opened") {
      // Uma nova abertura sem fechamento anterior fecha a anterior implicitamente.
      open.set(ev.processName, ev.detectedAt);
    } else if (ev.eventType === "app_closed") {
      const started = open.get(ev.processName);
      if (started) {
        intervals.push({
          processName: ev.processName,
          start: started,
          end: ev.detectedAt,
        });
        open.delete(ev.processName);
      }
    }
  }

  for (const [processName, start] of open) {
    intervals.push({ processName, start, end: null });
  }
  return intervals.sort((a, b) => a.start.localeCompare(b.start));
}

/**
 * Retorna os intervalos de programa aberto que NAO se sobrepoem a nenhuma
 * sessao registrada (nem excluida) — as lacunas a reconstruir. `nowMs` fecha os
 * intervalos ainda abertos.
 */
export function findGaps(
  intervals: AppInterval[],
  entries: TimeEntry[],
  nowMs: number,
): AppInterval[] {
  return intervals.filter((iv) => {
    const ivStart = Date.parse(iv.start);
    const ivEnd = iv.end ? Date.parse(iv.end) : nowMs;
    const overlaps = entries.some((e) => {
      if (e.deletedAt) return false;
      const es = Date.parse(e.startedAt);
      const ee = e.endedAt ? Date.parse(e.endedAt) : es;
      return es < ivEnd && ee > ivStart;
    });
    return !overlaps && ivEnd - ivStart >= 60_000; // ignora lacunas < 1 min
  });
}

export const EVENT_LABELS: Record<ActivityEventType, string> = {
  app_opened: "aberto",
  app_closed: "fechado",
  idle_started: "inatividade iniciada",
  idle_ended: "atividade retomada",
  timer_started: "cronometro iniciado",
  timer_paused: "cronometro pausado",
  timer_resumed: "cronometro retomado",
  timer_stopped: "cronometro encerrado",
};
