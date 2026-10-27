import type { FinancialInsightSeverity, FinancialInsightType } from "@/db/schema";
import { averageOfRecent } from "@/lib/calculations/forecast";
import {
  addDays,
  addMonths,
  daysBetween,
  effectiveStatus,
  monthIndex,
  monthKey,
  sameMonth,
} from "@/lib/finance-intelligence/dates";
import { futureCommitments } from "@/lib/finance-intelligence/kernel/commitments";
import { goalSummary } from "@/lib/finance-intelligence/kernel/goals";
import { monthOverview } from "@/lib/finance-intelligence/kernel/overview";
import { safeToSpend } from "@/lib/finance-intelligence/kernel/safe-to-spend";
import { subscriptionSummary } from "@/lib/finance-intelligence/kernel/subscriptions";
import { sensitivityFactor } from "@/lib/finance-intelligence/policies";
import { sanitizeLabel } from "@/lib/finance-intelligence/sanitize";
import { formatCurrency } from "@/lib/formatters/currency";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";

export type FinanceObservation = {
  detector: FinancialInsightType;
  dedupeKey: string;
  severity: FinancialInsightSeverity;
  /** 0–100. Código decide; a LLM nunca mexe aqui. */
  materialityScore: number;
  title: string;
  summary: string;
  facts: Record<string, unknown>;
  entityRefs: { type: string; id: string }[];
};

export type ObserverInput = {
  snapshot: FinanceSnapshot;
  /** O Safe-to-Spend da rodada anterior, quando houve. */
  previousSafeToSpendCents: number | null;
};

/**
 * Limiares em centavos, antes da sensibilidade. `observer_sensitivity = high`
 * multiplica por 0,6 (fala mais cedo); `low`, por 1,6.
 */
export const THRESHOLDS = {
  dueSoonDays: 3,
  dueSoonMinCents: 5000,
  dueSoonWarningCents: 100000,
  overdueCriticalDays: 7,
  overdueCriticalCents: 100000,
  incomeGraceDays: 3,
  nfMissingFromDay: 10,
  spikeRatio: 1.25,
  spikeCriticalRatio: 1.6,
  spikeMinDeltaCents: 30000,
  installmentShareOfIncome: 0.3,
  subscriptionShareOfIncome: 0.1,
  trialWarningDays: 3,
  s2sDropMinCents: 30000,
  s2sDropShare: 0.15,
  futureMonths: 6,
  goalMinMonthlyCents: 5000,
} as const;

const SEVERITY_ORDER: Record<FinancialInsightSeverity, number> = { info: 0, warning: 1, critical: 2 };

export function severityRank(severity: FinancialInsightSeverity) {
  return SEVERITY_ORDER[severity];
}

/** Materialidade 0–100 em escala log: R$ 50 ≈ 20, R$ 500 ≈ 50, R$ 5.000 ≈ 80. */
export function materialityOf(amountCents: number) {
  const reais = Math.abs(amountCents) / 100;
  if (reais < 1) return 0;
  return Math.min(100, Math.round(Math.log10(reais) * 30 - 30));
}

function detectDueSoon(snapshot: FinanceSnapshot, factor: number): FinanceObservation[] {
  const { today } = snapshot;
  const until = addDays(today, THRESHOLDS.dueSoonDays);
  const out: FinanceObservation[] = [];

  const payables = [
    ...snapshot.bills.map((bill) => ({ kind: "bill", id: bill.id, label: bill.name, amountCents: bill.amountCents, dueDate: bill.dueDate, status: bill.status })),
    ...snapshot.invoices.map((invoice) => ({
      kind: "invoice",
      id: invoice.id,
      label: `Fatura ${snapshot.cards.find((card) => card.id === invoice.cardId)?.name ?? "cartão"}`,
      amountCents: invoice.amountCents,
      dueDate: invoice.dueDate,
      status: invoice.status,
    })),
  ];

  for (const item of payables) {
    if (effectiveStatus(item.status, item.dueDate, today) !== "pending") continue;
    if (item.dueDate < today || item.dueDate > until) continue;
    if (item.amountCents < THRESHOLDS.dueSoonMinCents * factor) continue;
    const days = daysBetween(today, item.dueDate);
    const label = sanitizeLabel(item.label);
    const when = days === 0 ? "hoje" : days === 1 ? "amanhã" : `em ${days} dias`;
    out.push({
      detector: "bill_due_soon",
      dedupeKey: `bill_due_soon:${item.kind}:${item.id}:${item.dueDate}`,
      severity: days === 0 || item.amountCents >= THRESHOLDS.dueSoonWarningCents * factor ? "warning" : "info",
      materialityScore: materialityOf(item.amountCents),
      title: `${label} vence ${when}`,
      summary: `${label}: ${formatCurrency(item.amountCents)}, vencimento ${item.dueDate}.`,
      facts: { kind: item.kind, label, amountCents: item.amountCents, dueDate: item.dueDate, daysUntil: days },
      entityRefs: [{ type: item.kind, id: item.id }],
    });
  }
  return out;
}

