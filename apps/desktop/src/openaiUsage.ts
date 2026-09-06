/** Formata a unidade exata que atravessa o IPC: milionésimos de dólar. */
export function usdDeMicros(micros: number) {
  return new Intl.NumberFormat("pt-BR", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: 2,
    maximumFractionDigits: micros % 10_000 === 0 ? 2 : 4,
  }).format(micros / 1_000_000);
}

export function fracaoDoLimite(gastoMicros: number, limiteCentavos: number | null) {
  if (!limiteCentavos || limiteCentavos <= 0) return null;
  return Math.max(0, gastoMicros / (limiteCentavos * 10_000));
}
