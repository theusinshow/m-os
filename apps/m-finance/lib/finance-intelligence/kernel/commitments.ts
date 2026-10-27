import {
  addDays,
  addMonths,
  daysBetween,
  effectiveStatus,
  monthKey,
  monthOfDate,
  sameMonth,
} from "@/lib/finance-intelligence/dates";
import { monthOverview } from "@/lib/finance-intelligence/kernel/overview";
import type { ResolvedPolicies } from "@/lib/finance-intelligence/policies";
import { sanitizeLabel } from "@/lib/finance-intelligence/sanitize";
import type { FinanceSnapshot, MonthParts } from "@/lib/finance-intelligence/types";

export type UpcomingItem = {
  kind: "bill" | "invoice" | "subscription";
  id: string;
  label: string;
  amountCents: number;
  dueDate: string;
  daysUntil: number;
  status: "pending" | "overdue";
  /** Fatura ainda não lançada: o valor é a média, não o banco. */
  estimated: boolean;
  /** Assinatura costuma cair no cartão — somar com a fatura contaria duas vezes. */
  mayBeInInvoice: boolean;
};

export const MAX_UPCOMING_ITEMS = 40;

/**
 * O que vence daqui até `days` dias, e o que já venceu e não foi pago.
 *
 * Vencido entra sempre, sem janela: uma conta de agosto esquecida é mais
 * urgente que qualquer uma de outubro. O total separa o que é obrigação
 * (conta, fatura) do que é cobrança de assinatura, porque a assinatura
 * normalmente já está dentro de uma fatura.
 */
export function upcomingCommitments(snapshot: FinanceSnapshot, days: number) {
  const { today } = snapshot;
  const until = addDays(today, days);
  const items: UpcomingItem[] = [];

  for (const bill of snapshot.bills) {
    const status = effectiveStatus(bill.status, bill.dueDate, today);
    if (status === "paid" || bill.dueDate > until) continue;
    items.push({
      kind: "bill",
      id: bill.id,
      label: sanitizeLabel(bill.name),
      amountCents: bill.amountCents,
      dueDate: bill.dueDate,
      daysUntil: daysBetween(today, bill.dueDate),
      status,
      estimated: false,
      mayBeInInvoice: false,
    });
  }

  // Faturas: as lançadas pela linha do banco; as estimadas pela projeção dos
  // meses que a janela alcança. Mês passado não estima (ver forecast.ts).
  const firstMonth = monthOfDate(today);
  const lastMonth = monthOfDate(until);
  for (let parts = firstMonth; monthKey(parts) <= monthKey(lastMonth); parts = addMonths(parts, 1)) {
    for (const line of monthOverview(snapshot, parts).invoices.lines) {
      if (line.status === "paid" || line.dueDate > until) continue;
      if (line.source === "estimated" && line.dueDate < today) continue;
      items.push({
        kind: "invoice",
        id: line.invoiceId ?? `estimate-${line.cardId}-${monthKey(parts)}`,
        label: `Fatura ${sanitizeLabel(line.cardName)}`,
        amountCents: line.amountCents,
        dueDate: line.dueDate,
        daysUntil: daysBetween(today, line.dueDate),
        status: line.status === "overdue" ? "overdue" : "pending",
        estimated: line.source === "estimated",
        mayBeInInvoice: false,
      });
    }
  }
  // Faturas lançadas e vencidas de meses anteriores ao atual.
  for (const invoice of snapshot.invoices) {
    if (monthKey(invoice) >= monthKey(firstMonth)) continue;
    const status = effectiveStatus(invoice.status, invoice.dueDate, today);
    if (status !== "overdue") continue;
    const card = snapshot.cards.find((row) => row.id === invoice.cardId);
    items.push({
      kind: "invoice",
      id: invoice.id,
      label: `Fatura ${sanitizeLabel(card?.name ?? "cartão")}`,
      amountCents: invoice.amountCents,
      dueDate: invoice.dueDate,
      daysUntil: daysBetween(today, invoice.dueDate),
      status,
      estimated: false,
      mayBeInInvoice: false,
    });
  }

  for (const subscription of snapshot.subscriptions) {
    if (subscription.status === "canceled") continue;
    if (subscription.nextChargeDate < today || subscription.nextChargeDate > until) continue;
    items.push({
      kind: "subscription",
      id: subscription.id,
      label: sanitizeLabel(subscription.name),
      amountCents: subscription.amountCents,
      dueDate: subscription.nextChargeDate,
      daysUntil: daysBetween(today, subscription.nextChargeDate),
      status: "pending",
      estimated: false,
      mayBeInInvoice: true,
    });
  }

  items.sort((a, b) => a.dueDate.localeCompare(b.dueDate) || b.amountCents - a.amountCents);
  const obligations = items.filter((item) => item.kind !== "subscription");

  return {
    days,
    from: today,
    until,
    totalCents: obligations.reduce((sum, item) => sum + item.amountCents, 0),
    overdueCents: obligations
      .filter((item) => item.status === "overdue")
      .reduce((sum, item) => sum + item.amountCents, 0),
    estimatedCents: obligations
      .filter((item) => item.estimated)
      .reduce((sum, item) => sum + item.amountCents, 0),
    subscriptionChargesCents: items
      .filter((item) => item.kind === "subscription")
      .reduce((sum, item) => sum + item.amountCents, 0),
    count: items.length,
    items: items.slice(0, MAX_UPCOMING_ITEMS),
    truncated: items.length > MAX_UPCOMING_ITEMS,
  };
}