function detectOverdue(snapshot: FinanceSnapshot, factor: number): FinanceObservation[] {
  const { today } = snapshot;
  const out: FinanceObservation[] = [];
  const items = [
    ...snapshot.bills.map((bill) => ({ kind: "bill", id: bill.id, label: bill.name, amountCents: bill.amountCents, dueDate: bill.dueDate, status: bill.status })),
    ...snapshot.invoices.map((invoice) => ({
      kind: "invoice",
      id: invoice.id,
      label: `Fatura ${snapshot.cards.find((card) => card.id === invoice.cardId)?.name ?? "cartão"}`,
      amountCents: invoice.amountCents,
      dueDate: invoice.dueDate,
      status: invoice.status,
    })),
  ];
  for (const item of items) {
    if (effectiveStatus(item.status, item.dueDate, today) !== "overdue") continue;
    const daysLate = daysBetween(item.dueDate, today);
    const label = sanitizeLabel(item.label);
    const critical =
      daysLate >= THRESHOLDS.overdueCriticalDays || item.amountCents >= THRESHOLDS.overdueCriticalCents * factor;
    out.push({
      detector: "overdue_commitment",
      dedupeKey: `overdue_commitment:${item.kind}:${item.id}`,
      severity: critical ? "critical" : "warning",
      materialityScore: Math.min(100, materialityOf(item.amountCents) + Math.min(daysLate, 20)),
      title: `${label} está vencida`,
      summary: `${label}: ${formatCurrency(item.amountCents)}, venceu em ${item.dueDate} (${daysLate} dia(s) de atraso).`,
      facts: { kind: item.kind, label, amountCents: item.amountCents, dueDate: item.dueDate, daysLate },
      entityRefs: [{ type: item.kind, id: item.id }],
    });
  }
  return out;
}

function detectIncomeMissing(snapshot: FinanceSnapshot): FinanceObservation[] {
  const { today, current } = snapshot;
  const out: FinanceObservation[] = [];
  const graceLimit = addDays(today, -THRESHOLDS.incomeGraceDays);

  for (const income of snapshot.incomes) {
    if (income.received || !income.expectedDate) continue;
    if (income.expectedDate > graceLimit) continue;
    if (monthIndex(income) < monthIndex(addMonths(current, -1))) continue;
    const label = sanitizeLabel(income.name);
    out.push({
      detector: "income_missing",
      dedupeKey: `income_missing:${income.id}`,
      severity: income.incomeType === "main" ? "warning" : "info",
      materialityScore: materialityOf(income.amountCents),
      title: `${label} ainda não entrou`,
      summary: `${label} (${formatCurrency(income.amountCents)}) era esperada em ${income.expectedDate} e não está marcada como recebida.`,
      facts: { label, amountCents: income.amountCents, expectedDate: income.expectedDate, incomeType: income.incomeType },
      entityRefs: [{ type: "income", id: income.id }],
    });
  }

  // A NF do mês que ainda não foi lançada, quando o histórico diz que deveria.
  const day = Number(today.slice(8, 10));
  const hasMain = snapshot.incomes.some((row) => row.incomeType === "main" && sameMonth(row, current));
  const history = snapshot.incomes
    .filter((row) => row.incomeType === "main")
    .map((row) => ({ month: row.month, year: row.year, amountCents: row.amountCents }));
  const { averageCents, count } = averageOfRecent(history, current);
  if (!hasMain && count > 0 && day >= THRESHOLDS.nfMissingFromDay) {
    out.push({
      detector: "income_missing",
      dedupeKey: `income_missing:nf:${monthKey(current)}`,
      severity: "info",
      materialityScore: materialityOf(averageCents),
      title: `A NF de ${monthKey(current)} ainda não foi lançada`,
      summary: `Sem receita principal lançada neste mês; a projeção usa a média de ${formatCurrency(averageCents)}.`,
      facts: { month: monthKey(current), estimatedCents: averageCents, basis: count },
      entityRefs: [],
    });
  }
  return out;
}

