type MonthParts = { month: number; year: number };

export type MonthTotals = MonthParts & {
  incomeCents: number;
  billsCents: number;
  invoicesCents: number;
  /** NF prevista para um mês em que ela ainda não foi lançada. */
  estimatedIncomeCents?: number;
  /** Faturas previstas dos cartões que ainda não têm fatura lançada no mês. */
  estimatedInvoicesCents?: number;
};

export type ProjectionRow = MonthParts & {
  /** Tudo que entra: lançado + estimado. */
  incomeCents: number;
  incomeEstimatedCents: number;
  billsCents: number;
  /** Faturas dos cartões: lançadas + estimadas. */
  invoicesCents: number;
  invoicesEstimatedCents: number;
  committedCents: number;
  remainingCents: number;
  /** Sem receita lançada nem estimada, a sobra do mês é uma pergunta, não uma resposta. */
  hasIncome: boolean;
  /** Alguma parte da conta é palpite — a sobra também é. */
  isEstimated: boolean;
  isCurrent: boolean;
};

function monthIndex({ month, year }: MonthParts) {
  return year * 12 + (month - 1);
}

/**
 * O mês atual e os próximos, com o que já se sabe de cada um.
 *
 * Uma nota emitida hoje cai na conta de outubro, e o app só sabia responder
 * sobre o mês que estava na tela. Lançada a receita no mês em que ela chega, a
 * projeção mostra a sobra de cada mês à frente sem ninguém precisar navegar
 * mês a mês para descobrir. O que ainda não foi lançado — a fatura que não
 * fechou, a nota que não foi emitida — entra como estimado, e a linha diz.
 */
export function buildMonthProjection(
  totals: MonthTotals[],
  current: MonthParts,
  count = 6,
): ProjectionRow[] {
  const from = monthIndex(current);

  return totals
    .filter((row) => monthIndex(row) >= from)
    .map((row) => ({
      ...row,
      estimatedIncomeCents: row.estimatedIncomeCents ?? 0,
      estimatedInvoicesCents: row.estimatedInvoicesCents ?? 0,
    }))
    .filter(
      (row) =>
        row.incomeCents > 0 ||
        row.billsCents > 0 ||
        row.invoicesCents > 0 ||
        row.estimatedIncomeCents > 0 ||
        row.estimatedInvoicesCents > 0,
    )
    .sort((a, b) => monthIndex(a) - monthIndex(b))
    .slice(0, count)
    .map((row) => {
      const incomeCents = row.incomeCents + row.estimatedIncomeCents;
      const invoicesCents = row.invoicesCents + row.estimatedInvoicesCents;
      const committedCents = row.billsCents + invoicesCents;
      return {
        month: row.month,
        year: row.year,
        incomeCents,
        incomeEstimatedCents: row.estimatedIncomeCents,
        billsCents: row.billsCents,
        invoicesCents,
        invoicesEstimatedCents: row.estimatedInvoicesCents,
        committedCents,
        remainingCents: incomeCents - committedCents,
        hasIncome: incomeCents > 0,
        isEstimated: row.estimatedIncomeCents > 0 || row.estimatedInvoicesCents > 0,
        isCurrent: monthIndex(row) === from,
      };
    });
}
