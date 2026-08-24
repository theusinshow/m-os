export type CategorySlice = {
  name: string;
  value: number;
  /** Percentual inteiro do total. A soma da lista é exatamente 100. */
  percent: number;
};

/**
 * Fatia as categorias do mês em algo que cabe num gráfico.
 *
 * Duas decisões moram aqui em vez do componente: o teto de categorias — sem
 * ele, vinte categorias viram um gráfico de 880px que ninguém lê inteiro — e a
 * distribuição do percentual, que precisa fechar em 100 porque o número aparece
 * rotulado na barra e `33% + 33% + 33%` lido numa tela parece defeito.
 */
export function toCategorySlices(
  data: { name: string; value: number }[],
  maxSlices = 8,
): CategorySlice[] {
  const positive = data.filter((item) => item.value > 0);
  const total = positive.reduce((sum, item) => sum + item.value, 0);
  if (total === 0) return [];

  const sorted = [...positive].sort((a, b) => b.value - a.value);

  // Cabendo tudo, não existe "Outras". Não cabendo, a última vaga é dela.
  const head = sorted.length > maxSlices ? sorted.slice(0, maxSlices - 1) : sorted;
  const tail = sorted.slice(head.length);
  const grouped =
    tail.length > 0
      ? [...head, { name: "Outras", value: tail.reduce((sum, item) => sum + item.value, 0) }]
      : head;

  const percents = distributePercents(
    grouped.map((slice) => slice.value),
    total,
  );

  return grouped.map((slice, index) => ({
    name: slice.name,
    value: slice.value,
    percent: percents[index],
  }));
}

/**
 * Maior resto: arredonda todo mundo para baixo e devolve as sobras para quem
 * tinha a maior fração. É o que garante que a soma feche em 100 sem distorcer
 * nenhuma fatia em mais de um ponto.
 */
function distributePercents(values: number[], total: number): number[] {
  const exact = values.map((value) => (value / total) * 100);
  const result = exact.map(Math.floor);
  const missing = 100 - result.reduce((sum, value) => sum + value, 0);

  const byFraction = exact
    .map((value, index) => ({ index, fraction: value - Math.floor(value) }))
    .sort((a, b) => b.fraction - a.fraction);

  for (let given = 0; given < missing; given += 1) {
    result[byFraction[given % byFraction.length].index] += 1;
  }

  return result;
}
