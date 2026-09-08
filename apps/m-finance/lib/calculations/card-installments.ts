type MonthParts = { month: number; year: number };

type CardExpenseRow = {
  description: string;
  amountCents: number;
  installmentId: string | null;
  installmentNumber: number | null;
  installmentTotal: number | null;
} & MonthParts;

export type CardInstallmentSeries = {
  installmentId: string;
  description: string;
  installmentCents: number;
  /** Em que parcela o parcelamento está no mês que a tela mostra. */
  currentNumber: number;
  installmentTotal: number;
  remainingCount: number;
  remainingCents: number;
  lastMonth: MonthParts;
};

function monthIndex({ month, year }: MonthParts) {
  return year * 12 + (month - 1);
}

function monthAtOffset({ month, year }: MonthParts, offset: number): MonthParts {
  const index = monthIndex({ month, year }) + offset;
  return { month: (index % 12) + 1, year: Math.floor(index / 12) };
}

/**
 * Os parcelamentos vivos de um cartão, do ponto de vista de um mês.
 *
 * A tela do cartão mostrava cada parcela como uma linha solta — "Parcela 3/10,
 * R$ 400" — e nada dizia que ainda faltavam sete delas, R$ 2.800. O
 * parcelamento é a unidade que interessa; a parcela do mês é só onde ele está
 * agora.
 */
export function summarizeCardInstallments(
  expenses: CardExpenseRow[],
  activeMonth: MonthParts,
): CardInstallmentSeries[] {
  const activeIndex = monthIndex(activeMonth);
  const bySeries = new Map<string, CardExpenseRow>();

  for (const expense of expenses) {
    if (!expense.installmentId || !expense.installmentNumber || !expense.installmentTotal) {
      continue;
    }
    // A linha do mês ativo é a que dá o "onde estou". Sem ela, o parcelamento
    // ou já acabou ou ainda não começou — nos dois casos, não é deste mês.
    if (monthIndex(expense) !== activeIndex) continue;
    bySeries.set(expense.installmentId, expense);
  }

  return [...bySeries.values()]
    .map((expense) => {
      const currentNumber = expense.installmentNumber as number;
      const installmentTotal = expense.installmentTotal as number;
      const remainingCount = Math.max(installmentTotal - currentNumber, 0);

      return {
        installmentId: expense.installmentId as string,
        description: expense.description,
        installmentCents: expense.amountCents,
        currentNumber,
        installmentTotal,
        remainingCount,
        remainingCents: remainingCount * expense.amountCents,
        lastMonth: monthAtOffset(activeMonth, remainingCount),
      };
    })
    .sort((a, b) => b.remainingCents - a.remainingCents || b.installmentCents - a.installmentCents);
}
