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

// Largura média de um caractere do rótulo em 12px, medida no navegador com a
// fonte do app. Estimar em vez de medir é escolha: medir texto SVG exige montar
// o nó antes de saber o tamanho da área de desenho, e a conta fica circular.
const LABEL_CHAR_PX = 6.2;
// Distância entre o fim da barra e o começo do rótulo, igual à do componente.
const LABEL_GAP_PX = 8;
// Folga para o arredondamento da área de desenho não comer o último caractere.
const LABEL_SLACK_PX = 4;

/**
 * Espaço à direita que os rótulos de valor precisam.
 *
 * O rótulo — `R$ 2.450,00 · 36%` — é desenhado fora da barra, e o gráfico
 * reservava 96px fixos para ele. Cabia nos valores da tela de quem escreveu, e
 * cortava o `%` do maior valor a 375px de viewport, onde a área de desenho
 * encolhe mas o rótulo não. A reserva agora sai do rótulo mais largo da própria
 * série: em telas largas ela devolve espaço à barra, e em telas estreitas ela
 * garante que o número inteiro apareça.
 */
export function categoryLabelReserve(labels: string[]): number {
  const longest = labels.reduce((max, label) => Math.max(max, label.length), 0);
  if (longest === 0) return 0;

  return Math.ceil(longest * LABEL_CHAR_PX) + LABEL_GAP_PX + LABEL_SLACK_PX;
}
