// Tokens do M/OS para consumidores JS.
//
// Recharts e outros widgets de canvas/SVG precisam de hex literal — eles não
// leem as custom properties do CSS. Este arquivo é a única exceção autorizada
// ao "nenhum valor de cor hardcoded em componente", e existe justamente para
// que a exceção fique num lugar só, auditável, em vez de espalhada.
//
// Os valores espelham `packages/design-system/tokens.css` no tema escuro.
// Ao mudar um token lá, mudar aqui.
export const COLORS = {
  accent: "#E7C24E", // --signal-fill
  positive: "#5FA37E", // --success
  negative: "#D95546", // --danger
  textPrimary: "#E7EAEC", // --text
  textSecondary: "#8C949A", // --text-secondary
  muted: "#626A70", // --text-system
  faint: "#565E63", // --text-disabled
} as const;

export const STATUS_COLORS = {
  paid: COLORS.positive,
  pending: COLORS.muted,
  overdue: COLORS.negative,
} as const;

export const SEVERITY_COLORS = {
  info: COLORS.muted,
  warning: COLORS.textSecondary,
  danger: COLORS.negative,
} as const;

// Série categórica em RAMPA, não em matiz.
//
// O sistema tem um acento só, e inventar cinco matizes para diferenciar
// categorias devolveria pela porta do gráfico a paleta que o design system
// recusa na interface. A rampa separa por CLAREZA: o sódio marca a série em
// foco e o resto desce pela escala neutra.
//
// Diferenciar por luminosidade também sobrevive ao daltonismo, o que uma
// sequência de matizes não faz.
export const CHART_PALETTE = [
  COLORS.accent,
  COLORS.textPrimary,
  COLORS.textSecondary,
  COLORS.muted,
  COLORS.faint,
  COLORS.positive,
] as const;

export const CHART_GRID = "rgba(231,234,236,0.06)";
export const CHART_CURSOR_FILL = "rgba(231,234,236,0.04)";
export const CHART_CURSOR_STROKE = "rgba(231,234,236,0.12)";
