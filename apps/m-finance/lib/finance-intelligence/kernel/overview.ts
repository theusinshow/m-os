import type { MonthHealth } from "@/db/schema";
import { forecastCardMonth, forecastMainIncome } from "@/lib/calculations/forecast";
import { getDashboardSummary } from "@/lib/calculations/dashboard";
import {
  effectiveStatus,
  monthIndex,
  monthKey,
  sameMonth,
} from "@/lib/finance-intelligence/dates";
import type { ResolvedPolicies } from "@/lib/finance-intelligence/policies";
import type {
  FinanceSnapshot,
  IncomeType,
  MonthParts,
  PayableStatus,
} from "@/lib/finance-intelligence/types";

export type InvoiceLine = {
  cardId: string;
  cardName: string;
  cardType: "personal" | "business";
  amountCents: number;
  source: "actual" | "estimated";
  /** Só a fatura lançada tem status e id — só ela pode ser paga. */
  invoiceId: string | null;
  status: PayableStatus;
  dueDate: string;
};

export type MonthOverview = MonthParts & {
  key: string;
  position: "past" | "current" | "future";
  income: {
    totalCents: number;
    actualCents: number;
    estimatedCents: number;
    receivedCents: number;
    reliableCents: number;
    byType: Record<IncomeType, number>;
  };
  bills: {
    totalCents: number;
    paidCents: number;
    pendingCents: number;
    overdueCents: number;
    count: number;
  };
  invoices: {
    totalCents: number;
    actualCents: number;
    estimatedCents: number;
    paidCents: number;
    lines: InvoiceLine[];
  };
  /** Contas + faturas (reais e estimadas). */
  committedCents: number;
  paidCents: number;
  pendingCents: number;
  overdueCents: number;
  /** Parcelas de cartão e séries de contas que caem neste mês. */
  installmentsCents: number;
  /** A "Sobra estimada" do dashboard: receita total − comprometido. */
  accountingRemainingCents: number;
  reliableRemainingCents: number;
  health: MonthHealth;
  hasIncome: boolean;
  /** Alguma parcela do mês é palpite (NF ou fatura estimada). */
  isEstimated: boolean;
};

/**
 * Quanto de uma receita conta como certo.
 *
 * Recebida é fato e vale inteira. A que ainda vai entrar vale pelo peso do
 * tipo — é o que faz "freelance conta 50%" mudar o Safe-to-Spend sem mudar o
 * que o dashboard chama de receita.
 */
export function reliableShare(
  amountCents: number,
  incomeType: IncomeType,
  received: boolean,
  rules: ResolvedPolicies["reliableIncomeRules"],
) {
  if (received) return amountCents;
  return Math.round(amountCents * rules[incomeType]);
}

/**
 * O mês como o dashboard o mostra, com o que ele não mostrava: renda confiável,
 * parcelas e a separação entre real e estimado.
 *
 * Os totais passam por `getDashboardSummary` com as MESMAS entradas que a
 * página monta (receita real + NF estimada; contas; faturas reais + estimadas).
 * Assim a "sobra" que o Hermes cita é, centavo por centavo, a que a tela exibe.
 */
