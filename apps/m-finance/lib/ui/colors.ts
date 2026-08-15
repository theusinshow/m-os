// Brand color tokens for JS consumers. Recharts and other canvas/SVG widgets
// need literal hex strings, not the CSS custom properties from globals.css —
// keep these in sync with the @theme block there.
export const COLORS = {
  accent: "#FB3640",
  positive: "#54D18A",
  fair: "#D8B45A",
  tight: "#D98245",
  info: "#5AA9D8",
  muted: "#8A8780",
  textPrimary: "#F5F2ED",
} as const;

export const STATUS_COLORS = {
  paid: COLORS.positive,
  pending: COLORS.muted,
  overdue: COLORS.accent,
} as const;

export const SEVERITY_COLORS = {
  info: COLORS.muted,
  warning: COLORS.fair,
  danger: COLORS.accent,
} as const;

// Sequence of distinguishable hues for categorical bars (e.g. by category).
export const CHART_PALETTE = [
  COLORS.accent,
  COLORS.tight,
  COLORS.fair,
  COLORS.positive,
  COLORS.info,
  COLORS.muted,
] as const;

export const CHART_GRID = "rgba(245,242,237,0.06)";
export const CHART_CURSOR_FILL = "rgba(245,242,237,0.04)";
export const CHART_CURSOR_STROKE = "rgba(245,242,237,0.12)";
