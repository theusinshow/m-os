/**
 * Agrupamento de linhas do relatorio por chave (projeto, tipo de atividade).
 *
 * Opera sobre as linhas JA calculadas pela pagina — as mesmas que produzem o
 * "Valor total". Assim a soma dos grupos fecha com o total por construcao: nao
 * ha um segundo caminho de calculo que possa divergir.
 */

import type { Cents, Seconds } from "@/types/domain";

/** Uma linha do relatorio, reduzida ao que a agregacao precisa. */
export interface ReportRow {
  /** Duracao faturavel ja liquida e arredondada. */
  roundedBillable: Seconds;
  amount: Cents;
}

export interface Group<K> {
  key: K;
  seconds: Seconds;
  amount: Cents;
}

/**
 * Agrupa as linhas pela chave dada, do maior valor para o menor — quem cobra
 * quer ver primeiro o que mais pesa na fatura. Empates ficam na ordem em que
 * apareceram.
 */
export function groupBy<T extends ReportRow, K>(
  rows: T[],
  keyOf: (row: T) => K,
): Group<K>[] {
  const map = new Map<K, Group<K>>();
  for (const row of rows) {
    const key = keyOf(row);
    const current = map.get(key) ?? { key, seconds: 0, amount: 0 };
    current.seconds += row.roundedBillable;
    current.amount += row.amount;
    map.set(key, current);
  }
  return [...map.values()].sort((a, b) => b.amount - a.amount);
}