export type EndingInstallment = { label: string; installmentCents: number; source: "card" | "bill" };

export type FutureMonthRow = MonthParts & {
  key: string;
  incomeCents: number;
  incomeEstimatedCents: number;
  reliableIncomeCents: number;
  billsCents: number;
  invoicesCents: number;
  invoicesEstimatedCents: number;
  committedCents: number;
  installmentsCents: number;
  accountingRemainingCents: number;
  reliableRemainingCents: number;
  hasIncome: boolean;
  isEstimated: boolean;
  /** Parcelamentos cuja ÚLTIMA parcela cai neste mês — o alívio do mês seguinte. */
  endingInstallments: EndingInstallment[];
};

/** Parcelamentos (cartão e contas em série) que terminam no mês. */
export function installmentsEndingIn(snapshot: FinanceSnapshot, target: MonthParts): EndingInstallment[] {
  const ending: EndingInstallment[] = [];
  for (const row of snapshot.cardExpenses) {
    if (!row.installmentId || !sameMonth(row, target)) continue;
    if (row.installmentNumber !== row.installmentTotal) continue;
    ending.push({ label: sanitizeLabel(row.description), installmentCents: row.amountCents, source: "card" });
  }
  for (const bill of snapshot.bills) {
    if (!bill.seriesId || !sameMonth(bill, target)) continue;
    if (bill.seriesNumber !== bill.seriesTotal) continue;
    ending.push({ label: sanitizeLabel(bill.name), installmentCents: bill.amountCents, source: "bill" });
  }
  return ending.sort((a, b) => b.installmentCents - a.installmentCents);
}

export function futureMonthRow(
  snapshot: FinanceSnapshot,
  parts: MonthParts,
  policies: ResolvedPolicies = snapshot.policies,
): FutureMonthRow {
  const overview = monthOverview(snapshot, parts, policies);
  return {
    month: overview.month,
    year: overview.year,
    key: overview.key,
    incomeCents: overview.income.totalCents,
    incomeEstimatedCents: overview.income.estimatedCents,
    reliableIncomeCents: overview.income.reliableCents,
    billsCents: overview.bills.totalCents,
    invoicesCents: overview.invoices.totalCents,
    invoicesEstimatedCents: overview.invoices.estimatedCents,
    committedCents: overview.committedCents,
    installmentsCents: overview.installmentsCents,
    accountingRemainingCents: overview.accountingRemainingCents,
    reliableRemainingCents: overview.reliableRemainingCents,
    hasIncome: overview.hasIncome,
    isEstimated: overview.isEstimated,
    endingInstallments: installmentsEndingIn(snapshot, parts),
  };
}

/**
 * O comprometimento mês a mês a partir do mês atual.
 *
 * É a pergunta "quanto ainda tenho comprometido até dezembro?" respondida por
 * conta, e não por palpite: contas materializadas, faturas lançadas, faturas
 * estimadas (com o piso das parcelas já conhecidas) e a renda confiável ao
 * lado de cada mês.
 */
export function futureCommitments(snapshot: FinanceSnapshot, months: number) {
  const rows = Array.from({ length: months }, (_, offset) =>
    futureMonthRow(snapshot, addMonths(snapshot.current, offset)),
  );

  const withIncome = rows.filter((row) => row.hasIncome);
  const tightest = withIncome.reduce<FutureMonthRow | null>(
    (worst, row) =>
      worst === null || row.reliableRemainingCents < worst.reliableRemainingCents ? row : worst,
    null,
  );
  const heaviest = rows.reduce<FutureMonthRow | null>(
    (peak, row) => (peak === null || row.committedCents > peak.committedCents ? row : peak),
    null,
  );

  return {
    months: rows,
    totalCommittedCents: rows.reduce((sum, row) => sum + row.committedCents, 0),
    totalInstallmentsCents: rows.reduce((sum, row) => sum + row.installmentsCents, 0),
    heaviestMonth: heaviest ? { key: heaviest.key, committedCents: heaviest.committedCents } : null,
    tightestMonth: tightest
      ? { key: tightest.key, reliableRemainingCents: tightest.reliableRemainingCents }
      : null,
    monthsWithoutIncome: rows.filter((row) => !row.hasIncome).map((row) => row.key),
  };
}
