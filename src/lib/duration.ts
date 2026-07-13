/**
 * Calculo de duracao do cronometro e distincoes de tempo (secoes 9 e 11).
 *
 * Regra critica: a duracao NUNCA depende apenas de um contador do frontend.
 * A fonte da verdade e o backend, que persiste timestamps e recalcula. As
 * funcoes aqui derivam valores para exibicao a partir desses timestamps.
 *
 * Distincoes documentadas:
 *  - duracao bruta (gross): tempo total entre inicio e fim;
 *  - tempo inativo (idle): periodo sem atividade detectada;
 *  - duracao liquida (net): bruta menos inativa descontada;
 *  - duracao faturavel (billable): liquida quando billable = true, senao 0;
 *  - duracao arredondada: aplicada apenas na visualizacao/cobranca.
 */

import type { ActiveTimer, Seconds } from "@/types/domain";

/**
 * Segundos decorridos de um cronometro ativo em um instante de referencia.
 *
 * Considera o tempo acumulado (pausas anteriores) e, se estiver rodando,
 * soma o intervalo desde `lastResumedAt` ate `nowMs`. Quando pausado,
 * retorna apenas o acumulado.
 *
 * Robusto a relogio para tras: diferencas negativas sao tratadas como 0
 * (nunca reduzem o tempo ja acumulado).
 */
export function elapsedSeconds(timer: ActiveTimer, nowMs: number): Seconds {
  const accumulated = Math.max(0, timer.accumulatedSeconds);
  if (timer.status === "paused") {
    return accumulated;
  }
  const resumedMs = Date.parse(timer.lastResumedAt);
  if (Number.isNaN(resumedMs)) {
    return accumulated;
  }
  const deltaSeconds = Math.max(0, Math.floor((nowMs - resumedMs) / 1000));
  return accumulated + deltaSeconds;
}

/** Duracao liquida = bruta - inativa (nunca negativa). */
export function netDuration(grossSeconds: Seconds, idleSeconds: Seconds): Seconds {
  return Math.max(0, grossSeconds - Math.max(0, idleSeconds));
}

/** Duracao faturavel: liquida quando billable, senao 0. */
export function billableDuration(
  netSeconds: Seconds,
  billable: boolean,
): Seconds {
  return billable ? Math.max(0, netSeconds) : 0;
}
