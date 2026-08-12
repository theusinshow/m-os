/**
 * Sinaliza sessoes com duracao implausivel — tipicamente o cronometro deixado
 * ligado durante a noite. E regra de VISUALIZACAO: nao altera nada no banco,
 * so decide se a tela mostra o selo "Conferir?" (regra critica 5).
 */

import type { TimeEntry } from "@/types/domain";

export type SuspicionReason = "muito-longa" | "madrugada";

export interface Suspicion {
  suspicious: boolean;
  reasons: SuspicionReason[];
}

/** Acima disto, uma unica sessao de cronometro vira candidata a esquecimento. */
export const LONG_SESSION_HOURS = 8;

/** Hora local em que praticamente ninguem esta desenhando. */
const DEAD_HOUR = 4;

/**
 * Folga maxima entre a duracao gravada e o intervalo de relogio para a sessao
 * ainda contar como continua.
 *
 * Sem isto, um cronometro PAUSADO e retomado dias depois (25 min gravados num
 * intervalo de 3 dias) seria marcado como "madrugada" so porque o intervalo
 * inicio->fim passa por cima das 04:00 — sendo que o cronometro estava parado
 * naquelas horas. Casos reais desses aparecem no historico; a folga separa os
 * dois: cronometro esquecido ligado tem folga zero, o pausado tem dezenas de
 * horas.
 */
const CONTINUOUS_SLACK_SECONDS = 2 * 3600;

const NOT_SUSPICIOUS: Suspicion = { suspicious: false, reasons: [] };

/**
 * True se o intervalo local [start, end] contem alguma ocorrencia de DEAD_HOUR.
 * Uma sessao de varios dias precisa de uma unica batida para valer.
 */
function crossesDeadHour(start: Date, end: Date): boolean {
  const probe = new Date(start);
  probe.setHours(DEAD_HOUR, 0, 0, 0);
  // A batida do dia do inicio pode ter ficado para tras; nesse caso a proxima
  // candidata e a do dia seguinte.
  if (probe.getTime() < start.getTime()) {
    probe.setDate(probe.getDate() + 1);
  }
  return probe.getTime() <= end.getTime();
}

export function inspectEntry(entry: TimeEntry): Suspicion {
  // Sessao manual foi digitada de proposito e a reconstruida nasce de uma
  // decisao explicita na linha do tempo: marcar as duas seria alarme falso.
  if (entry.source !== "timer") return NOT_SUSPICIOUS;
  // Cronometro em andamento nao e erro — ainda da tempo de encerrar.
  if (entry.endedAt === null) return NOT_SUSPICIOUS;

  const start = new Date(entry.startedAt);
  const end = new Date(entry.endedAt);

  const reasons: SuspicionReason[] = [];
  if (entry.durationSeconds > LONG_SESSION_HOURS * 3600) {
    reasons.push("muito-longa");
  }

  // A hora de parede so diz algo sobre a madrugada se o cronometro esteve
  // rodando: num cronometro pausado, o intervalo nao representa trabalho.
  const spanSeconds = (end.getTime() - start.getTime()) / 1000;
  const continuous =
    spanSeconds - entry.durationSeconds <= CONTINUOUS_SLACK_SECONDS;
  if (continuous && crossesDeadHour(start, end)) {
    reasons.push("madrugada");
  }

  return { suspicious: reasons.length > 0, reasons };
}