function detectCardSpike(snapshot: FinanceSnapshot, factor: number): FinanceObservation[] {
  const { current } = snapshot;
  const out: FinanceObservation[] = [];
  for (const card of snapshot.cards) {
    const invoices = snapshot.invoices.filter((row) => row.cardId === card.id);
    const now = invoices.find((row) => sameMonth(row, current));
    if (!now) continue;
    const { averageCents, count } = averageOfRecent(
      invoices.map((row) => ({ month: row.month, year: row.year, amountCents: row.amountCents })),
      current,
    );
    if (count < 2 || averageCents === 0) continue;
    const ratio = now.amountCents / averageCents;
    const deltaCents = now.amountCents - averageCents;
    if (ratio < THRESHOLDS.spikeRatio || deltaCents < THRESHOLDS.spikeMinDeltaCents * factor) continue;
    const name = sanitizeLabel(card.name);
    out.push({
      detector: "card_spending_spike",
      dedupeKey: `card_spending_spike:${card.id}:${monthKey(current)}`,
      severity: ratio >= THRESHOLDS.spikeCriticalRatio ? "critical" : "warning",
      materialityScore: materialityOf(deltaCents),
      title: `Fatura ${name} ${Math.round((ratio - 1) * 100)}% acima da média`,
      summary: `${formatCurrency(now.amountCents)} contra média de ${formatCurrency(averageCents)} nas últimas ${count} faturas (+${formatCurrency(deltaCents)}).`,
      facts: { cardName: name, month: monthKey(current), amountCents: now.amountCents, averageCents, basis: count, deltaCents },
      entityRefs: [{ type: "card", id: card.id }, { type: "invoice", id: now.id }],
    });
  }
  return out;
}

function detectFuturePressure(snapshot: FinanceSnapshot): FinanceObservation[] {
  const buffer = snapshot.policies.minimumMonthEndBufferCents;
  const rows = futureCommitments(snapshot, THRESHOLDS.futureMonths + 1).months.slice(1);
  return rows
    .filter((row) => row.hasIncome && row.reliableRemainingCents - buffer < 0)
    .sort((a, b) => a.reliableRemainingCents - b.reliableRemainingCents)
    .slice(0, 2)
    .map((row) => {
      const gap = buffer - row.reliableRemainingCents;
      return {
        detector: "future_month_pressure" as const,
        dedupeKey: `future_month_pressure:${row.key}`,
        severity: row.reliableRemainingCents < 0 ? ("critical" as const) : ("warning" as const),
        materialityScore: materialityOf(gap),
        title: `${row.key} fica apertado`,
        summary: `Renda confiável ${formatCurrency(row.reliableIncomeCents)} contra ${formatCurrency(row.committedCents)} comprometidos${row.isEstimated ? " (com valores estimados)" : ""}; faltam ${formatCurrency(gap)} para a margem.`,
        facts: {
          month: row.key,
          reliableIncomeCents: row.reliableIncomeCents,
          commitmentsCents: row.committedCents,
          installmentsCents: row.installmentsCents,
          reliableRemainingCents: row.reliableRemainingCents,
          bufferCents: buffer,
          isEstimated: row.isEstimated,
        },
        entityRefs: [],
      };
    });
}