export function monthOverview(
  snapshot: FinanceSnapshot,
  target: MonthParts,
  policies: ResolvedPolicies = snapshot.policies,
): MonthOverview {
  const { today, current } = snapshot;
  const position =
    monthIndex(target) < monthIndex(current)
      ? "past"
      : monthIndex(target) === monthIndex(current)
        ? "current"
        : "future";

  const incomes = snapshot.incomes.filter((row) => sameMonth(row, target));
  const bills = snapshot.bills
    .filter((row) => sameMonth(row, target))
    .map((bill) => ({ ...bill, status: effectiveStatus(bill.status, bill.dueDate, today) }));

  const cardLines = forecastCardMonth(
    { cards: snapshot.cards, invoices: snapshot.invoices, installments: installmentsByCardMonth(snapshot) },
    target,
    current,
  );
  const lines: InvoiceLine[] = cardLines
    .filter((line) => line.source !== "none")
    .map((line) => ({
      cardId: line.card.id,
      cardName: line.card.name,
      cardType: line.card.cardType,
      amountCents: line.amountCents,
      source: line.source as "actual" | "estimated",
      invoiceId: line.invoice?.id ?? null,
      status: effectiveStatus(line.invoice?.status ?? "pending", line.dueDate, today),
      dueDate: line.dueDate,
    }));

  const mainIncome = forecastMainIncome(snapshot.incomes, target, current);
  const estimatedIncomeCents = mainIncome.source === "estimated" ? mainIncome.amountCents : 0;

  const summary = getDashboardSummary({
    incomes: [
      ...incomes.map((income) => ({ id: income.id, amountCents: income.amountCents, received: income.received })),
      ...(estimatedIncomeCents > 0
        ? [{ id: "estimate-nf", amountCents: estimatedIncomeCents, received: false }]
        : []),
    ],
    bills: bills.map((bill) => ({
      id: bill.id,
      name: bill.name,
      amountCents: bill.amountCents,
      dueDate: bill.dueDate,
      status: bill.status,
    })),
    invoices: lines.map((line) => ({
      id: line.invoiceId ?? `estimate-${line.cardId}`,
      name: line.cardName,
      amountCents: line.amountCents,
      dueDate: line.dueDate,
      status: line.status,
      cardType: line.cardType,
    })),
  });

  const byType: Record<IncomeType, number> = { main: 0, freelance: 0, extra: 0 };
  let receivedCents = 0;
  let reliableCents = 0;
  for (const income of incomes) {
    byType[income.incomeType] += income.amountCents;
    if (income.received) receivedCents += income.amountCents;
    reliableCents += reliableShare(income.amountCents, income.incomeType, income.received, policies.reliableIncomeRules);
  }
  byType.main += estimatedIncomeCents;
  reliableCents += reliableShare(estimatedIncomeCents, "main", false, policies.reliableIncomeRules);

  const actualInvoices = lines.filter((line) => line.source === "actual");
  const estimatedInvoicesCents = lines
    .filter((line) => line.source === "estimated")
    .reduce((sum, line) => sum + line.amountCents, 0);
  const committedCents = summary.totalBillsCents + summary.totalInvoicesCents;

  const installmentsCents =
    snapshot.cardExpenses
      .filter((row) => row.installmentId && sameMonth(row, target))
      .reduce((sum, row) => sum + row.amountCents, 0) +
    bills.filter((bill) => bill.seriesId).reduce((sum, bill) => sum + bill.amountCents, 0);

  return {
    month: target.month,
    year: target.year,
    key: monthKey(target),
    position,
    income: {
      totalCents: summary.totalIncomeCents,
      actualCents: summary.totalIncomeCents - estimatedIncomeCents,
      estimatedCents: estimatedIncomeCents,
      receivedCents,
      reliableCents,
      byType,
    },
    bills: {
      totalCents: summary.totalBillsCents,
      paidCents: sumBy(bills, "paid"),
      pendingCents: sumBy(bills, "pending"),
      overdueCents: sumBy(bills, "overdue"),
      count: bills.length,
    },
    invoices: {
      totalCents: summary.totalInvoicesCents,
      actualCents: actualInvoices.reduce((sum, line) => sum + line.amountCents, 0),
      estimatedCents: estimatedInvoicesCents,
      paidCents: lines.filter((line) => line.status === "paid").reduce((sum, line) => sum + line.amountCents, 0),
      lines,
    },
    committedCents,
    paidCents: summary.totalPaidCents,
    pendingCents: summary.totalPendingCents,
    overdueCents: summary.totalOverdueCents,
    installmentsCents,
    accountingRemainingCents: summary.estimatedRemainingCents,
    reliableRemainingCents: reliableCents - committedCents,
    health: summary.monthHealth,
    hasIncome: summary.totalIncomeCents > 0,
    isEstimated: estimatedIncomeCents > 0 || estimatedInvoicesCents > 0,
  };
}

function sumBy(rows: { status: PayableStatus; amountCents: number }[], status: PayableStatus) {
  return rows.filter((row) => row.status === status).reduce((sum, row) => sum + row.amountCents, 0);
}

/** Parcelas de cartão somadas por cartão e mês — a entrada que a estimativa de fatura pede. */
export function installmentsByCardMonth(snapshot: FinanceSnapshot) {
  const map = new Map<string, MonthParts & { cardId: string; amountCents: number }>();
  for (const row of snapshot.cardExpenses) {
    if (!row.installmentId) continue;
    const key = `${row.cardId}:${monthKey(row)}`;
    const existing = map.get(key);
    if (existing) existing.amountCents += row.amountCents;
    else map.set(key, { cardId: row.cardId, month: row.month, year: row.year, amountCents: row.amountCents });
  }
  return [...map.values()];
}
