/**
 * A aritmética das formas de plot — `ADR-040`.
 *
 * Vive separada do SVG por um motivo: `vitest.config.ts` roda só funções puras
 * em ambiente de nó, e é aqui que mora o que pode mentir sobre um valor. O
 * `Plot.tsx` desenha o que este arquivo calcula, e não calcula nada por conta.
 *
 * A distinção que organiza tudo: **`rx` não mente, `linecap` mente**. O canto
 * arredondado de um `rect` arredonda para DENTRO da geometria e a barra mantém
 * a altura exata do valor. A ponta arredondada de um traço estende para FORA,
 * meia espessura por ponta — e só ela precisa de compensação.
 */

/** Piso do traço compensado. Um dash quase-zero com cap redondo pinta o disco
 *  de forma determinística; zero puro fica a critério do renderizador. */
export const MIN_DASH = 0.01;

/**
 * Comprimento a DESENHAR para que o PINTADO seja `desired`, com cap redondo.
 *
 * Abaixo de uma espessura o resultado cai no piso, e o que aparece é um disco
 * do diâmetro do traço: o anel para de medir e passa a afirmar presença. Zero
 * continua não desenhando nada, que é a regra herdada da ADR-034.
 */
export function compensatedLength(desired: number, stroke: number) {
  if (desired <= 0) return 0;
  return Math.max(MIN_DASH, desired - stroke);
}

export type Rect = { x: number; y: number; width: number; height: number };

/**
 * Barras de largura igual, assentadas na linha de base.
 *
 * Sem altura mínima de propósito: o `rx` do `rect` é limitado pela própria
 * altura, então uma barra baixa sai arredondada sem que ninguém precise
 * inflá-la. Altura zero volta zero, e quem desenha decide não desenhar.
 */
export function barRects(ratios: number[], options: { width: number; height: number; gap: number }): Rect[] {
  const { width, height, gap } = options;
  const count = ratios.length;
  if (count === 0) return [];

  const barWidth = Math.max(0, (width - gap * (count - 1)) / count);
  return ratios.map((ratio, index) => {
    const clamped = Math.max(0, Math.min(1, ratio));
    const barHeight = clamped * height;
    return {
      x: index * (barWidth + gap),
      y: height - barHeight,
      width: barWidth,
      height: barHeight,
    };
  });
}

export type StackSegment = { index: number; x: number; width: number };

/**
 * Uma barra repartida na proporção dos valores.
 *
 * Os vãos só são descontados entre segmentos que existem: um valor zero não
 * ocupa lugar nem deixa buraco, senão a soma das partes não fecharia a largura.
 */
export function stackSegments(values: number[], options: { width: number; gap: number }): StackSegment[] {
  const { width, gap } = options;
  const positive = values.map((value) => Math.max(0, value));
  const total = positive.reduce((sum, value) => sum + value, 0);
  if (total <= 0) return [];

  const visible = positive.filter((value) => value > 0).length;
  const available = Math.max(0, width - gap * Math.max(0, visible - 1));

  const segments: StackSegment[] = [];
  let cursor = 0;
  positive.forEach((value, index) => {
    if (value <= 0) return;
    const segmentWidth = (value / total) * available;
    segments.push({ index, x: cursor, width: segmentWidth });
    cursor += segmentWidth + gap;
  });
  return segments;
}

/**
 * Valor contra meta, numa régua só.
 *
 * A escala é o maior dos dois, e é isso que deixa o estouro ser DESENHADO: o
 * anel do `BudgetRing` parava em cheio e dizia o excesso só no texto, porque
 * uma segunda volta se leria como "começou de novo". Aqui a barra vai ao fim e
 * é a marca da meta que recua para dentro.
 */
export function bulletGeometry(value: number, target: number, width: number) {
  const scale = Math.max(Math.max(0, value), Math.max(0, target));
  if (scale <= 0) return { fill: 0, mark: 0, over: false };
  return {
    fill: (Math.max(0, value) / scale) * width,
    mark: (Math.max(0, target) / scale) * width,
    over: value > target,
  };
}

/**
 * O `d` de uma polilinha, do mais antigo à esquerda ao mais novo à direita.
 *
 * O `inset` existe para o cap redondo não ser cortado pela borda do viewBox:
 * ele estende meia espessura além do ponto final.
 */
export function sparkPath(ratios: number[], options: { width: number; height: number; inset: number }) {
  const { width, height, inset } = options;
  if (ratios.length < 2) return "";

  const usableWidth = Math.max(0, width - inset * 2);
  const usableHeight = Math.max(0, height - inset * 2);
  const step = usableWidth / (ratios.length - 1);

  return ratios
    .map((ratio, index) => {
      const clamped = Math.max(0, Math.min(1, ratio));
      const x = inset + index * step;
      const y = inset + usableHeight - clamped * usableHeight;
      return `${index === 0 ? "M" : "L"}${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}
