/**
 * Arredondamento de duracao (secao 12).
 *
 * IMPORTANTE: o arredondamento NUNCA substitui a duracao real armazenada.
 * Ele e aplicado apenas na visualizacao ou no calculo de cobranca. O banco
 * sempre preserva o tempo real (ver DATABASE.md / ARCHITECTURE.md).
 */

import type { RoundingMode, Seconds } from "@/types/domain";

export interface RoundingConfig {
  enabled: boolean;
  intervalMinutes: number;
  mode: RoundingMode;
}

/**
 * Arredonda uma duracao (em segundos) para o intervalo configurado.
 * Retorna a duracao original quando desativado ou intervalo invalido.
 *
 * Exemplo: 1h07 (4020s), intervalo 15min, modo "up" -> 1h15 (4500s).
 */
export function roundDuration(
  seconds: Seconds,
  config: RoundingConfig,
): Seconds {
  if (!config.enabled || config.intervalMinutes <= 0) {
    return seconds;
  }
  const intervalSeconds = config.intervalMinutes * 60;
  const quotient = seconds / intervalSeconds;

  let units: number;
  switch (config.mode) {
    case "up":
      units = Math.ceil(quotient);
      break;
    case "down":
      units = Math.floor(quotient);
      break;
    case "nearest":
    default:
      units = Math.round(quotient);
      break;
  }
  return units * intervalSeconds;
}
