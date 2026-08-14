/**
 * Simbolo do M/OS: barra solida em campo sodio.
 *
 * Tres desenhos com o angulo corrigido por escala. Escalar um unico SVG entre
 * tamanhos e proibido pelo handoff, e o motivo e optico: a mesma inclinacao
 * geometrica le mais fina conforme o desenho encolhe, entao o angulo abre para
 * compensar. viewBox 0 0 64 64 nos tres, centroide em (32,32) — por isso a
 * rotacao e sempre transform-origin: center.
 */
const bars = {
  /** 1024 · 512 · 256 · 128 — 22 graus */
  large: "38,8 53,8 26,56 11,56",
  /** 64 · 48 — 18 graus */
  medium: "40,10 54,10 24,54 10,54",
  /** 32 · 24 · 16 — 14 graus */
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
