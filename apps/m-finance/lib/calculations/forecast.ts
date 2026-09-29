import { composeMonthDate } from "@/lib/due-date";

/**
 * Real ou estimado: o que o app sabe de um mês que ainda não aconteceu.
 *
 * O app só conhecia o que alguém digitou. Setembro tinha as quatro faturas
 * lançadas; outubro não tinha nenhuma — e a tela de cartões mostrava outubro
 * vazio, a projeção contava só as contas contra a nota fiscal, e o mês parecia
 * ter R$ 3.400 de folga que não existiam. Um mês à frente sem fatura lançada
 * não é um mês sem fatura: é um mês cuja fatura ainda não fechou.
 *
 * A estimativa nunca é gravada. Ela é calculada na leitura, aparece marcada
 * como estimada, e deixa de existir no instante em que o valor real é lançado.
 */

export type MonthParts = { month: number; year: number };
export type ForecastSource = "actual" | "estimated" | "none";
type PayableStatus = "pending" | "paid" | "overdue";

/** Quantos meses reais entram na média. */
export const FORECAST_WINDOW = 3;

export function monthIndex({ month, year }: MonthParts) {
  return year * 12 + (month - 1);
}

function sameMonth(a: MonthParts, b: MonthParts) {
  return a.month === b.month && a.year === b.year;
}

/**
 * A média dos últimos `window` meses com valor, estritamente antes do alvo.
 * Linhas do mesmo mês devem chegar já somadas.
 */
export function averageOfRecent(
  history: (MonthParts & { amountCents: number })[],
  target: MonthParts,
  window = FORECAST_WINDOW,
) {
  const before = history
    .filter((row) => monthIndex(row) < monthIndex(target) && row.amountCents > 0)
    .sort((a, b) => monthIndex(b) - monthIndex(a))
    .slice(0, window);

  if (before.length === 0) return { averageCents: 0, count: 0 };

  const total = before.reduce((sum, row) => sum + row.amountCents, 0);
  return { averageCents: Math.round(total / before.length), count: before.length };
}

export type ForecastCard = {
  id: string;
  name: string;
  cardType: "personal" | "business";
  dueDay: number;
  isActive: boolean;
};

export type ForecastInvoice = MonthParts & {
  id: string;
  cardId: string;
  amountCents: number;
  dueDate: string;
  status: PayableStatus;
};

export type CardForecastInputs = {
  cards: ForecastCard[];
  invoices: ForecastInvoice[];
  /** Parcelas já lançadas, somadas por cartão e mês. */
  installments: (MonthParts & { cardId: string; amountCents: number })[];
};

export type CardMonthLine = {
  card: ForecastCard;
  source: ForecastSource;
  amountCents: number;
  dueDate: string;
  /** A fatura de verdade, quando existe. Só ela pode ser paga. */
  invoice: { id: string; status: PayableStatus; dueDate: string } | null;
  /** Quantas faturas reais formaram a média (0 quando não é estimativa). */
  basisCount: number;
  installmentsCents: number;
};

/**
 * A fatura de cada cartão num mês.
 *
 * - Lançada: é ela, mesmo que menor que a média — quem manda é o banco.
 * - Mês atual ou à frente sem fatura: estimada, pelo maior entre as parcelas
 *   já conhecidas daquele mês e a média das últimas faturas do cartão. As
 *   parcelas são o piso porque são dívida certa; a média cobre o resto.
 * - Mês passado sem fatura: não houve fatura. Estimar o passado seria
 *   inventar uma dívida que ninguém pagou.
 *
 * Cartão inativo só aparece no mês em que tem fatura real.
 */
export function forecastCardMonth(
  inputs: CardForecastInputs,
  target: MonthParts,
  today: MonthParts,
): CardMonthLine[] {
  const isPast = monthIndex(target) < monthIndex(today);

  return inputs.cards.flatMap((card): CardMonthLine[] => {
    const invoice = inputs.invoices.find(
      (row) => row.cardId === card.id && sameMonth(row, target),
    );
    const installmentsCents = inputs.installments
      .filter((row) => row.cardId === card.id && sameMonth(row, target))
      .reduce((sum, row) => sum + row.amountCents, 0);
    const dueDate = composeMonthDate(target.year, target.month, card.dueDay);

    if (invoice) {
      return [
        {
          card,
          source: "actual",
          amountCents: invoice.amountCents,
          dueDate: invoice.dueDate,
          invoice: { id: invoice.id, status: invoice.status, dueDate: invoice.dueDate },
          basisCount: 0,
          installmentsCents,
        },
      ];
    }

    if (!card.isActive) return [];

    const none: CardMonthLine = {
      card,
      source: "none",
      amountCents: 0,
      dueDate,
      invoice: null,
      basisCount: 0,
      installmentsCents,
    };

    if (isPast) return [none];

    const history = inputs.invoices.filter((row) => row.cardId === card.id);
    const { averageCents, count } = averageOfRecent(history, target);
    const amountCents = Math.max(averageCents, installmentsCents);

    if (amountCents === 0) return [none];

    return [
      {
        ...none,
        source: "estimated",
        amountCents,
        basisCount: averageCents >= installmentsCents ? count : 0,
      },
    ];
  });
}

type ForecastIncome = MonthParts & {
  amountCents: number;
  incomeType: "main" | "extra" | "freelance";
};

/**
 * A nota fiscal do mês: a lançada, ou a média das últimas quando o mês ainda
 * está à frente. O valor varia mês a mês, então a média é um palpite honesto —
 * e aparece como palpite até a nota ser emitida.
 *
 * Só a receita principal entra. Freelance e extra são sorte, não padrão.
 */
export function forecastMainIncome(
  incomes: ForecastIncome[],
  target: MonthParts,
  today: MonthParts,
): { source: ForecastSource; amountCents: number; basisCount: number } {
  const perMonth = new Map<number, MonthParts & { amountCents: number }>();
  for (const income of incomes) {
    if (income.incomeType !== "main") continue;
    const key = monthIndex(income);
    const existing = perMonth.get(key);
    if (existing) existing.amountCents += income.amountCents;
    else perMonth.set(key, { month: income.month, year: income.year, amountCents: income.amountCents });
  }

  const actual = perMonth.get(monthIndex(target));
  if (actual) return { source: "actual", amountCents: actual.amountCents, basisCount: 0 };

  if (monthIndex(target) < monthIndex(today)) {
    return { source: "none", amountCents: 0, basisCount: 0 };
  }

  const { averageCents, count } = averageOfRecent([...perMonth.values()], target);
  if (count === 0) return { source: "none", amountCents: 0, basisCount: 0 };
  return { source: "estimated", amountCents: averageCents, basisCount: count };
}
