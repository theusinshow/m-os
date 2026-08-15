/**
 * Calculo monetario (secao 20 / secao 8).
 *
 * Valores sempre em centavos inteiros para evitar erros de ponto flutuante.
 * O valor/hora de uma sessao usa o snapshot preservado no momento da sessao
 * (alterar o valor atual do projeto nao altera sessoes anteriores).
 */

import type { Cents, Seconds } from "@/types/domain";

/**
 * Valor de uma duracao dado um valor/hora em centavos.
 * Arredonda para o centavo inteiro mais proximo.
 *
 * Exemplo: 1h30 (5400s) a R$ 100,00/h (10000 centavos) -> 15000 (R$ 150,00).
 */
export function amountForDuration(
  seconds: Seconds,
  hourlyRateCents: Cents,
): Cents {
  const hours = Math.max(0, seconds) / 3600;
  return Math.round(hours * hourlyRateCents);
}

/** Converte reais (com centavos) para centavos inteiros: 123.45 -> 12345. */
export function toCents(amount: number): Cents {
  return Math.round(amount * 100);
}

/** Converte centavos para reais: 12345 -> 123.45. */
export function fromCents(cents: Cents): number {
  return cents / 100;
}
