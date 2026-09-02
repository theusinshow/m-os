/**
 * A geometria da marca, e o motivo de ela nao ser um SVG so.
 *
 * O simbolo do M/OS e uma barra solida inclinada. O `BRIEF-SISTEMA-DE-LOGOS.md`
 * proibe escalar um desenho unico: a mesma inclinacao le mais fina conforme o
 * icone encolhe, entao o angulo ABRE para compensar. Sao tres desenhos, no
 * mesmo viewBox de 64, com centroide em (32,32).
 */
type Ponto = readonly [number, number];

const BARRAS: Record<"grande" | "media" | "pequena", ReadonlyArray<Ponto>> = {
  // 22 graus — 128 para cima
  grande: [
    [38, 8],
    [53, 8],
    [26, 56],
    [11, 56],
  ],
  // 18 graus — 48 a 127
  media: [
    [40, 10],
    [54, 10],
    [24, 54],
    [10, 54],
  ],
  // 14 graus — abaixo de 48
  pequena: [
    [42, 12],
    [56, 12],
    [22, 52],
    [8, 52],
  ],
};

export function poligonoPara(tamanho: number): ReadonlyArray<Ponto> {
  if (tamanho >= 128) return BARRAS.grande;
  if (tamanho >= 48) return BARRAS.media;
  return BARRAS.pequena;
}

/** O mesmo poligono no formato que o atributo `points` do SVG espera. */
export function pontosDoPoligono(tamanho: number): string {
  return poligonoPara(tamanho)
    .map(([x, y]) => `${x},${y}`)
    .join(" ");
}
