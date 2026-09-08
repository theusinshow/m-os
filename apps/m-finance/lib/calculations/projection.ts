type MonthParts = { month: number; year: number };

export type MonthTotals = MonthParts & {
  incomeCents: number;
  billsCents: number;
  invoicesCents: number;
};

export type ProjectionRow = MonthParts & {
  incomeCents: number;
  committedCents: number;
  remainingCents: number;
  /** Sem receita lançada, a sobra do mês é uma pergunta, não uma resposta. */
  hasIncome: boolean;
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
 * mês a mês para descobrir.
 */
export function buildMonthProjection(
  totals: MonthTotals[],
  current: MonthParts,
  count = 6,
): ProjectionRow[] {
  const from = monthIndex(current);

  return totals
    .filter((row) => monthIndex(row) >= from)
    .filter((row) => row.incomeCents > 0 || row.billsCents > 0 || row.invoicesCents > 0)
    .sort((a, b) => monthIndex(a) - monthIndex(b))
    .slice(0, count)
    .map((row) => {
      const committedCents = row.billsCents + row.invoicesCents;
      return {
        month: row.month,
        year: row.year,
        incomeCents: row.incomeCents,
        committedCents,
        remainingCents: row.incomeCents - committedCents,
        hasIncome: row.incomeCents > 0,
        isCurrent: monthIndex(row) === from,
      };
    });
}
