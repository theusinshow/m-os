import { cardExposure } from "@/lib/finance-intelligence/kernel/cards";
import { futureCommitments, upcomingCommitments } from "@/lib/finance-intelligence/kernel/commitments";
import { goalSummary } from "@/lib/finance-intelligence/kernel/goals";
import { monthOverview } from "@/lib/finance-intelligence/kernel/overview";
import { safeToSpend } from "@/lib/finance-intelligence/kernel/safe-to-spend";
import { subscriptionSummary } from "@/lib/finance-intelligence/kernel/subscriptions";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";

export type PackInsight = {
  type: string;
  severity: "info" | "warning" | "critical";
  title: string;
  summary: string;
  /** `live` = detectado agora, em memória; `stored` = aberto no banco. */
  origin: "live" | "stored";
};

/** O que o M-Finance não sabe, dito de uma vez para o modelo não inventar. */
export const DATA_NOT_AVAILABLE = [
  "saldo em conta bancária",
  "limite do cartão",
  "data de fechamento da fatura",
  "compras que não foram lançadas",
];

export const PACK_LIMITS = {
  upcomingItems: 12,
  futureMonths: 6,
  cards: 8,
  subscriptions: 10,
  goals: 5,
  insights: 3,
};

/**
 * O contexto-base do modo financeiro: o bastante para responder "como estou?"
 * sem outro salto, e pequeno o bastante para descer em toda pergunta
 * financeira.
 *
 * Nenhuma compra individual histórica entra. Listas têm teto fixo; o que
 * passou do teto fica disponível pelas ferramentas específicas.
 */
export function contextPack(snapshot: FinanceSnapshot, insights: PackInsight[] = []) {
  const overview = monthOverview(snapshot, snapshot.current);
  const safe = safeToSpend(snapshot);
  const upcoming = upcomingCommitments(snapshot, 30);
  const horizon = Math.min(PACK_LIMITS.futureMonths, snapshot.policies.forecastHorizonMonths);
  const future = futureCommitments(snapshot, Math.max(horizon, 3));
  const exposure = cardExposure(snapshot, { months: 2 });
  const subs = subscriptionSummary(snapshot);
  const goals = goalSummary(snapshot);
  const policies = snapshot.policies;

  return {
    today: snapshot.today,
    currentMonth: {
      key: overview.key,
      incomeCents: overview.income.totalCents,
      incomeEstimatedCents: overview.income.estimatedCents,
      incomeReceivedCents: overview.income.receivedCents,
      reliableIncomeCents: overview.income.reliableCents,
      billsCents: overview.bills.totalCents,
      invoicesCents: overview.invoices.totalCents,
      invoicesEstimatedCents: overview.invoices.estimatedCents,
      committedCents: overview.committedCents,
      paidCents: overview.paidCents,
      pendingCents: overview.pendingCents,
      overdueCents: overview.overdueCents,
      installmentsCents: overview.installmentsCents,
      accountingRemainingCents: overview.accountingRemainingCents,
      health: overview.health,
      isEstimated: overview.isEstimated,
      safeToSpend: {
        safeToSpendCents: safe.safeToSpendCents,
        status: safe.status,
        deductions: safe.deductions.map(({ reason, label, amountCents }) => ({ reason, label, amountCents })),
        assumptions: safe.assumptions,
      },
    },
    next30Days: {
      totalCents: upcoming.totalCents,
      overdueCents: upcoming.overdueCents,
      estimatedCents: upcoming.estimatedCents,
      items: upcoming.items.slice(0, PACK_LIMITS.upcomingItems).map((item) => ({
        kind: item.kind,
        id: item.id,
        label: item.label,
        amountCents: item.amountCents,
        dueDate: item.dueDate,
        status: item.status,
        estimated: item.estimated,
      })),
      moreItems: Math.max(0, upcoming.count - PACK_LIMITS.upcomingItems),
    },
    futureCommitments: {
      months: future.months.map((row) => ({
        key: row.key,
        reliableIncomeCents: row.reliableIncomeCents,
        committedCents: row.committedCents,
        installmentsCents: row.installmentsCents,
        reliableRemainingCents: row.reliableRemainingCents,
        hasIncome: row.hasIncome,
        isEstimated: row.isEstimated,
        endingInstallments: row.endingInstallments.length,
      })),
      tightestMonth: future.tightestMonth,
      heaviestMonth: future.heaviestMonth,
    },
    cards: exposure.cards.slice(0, PACK_LIMITS.cards).map((card) => ({
      id: card.id,
      name: card.name,
      cardType: card.cardType,
      currentInvoiceCents: card.forecast[0]?.amountCents ?? 0,
      currentSource: card.forecast[0]?.source ?? "none",
      currentStatus: card.forecast[0]?.status ?? null,
      dueDate: card.forecast[0]?.dueDate ?? null,
      nextInvoiceCents: card.forecast[1]?.amountCents ?? 0,
      deltaVsPreviousCents: card.trend.deltaVsPreviousCents,
      futureInstallmentsCents: card.installments.futureInstallmentsCents,
    })),
    subscriptions: {
      activeMonthlyCents: subs.activeMonthlyCents,
      trialsMonthlyCents: subs.trialsMonthlyCents,
      activeCount: subs.activeCount,
      trialCount: subs.trialCount,
      items: subs.items.slice(0, PACK_LIMITS.subscriptions).map((item) => ({
        id: item.id,
        name: item.name,
        monthlyEquivalentCents: item.monthlyEquivalentCents,
        status: item.status,
        nextChargeDate: item.nextChargeDate,
      })),
    },
    goals: goals.goals
      .filter((goal) => goal.status === "active")
      .slice(0, PACK_LIMITS.goals)
      .map((goal) => ({
        id: goal.id,
        name: goal.name,
        priority: goal.priority,
        remainingCents: goal.remainingCents,
        progressPercent: goal.progressPercent,
        deadline: goal.deadline,
        requiredMonthlyCents: goal.requiredMonthlyCents,
      })),
    policies: {
      minimumMonthEndBufferCents: policies.minimumMonthEndBufferCents,
      reliableIncomeRules: policies.reliableIncomeRules,
      maxInstallmentCommitmentCents: policies.maxInstallmentCommitmentCents,
      forecastHorizonMonths: policies.forecastHorizonMonths,
      safeToSpendLookaheadMonths: policies.safeToSpend.lookaheadMonths,
      configured: policies.configured,
    },
    recentInsights: insights.slice(0, PACK_LIMITS.insights),
    dataNotAvailable: DATA_NOT_AVAILABLE,
  };
}

export type ContextPack = ReturnType<typeof contextPack>;
