export const REMAINDER_SLICE_NAME = "Outros";

type ClassifiedExpense = {
  description: string;
  amountCents: number;
};

export type InvoiceSlice = {
  name: string;
  valueCents: number;
  percent: number;
  /** A fatia que sobrou da fatura — o que ainda não foi classificado. */
  isRemainder: boolean;
};

export type InvoiceBreakdown = {
  totalCents: number;
  classifiedCents: number;
  unclassifiedCents: number;
  /** Classificou mais do que a fatura cobra: alguma coisa está errada e aparece. */
  isOverclassified: boolean;
  slices: InvoiceSlice[];
};

/** Duas linhas com a mesma origem são a mesma origem: "iFood" e "  ifood ". */
function originKey(description: string) {
  return description
    .trim()
    .toLocaleLowerCase("pt-BR")
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "");
}

/**
 * A fatura por origem, com o não classificado virando uma fatia própria.
 *
 * O total da fatura é o que o cartão vai cobrar — ele manda. As compras
 * lançadas explicam **parte** dele: quem não quer digitar 40 compras lança as
 * três que importam e deixa o resto em "Outros". O app antes era tudo ou nada:
 * ou o total era digitado, ou era a soma das compras, e lançar uma única
 * compra de R$ 120 numa fatura de R$ 1.027 apagava os outros R$ 907.
 */
export function breakdownInvoice(
  totalCents: number,
  expenses: ClassifiedExpense[],
): InvoiceBreakdown {
  const classifiedCents = expenses.reduce((total, item) => total + item.amountCents, 0);
  const isOverclassified = classifiedCents > totalCents;
  const unclassifiedCents = Math.max(totalCents - classifiedCents, 0);

  const byOrigin = new Map<string, { name: string; valueCents: number }>();
  for (const expense of expenses) {
    const key = originKey(expense.description);
    const existing = byOrigin.get(key);
    if (existing) {
      existing.valueCents += expense.amountCents;
    } else {
      byOrigin.set(key, { name: expense.description.trim(), valueCents: expense.amountCents });
    }
  }

  const parts = [...byOrigin.values()].map((origin) => ({ ...origin, isRemainder: false }));
  if (unclassifiedCents > 0) {
    parts.push({
      name: REMAINDER_SLICE_NAME,
      valueCents: unclassifiedCents,
      isRemainder: true,
    });
  }

  // O denominador é o que existe de fato: normalmente o total da fatura, e o
  // classificado quando alguém lançou mais do que a fatura cobra.
  const base = Math.max(totalCents, classifiedCents);
  if (base === 0) {
    return { totalCents, classifiedCents, unclassifiedCents, isOverclassified, slices: [] };
  }

  const slices = parts
    .sort((a, b) => b.valueCents - a.valueCents)
    .map((part) => ({
      name: part.name,
      valueCents: part.valueCents,
      percent: Math.round((part.valueCents / base) * 100),
      isRemainder: part.isRemainder,
    }));

  return { totalCents, classifiedCents, unclassifiedCents, isOverclassified, slices };
}

/**
 * O total que a fatura passa a ter depois de mexer nas compras.
 *
 * Existem duas faturas na cabeça de quem usa: a que ele digitou (o valor que o
 * banco vai cobrar) e a que o app somou das compras lançadas. Sem uma coluna
 * para marcar qual é qual, a regra sai do próprio número: se o total é
 * exatamente a soma anterior, ele nasceu da soma e continua acompanhando. Se
 * foi digitado, ele manda — e só cede quando alguém classifica mais do que a
 * fatura cobra, porque aí o total digitado é que estava errado.
 */
export function nextInvoiceTotal({
  currentTotal,
  previousSum,
  newSum,
}: {
  currentTotal: number | null;
  previousSum: number;
  newSum: number;
}): number {
  if (currentTotal === null) return newSum;
  if (currentTotal === previousSum) return newSum;
  if (newSum > currentTotal) return newSum;
  return currentTotal;
}
