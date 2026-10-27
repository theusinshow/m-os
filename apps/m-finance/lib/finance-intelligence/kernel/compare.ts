import { sameMonth } from "@/lib/finance-intelligence/dates";
import { monthOverview } from "@/lib/finance-intelligence/kernel/overview";
import { sanitizeLabel } from "@/lib/finance-intelligence/sanitize";
import type { FinanceSnapshot, MonthParts } from "@/lib/finance-intelligence/types";

export type Delta = { fromCents: number; toCents: number; deltaCents: number; deltaPercent: number | null };

/** Variação percentual com uma casa. Sem base (zero) não há percentual — não é infinito. */
export function delta(fromCents: number, toCents: number): Delta {
  return {
    fromCents,
    toCents,
    deltaCents: toCents - fromCents,
    deltaPercent:
      fromCents === 0 ? null : Math.round(((toCents - fromCents) / Math.abs(fromCents)) * 1000) / 10,
  };
}

/**
 * Dois meses lado a lado: totais, cartão por cartão e categoria por categoria.
 *
 * A comparação é entre meses INTEIROS como estão agora — inclusive o que é
 * estimado. Para comparar "o mesmo ponto do mês passado" seria preciso saber
 * quanto a fatura somava naquele dia, e o banco só guarda o total atual.
 */
export function comparePeriods(snapshot: FinanceSnapshot, from: MonthParts, to: MonthParts) {
  const a = monthOverview(snapshot, from);
  const b = monthOverview(snapshot, to);

  const cardIds = new Set([...a.invoices.lines, ...b.invoices.lines].map((line) => line.cardId));
  const byCard = [...cardIds]
    .map((cardId) => {
      const lineA = a.invoices.lines.find((line) => line.cardId === cardId);
      const lineB = b.invoices.lines.find((line) => line.cardId === cardId);
      return {
        cardId,
        cardName: sanitizeLabel(lineA?.cardName ?? lineB?.cardName ?? "cartão"),
        estimated: lineA?.source === "estimated" || lineB?.source === "estimated",
        ...delta(lineA?.amountCents ?? 0, lineB?.amountCents ?? 0),
      };
    })
    .sort((x, y) => Math.abs(y.deltaCents) - Math.abs(x.deltaCents));

  const categoryTotals = (parts: MonthParts) => {
    const map = new Map<string, number>();
    for (const bill of snapshot.bills) {
      if (!sameMonth(bill, parts)) continue;
      const key = sanitizeLabel(bill.categoryName ?? "Sem categoria");
      map.set(key, (map.get(key) ?? 0) + bill.amountCents);
    }
    return map;
  };
  const catA = categoryTotals(from);
  const catB = categoryTotals(to);
  const byCategory = [...new Set([...catA.keys(), ...catB.keys()])]
    .map((category) => ({ category, ...delta(catA.get(category) ?? 0, catB.get(category) ?? 0) }))
    .sort((x, y) => Math.abs(y.deltaCents) - Math.abs(x.deltaCents))
    .slice(0, 10);

  return {
    from: a.key,
    to: b.key,
    estimatedInvolved: a.isEstimated || b.isEstimated,
    income: delta(a.income.totalCents, b.income.totalCents),
    reliableIncome: delta(a.income.reliableCents, b.income.reliableCents),
    bills: delta(a.bills.totalCents, b.bills.totalCents),
    invoices: delta(a.invoices.totalCents, b.invoices.totalCents),
    committed: delta(a.committedCents, b.committedCents),
    installments: delta(a.installmentsCents, b.installmentsCents),
    accountingRemaining: delta(a.accountingRemainingCents, b.accountingRemainingCents),
    commitmentRatio: {
      fromPercent: ratio(a.committedCents, a.income.reliableCents),
      toPercent: ratio(b.committedCents, b.income.reliableCents),
    },
    byCard,
    byCategory,
  };
}

function ratio(part: number, whole: number) {
  return whole > 0 ? Math.round((part / whole) * 1000) / 10 : null;
}
