/**
 * Simbolo do M/OS: barra solida em campo sodio.
 *
 * Tres desenhos com o angulo corrigido por escala. Escalar um unico SVG entre
 * tamanhos e proibido pelo handoff, e o motivo e optico: a mesma inclinacao
 * geometrica le mais fina conforme o desenho encolhe, entao o angulo abre para
 * compensar. viewBox 0 0 64 64 nos tres, centroide em (32,32) — por isso a
 * rotacao e sempre transform-origin: center.
 *
 * Os graus de cada faixa sao o NOME que o handoff deu a ela (22, 18, 14), e nao
 * uma medida: a linha de centro de cada poligono inclina 29.4, 34.3 e 40.4
 * graus da vertical. A ordem confere — o angulo abre ao encolher —, o numero
 * nao. Os dois ficam escritos para ninguem "corrigir" um poligono pelo rotulo.
 */
const bars = {
  /** 1024 · 512 · 256 · 128 — 22 graus no handoff, 29.4 medidos */
  large: "38,8 53,8 26,56 11,56",
  /** 64 · 48 — 18 graus no handoff, 34.3 medidos */
  medium: "40,10 54,10 24,54 10,54",
  /** 32 · 24 · 16 — 14 graus no handoff, 40.4 medidos */
  small: "42,12 56,12 22,52 8,52",
} as const;

function barFor(size: number) {
  if (size >= 128) return bars.large;
  if (size >= 48) return bars.medium;
  return bars.small;
}

/**
 * `spinning` e o estado de sistema ocupado — o unico spinner do sistema.
 * Nao existe circulo, nao existem tres pontos.
 */
export function MosSymbol({ size = 16, spinning = false }: { size?: number; spinning?: boolean }) {
  return (
    <svg
      className="mos-symbol"
      data-spinning={spinning || undefined}
      width={size}
      height={size}
      viewBox="0 0 64 64"
      aria-hidden="true"
      focusable="false"
    >
      <polygon points={barFor(size)} />
    </svg>
  );
}