function detectInstallmentPressure(snapshot: FinanceSnapshot, factor: number): FinanceObservation[] {
  const rows = futureCommitments(snapshot, THRESHOLDS.futureMonths).months;
  const peak = rows.reduce((top, row) => (row.installmentsCents > top.installmentsCents ? row : top), rows[0]);
  if (!peak || peak.installmentsCents === 0) return [];
  const cap = snapshot.policies.maxInstallmentCommitmentCents;
  const share = peak.reliableIncomeCents > 0 ? peak.installmentsCents / peak.reliableIncomeCents : null;
  const overCap = cap !== null && peak.installmentsCents > cap;
  const heavy = share !== null && share >= THRESHOLDS.installmentShareOfIncome * factor;
  if (!overCap && !heavy) return [];

  const relief = rows.find(
    (row) => monthIndex(row) > monthIndex(peak) && row.installmentsCents <= peak.installmentsCents * 0.7,
  );
  return [
    {
      detector: "installment_pressure",
      dedupeKey: `installment_pressure:${peak.key}`,
      severity: overCap ? "warning" : "info",
      materialityScore: materialityOf(peak.installmentsCents),
      title: overCap ? `Parcelas acima do teto em ${peak.key}` : `Parcelas pesam em ${peak.key}`,
      summary:
        `${formatCurrency(peak.installmentsCents)} em parcelas` +
        (share !== null ? ` (${Math.round(share * 100)}% da renda confiável)` : "") +
        (overCap && cap !== null ? `, acima do teto de ${formatCurrency(cap)}` : "") +
        (relief ? `. Recua em ${relief.key}.` : "."),
      facts: {
        month: peak.key,
        installmentsCents: peak.installmentsCents,
        capCents: cap,
        shareOfReliableIncome: share,
        reliefMonth: relief?.key ?? null,
      },
      entityRefs: [],
    },
  ];
}

function detectSubscriptionLoad(snapshot: FinanceSnapshot, factor: number): FinanceObservation[] {
  const out: FinanceObservation[] = [];
  const summary = subscriptionSummary(snapshot);
  for (const item of summary.items) {
    if (item.status !== "trial") continue;
    if (item.daysUntilCharge < 0 || item.daysUntilCharge > THRESHOLDS.trialWarningDays) continue;
    out.push({
      detector: "subscription_load",
      dedupeKey: `subscription_load:trial:${item.id}:${item.nextChargeDate}`,
      severity: "warning",
      materialityScore: materialityOf(item.amountCents),
      title: `Teste de ${item.name} vira cobrança em ${item.daysUntilCharge} dia(s)`,
      summary: `Em ${item.nextChargeDate} começa a cobrar ${formatCurrency(item.amountCents)}. Cancele antes, se não for ficar.`,
      facts: { name: item.name, amountCents: item.amountCents, chargeDate: item.nextChargeDate },
      entityRefs: [{ type: "subscription", id: item.id }],
    });
  }

  const reliable = monthOverview(snapshot, snapshot.current).income.reliableCents;
  if (reliable > 0 && summary.activeMonthlyCents >= reliable * THRESHOLDS.subscriptionShareOfIncome * factor) {
    const share = summary.activeMonthlyCents / reliable;
    out.push({
      detector: "subscription_load",
      dedupeKey: `subscription_load:total:${monthKey(snapshot.current)}`,
      severity: "info",
      materialityScore: materialityOf(summary.activeMonthlyCents),
      title: `Assinaturas somam ${formatCurrency(summary.activeMonthlyCents)} por mês`,
      summary: `${summary.activeCount} assinatura(s) ativas, ${Math.round(share * 100)}% da renda confiável do mês.`,
      facts: {
        activeMonthlyCents: summary.activeMonthlyCents,
        activeCount: summary.activeCount,
        shareOfReliableIncome: share,
        top: summary.items.slice(0, 3).map((item) => ({ name: item.name, monthlyCents: item.monthlyEquivalentCents })),
      },
      entityRefs: [],
    });
  }
  return out;
}

function detectSafeToSpendDrop(
  snapshot: FinanceSnapshot,
  previous: number | null,
  factor: number,
): FinanceObservation[] {
  if (previous === null) return [];
  const now = safeToSpend(snapshot);
  const drop = previous - now.safeToSpendCents;
  const threshold = Math.max(THRESHOLDS.s2sDropMinCents * factor, Math.abs(previous) * THRESHOLDS.s2sDropShare);
  if (drop < threshold) return [];
  return [
    {
      detector: "safe_to_spend_drop",
      dedupeKey: `safe_to_spend_drop:${monthKey(snapshot.current)}`,
      severity: now.safeToSpendCents < 0 && previous >= 0 ? "critical" : "warning",
      materialityScore: materialityOf(drop),
      title: `Safe-to-Spend caiu ${formatCurrency(drop)}`,
      summary: `De ${formatCurrency(previous)} para ${formatCurrency(now.safeToSpendCents)} desde a última verificação.`,
      facts: {
        previousCents: previous,
        currentCents: now.safeToSpendCents,
        dropCents: drop,
        deductions: now.deductions.map(({ reason, amountCents }) => ({ reason, amountCents })),
      },
      entityRefs: [],
    },
  ];
}

