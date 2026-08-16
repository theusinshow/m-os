/**
 * Sinaliza sessões com duração implausível — tipicamente o cronômetro deixado
 * ligado durante a noite.
 *
 * É regra de VISUALIZAÇÃO: não altera nada no banco, só decide se a tela mostra
 * o selo "Conferir?". O banco guarda sempre o tempo real.
 *
 * Vive no renderer e não em `mos-core` — ao contrário das outras regras de
 * tempo — porque depende da hora LOCAL de parede. O backend guarda tudo em UTC
 * e não tem como saber o fuso de quem está olhando; a mesma sessão que atravessa
 * as 4h da manhã em Florianópolis não atravessa nada em UTC.
 */

import type { TimeEntry } from "./types";

export type SuspicionReason = "muito-longa" | "madrugada";

/** Acima disto, uma única sessão de cronômetro vira candidata a esquecimento. */
export const LONG_SESSION_HOURS = 8;

/** Hora local em que praticamente ninguém está desenhando. */
const DEAD_HOUR = 4;

/**
 * Folga máxima entre a duração gravada e o intervalo de relógio para a sessão
 * ainda contar como contínua.
 *
 * Sem isto, um cronômetro PAUSADO e retomado dias depois (25 min gravados num
 * intervalo de 3 dias) seria marcado como "madrugada" só porque o intervalo
 * início→fim passa por cima das 04:00 — sendo que o cronômetro estava parado
 * naquelas horas. Casos reais desses aparecem no histórico; a folga separa os
 * dois: cronômetro esquecido ligado tem folga zero, o pausado tem dezenas de
 * horas.
 */
const CONTINUOUS_SLACK_SECONDS = 2 * 3600;

/**
 * True se o intervalo local [start, end] contém alguma ocorrência de DEAD_HOUR.
 * Uma sessão de vários dias precisa de uma única batida para valer.
 */
function crossesDeadHour(start: Date, end: Date): boolean {
  const probe = new Date(start);
  probe.setHours(DEAD_HOUR, 0, 0, 0);
  // A batida do dia do início pode ter ficado para trás; nesse caso a próxima
  // candidata é a do dia seguinte.
  if (probe.getTime() < start.getTime()) {
    probe.setDate(probe.getDate() + 1);
  }
  return probe.getTime() <= end.getTime();
}

export function inspectEntry(entry: TimeEntry): SuspicionReason[] {
  // Sessão manual foi digitada de propósito e a reconstruída nasce de uma
  // decisão explícita na linha do tempo: marcar as duas seria alarme falso.
  if (entry.source !== "timer") return [];
  // Cronômetro em andamento não é erro — ainda dá tempo de encerrar.
  if (entry.endedAt === null) return [];

  const start = new Date(entry.startedAt);
  const end = new Date(entry.endedAt);

  const reasons: SuspicionReason[] = [];
  if (entry.durationSeconds > LONG_SESSION_HOURS * 3600) {
    reasons.push("muito-longa");
  }

  // A hora de parede só diz algo sobre a madrugada se o cronômetro esteve
  // rodando: num cronômetro pausado, o intervalo não representa trabalho.
  const spanSeconds = (end.getTime() - start.getTime()) / 1000;
  const continuous = spanSeconds - entry.durationSeconds <= CONTINUOUS_SLACK_SECONDS;
  if (continuous && crossesDeadHour(start, end)) {
    reasons.push("madrugada");
  }

  return reasons;
}

const REASON_TEXT: Record<SuspicionReason, string> = {
  "muito-longa": `sessão de cronômetro acima de ${LONG_SESSION_HOURS}h`,
  madrugada: "cronômetro rodando pela madrugada",
};

/** O texto do `title` do selo — diz POR QUE, não só que algo está estranho. */
export function suspicionText(reasons: SuspicionReason[]) {
  return `Conferir: ${reasons.map((reason) => REASON_TEXT[reason]).join(" e ")}.`;
}
