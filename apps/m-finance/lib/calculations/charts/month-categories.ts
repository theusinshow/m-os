type CategorizedBill = {
  amountCents: number;
  categoryName: string | null;
};

type MonthInvoice = {
  amountCents: number;
  name: string;
};

/** O rótulo das faturas no gráfico de categoria. */
export const CARDS_SLICE_NAME = "Cartões";

/**
 * O mês inteiro fatiado por categoria — contas **e** faturas.
 *
 * O gráfico somava só as contas enquanto o card respondia por dois terços do
 * dinheiro do mês. O rótulo dizia "para onde as contas do mês estão indo" e
 * mostrava 38% num item que era 13% do total real. Fatura não tem categoria
 * (o app guarda o total, não as compras), então todas entram numa fatia só:
 * "Cartões" é uma resposta honesta, "Moradia 38%" não era.
 */
export function toMonthCategoryData(
  bills: CategorizedBill[],
  invoices: MonthInvoice[],
): { name: string; value: number }[] {
  const byName = new Map<string, number>();

  for (const bill of bills) {
    const name = bill.categoryName ?? "Sem categoria";
    byName.set(name, (byName.get(name) ?? 0) + bill.amountCents);
  }

  const cardsTotal = invoices.reduce((total, invoice) => total + invoice.amountCents, 0);
  if (cardsTotal > 0) {
    byName.set(CARDS_SLICE_NAME, (byName.get(CARDS_SLICE_NAME) ?? 0) + cardsTotal);
  }

  return [...byName.entries()]
    .map(([name, value]) => ({ name, value }))
    .sort((a, b) => b.value - a.value);
}
