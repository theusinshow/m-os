export type WaterfallKind = "in" | "out" | "total";

export type WaterfallStep = {
  label: string;
  /** Onde a barra começa no eixo. Empilhado como série transparente. */
  offset: number;
  /** Altura da barra. Sempre positiva — o sinal mora em `value`. */
  delta: number;
  /** O valor com sinal, para tooltip e para escolher a cor do total. */
  value: number;
  kind: WaterfallKind;
};

/**
 * O cálculo central do mês, como cascata.
 *
 * Recharts não tem waterfall, e não precisa ter: uma barra flutuante é uma
 * série transparente de `offset` com a série visível de `delta` empilhada em
 * cima. Separar `delta` (altura, sempre positiva) de `value` (o número com
 * sinal) existe porque barra de altura negativa não desenha — mas o tooltip
 * precisa dizer "−R$ 2.000,00", não "R$ 2.000,00".
 */
export function toWaterfallSteps({
  incomeCents,
  billsCents,
  invoicesCents,
}: {
  incomeCents: number;
  billsCents: number;
  invoicesCents: number;
}): WaterfallStep[] {
  const afterBills = incomeCents - billsCents;
  const remaining = afterBills - invoicesCents;

  return [
    { label: "Receita", offset: 0, delta: incomeCents, value: incomeCents, kind: "in" },
    { label: "Contas", offset: afterBills, delta: billsCents, value: -billsCents, kind: "out" },
    {
      label: "Faturas",
      offset: remaining,
      delta: invoicesCents,
      value: -invoicesCents,
      kind: "out",
    },
    {
      label: "Sobra",
      // Negativa, a barra desce do zero; positiva, sobe dele.
      offset: remaining < 0 ? remaining : 0,
      delta: Math.abs(remaining),
      value: remaining,
      kind: "total",
    },
  ];
}