/**
 * Meta com prazo que não fecha: o ritmo mensal que ela pede é maior do que a
 * folga confiável dos meses até o prazo (o pior deles, até seis à frente), ou o
 * prazo já passou sem ela concluir. Meta sem prazo não entra — sem prazo não
 * há "atrasada", só "devagar".
 */
function detectGoalAtRisk(snapshot: FinanceSnapshot, factor: number): FinanceObservation[] {
  const buffer = snapshot.policies.minimumMonthEndBufferCents;
  const rows = futureCommitments(snapshot, THRESHOLDS.futureMonths).months;
  const out: FinanceObservation[] = [];

  for (const goal of goalSummary(snapshot).goals) {
    if (goal.status !== "active" || goal.deadline === null || goal.remainingCents === 0) continue;
    const name = goal.name;
    const base = {
      detector: "goal_at_risk" as const,
      dedupeKey: `goal_at_risk:${goal.id}:${monthKey(snapshot.current)}`,
      entityRefs: [{ type: "goal", id: goal.id }],
    };

    if (goal.deadlinePassed) {
      out.push({
        ...base,
        severity: "warning",
        materialityScore: materialityOf(goal.remainingCents),
        title: `O prazo de "${name}" passou`,
        summary: `Faltam ${formatCurrency(goal.remainingCents)} e o prazo era ${goal.deadline}. Vale um prazo novo ou pausar a meta.`,
        facts: { goal: name, remainingCents: goal.remainingCents, deadline: goal.deadline, deadlinePassed: true },
      });
      continue;
    }

    const required = goal.requiredMonthlyCents ?? 0;
    if (required < THRESHOLDS.goalMinMonthlyCents * factor) continue;
    const window = rows.slice(0, Math.max(1, Math.min(goal.monthsLeft ?? 1, rows.length))).filter((row) => row.hasIncome);
    if (window.length === 0) continue;
    const tightest = window.reduce((low, row) => (row.reliableRemainingCents < low.reliableRemainingCents ? row : low));
    const room = tightest.reliableRemainingCents - buffer;
    if (required <= room) continue;

    out.push({
      ...base,
      severity: room <= 0 ? "critical" : "warning",
      materialityScore: materialityOf(required - Math.max(room, 0)),
      title: `"${name}" não fecha no prazo`,
      summary: `Pede ${formatCurrency(required)}/mês até ${goal.deadline}, mas ${tightest.key} tem só ${formatCurrency(Math.max(room, 0))} de folga confiável.`,
      facts: {
        goal: name,
        requiredMonthlyCents: required,
        deadline: goal.deadline,
        tightestMonth: tightest.key,
        roomCents: room,
      },
    });
  }
  return out;
}

/** Todos os detectores, na ordem em que a Home deveria ler: o urgente primeiro. */
export function runDetectors({ snapshot, previousSafeToSpendCents }: ObserverInput): FinanceObservation[] {
  const factor = sensitivityFactor(snapshot.policies.observerSensitivity);
  const all = [
    ...detectOverdue(snapshot, factor),
    ...detectDueSoon(snapshot, factor),
    ...detectIncomeMissing(snapshot),
    ...detectFuturePressure(snapshot),
    ...detectCardSpike(snapshot, factor),
    ...detectSafeToSpendDrop(snapshot, previousSafeToSpendCents, factor),
    ...detectInstallmentPressure(snapshot, factor),
    ...detectSubscriptionLoad(snapshot, factor),
    ...detectGoalAtRisk(snapshot, factor),
  ];
  return all.sort(
    (a, b) => severityRank(b.severity) - severityRank(a.severity) || b.materialityScore - a.materialityScore,
  );
}
