import type { MonthHealth } from "@/db/schema";
import { classifyMonthHealth } from "@/lib/calculations/month-health";
import { addMonths, monthIndex, monthKey } from "@/lib/finance-intelligence/dates";
import { futureMonthRow } from "@/lib/finance-intelligence/kernel/commitments";
import { goalSummary } from "@/lib/finance-intelligence/kernel/goals";
import { monthOverview } from "@/lib/finance-intelligence/kernel/overview";
import type { ResolvedPolicies } from "@/lib/finance-intelligence/policies";
import { formatCurrency } from "@/lib/formatters/currency";
import type { FinanceSnapshot, MonthParts } from "@/lib/finance-intelligence/types";

export type SafeToSpendDeductionReason =
  | "income_reliability"
  | "minimum_buffer"
  | "goal_reserve"
  | "future_pressure";

export type SafeToSpendDeduction = {
  reason: SafeToSpendDeductionReason;
  label: string;
  amountCents: number;
  detail: string;
};

export type SafeToSpendResult = MonthParts & {
  key: string;
  safeToSpendCents: number;
  accountingRemainingCents: number;
  reliableRemainingCents: number;
  totalIncomeCents: number;
  reliableIncomeCents: number;
  committedCents: number;
  deductions: SafeToSpendDeduction[];
  futurePressure: { key: string; shortfallCents: number }[];
  assumptions: string[];
  status: "unknown" | MonthHealth;
  isEstimated: boolean;
  policy: {
    minimumMonthEndBufferCents: number;
    reliableIncomeRules: ResolvedPolicies["reliableIncomeRules"];
    lookaheadMonths: number;
    protectGoals: boolean;
  };
};

export type FuturePressureMonth = {
  key: string;
  reliableRemainingCents: number;
  hasIncome: boolean;
};

/**
 * Quanto do mês atual precisa ficar guardado para os meses à frente.
 *
 * Saldo corrido, e não soma de negativos: um novembro folgado cobre um
 * dezembro apertado, e contar o buraco de dezembro sem o alívio de novembro
 * seria reservar dinheiro que o próprio calendário já resolve. A reserva é o
 * ponto mais baixo desse saldo abaixo de zero.
 *
 * Mês sem renda conhecida fica de fora: sem NF lançada nem média para estimar,
 * a "sobra" dele é −(tudo), e esse número derrubaria o Safe-to-Spend inteiro
 * por falta de dado — não por falta de dinheiro.
 */
export function futurePressureReserve(months: FuturePressureMonth[], bufferCents: number) {
  let running = 0;
  let lowest = 0;
  const shortfalls: { key: string; shortfallCents: number }[] = [];

  for (const row of months) {
    if (!row.hasIncome) continue;
    const net = row.reliableRemainingCents - bufferCents;
    running += net;
    if (net < 0) shortfalls.push({ key: row.key, shortfallCents: -net });
    lowest = Math.min(lowest, running);
  }

  return { reserveCents: lowest < 0 ? -lowest : 0, shortfalls };
}

/**
 * Safe-to-Spend v1: quanto dá para gastar de forma discricionária AGORA sem
 * violar as políticas configuradas nem os compromissos já conhecidos.
 *
 * ```
 * sobra contábil       receita total − comprometido        (= dashboard)
 * − confiabilidade     receita total − receita confiável
 * − margem mínima      policy minimum_month_end_buffer
 * − metas protegidas   ritmo mensal das metas, se protectGoals
 * − pressão futura     falta corrida dos próximos N meses
 * = Safe-to-Spend
 * ```
 *
 * Nunca um número sozinho: as deduções e as premissas saem junto, e a
 * identidade "sobra − Σ deduções = Safe-to-Spend" é o que os testes cobram.
 */
export function safeToSpend(
  snapshot: FinanceSnapshot,
  target: MonthParts = snapshot.current,
  policies: ResolvedPolicies = snapshot.policies,
): SafeToSpendResult {
  const overview = monthOverview(snapshot, target, policies);
  const assumptions: string[] = [];
  const deductions: SafeToSpendDeduction[] = [];

  const reliabilityGap = overview.income.totalCents - overview.income.reliableCents;
  if (reliabilityGap > 0) {
    const rules = policies.reliableIncomeRules;
    deductions.push({
      reason: "income_reliability",
      label: "Receita ainda não garantida",
      amountCents: reliabilityGap,
      detail: `Receita não recebida conta pelo peso do tipo: principal ${pct(rules.main)}, freelance ${pct(rules.freelance)}, extra ${pct(rules.extra)}.`,
    });
  }
  if (overview.income.estimatedCents > 0) {
    assumptions.push(
      `A NF do mês ainda não foi lançada; entra estimada em ${formatCurrency(overview.income.estimatedCents)} (média das últimas).`,
    );
  }
  if (overview.invoices.estimatedCents > 0) {
    assumptions.push(
      `Há ${formatCurrency(overview.invoices.estimatedCents)} de fatura estimada — o valor real pode ser diferente.`,
    );
  }

  const buffer = policies.minimumMonthEndBufferCents;
  if (buffer > 0) {
    deductions.push({
      reason: "minimum_buffer",
      label: "Margem mínima",
      amountCents: buffer,
      detail: `Política: sobrar pelo menos ${formatCurrency(buffer)} ao fechar o mês.`,
    });
  } else {
    assumptions.push("Nenhuma margem mínima configurada — o Safe-to-Spend pode ir até zero.");
  }

  if (policies.safeToSpend.protectGoals) {
    const goals = goalSummary(snapshot).goals.filter(
      (goal) => goal.status === "active" && goal.requiredMonthlyCents !== null,
    );
    const reserve = goals.reduce((sum, goal) => sum + (goal.requiredMonthlyCents ?? 0), 0);
    if (reserve > 0) {
      deductions.push({
        reason: "goal_reserve",
        label: "Metas protegidas",
        amountCents: reserve,
        detail: `Ritmo mensal para cumprir ${goals.length} meta(s) no prazo.`,
      });
    }
  }

  const lookahead = policies.safeToSpend.lookaheadMonths;
  const futureRows = Array.from({ length: lookahead }, (_, offset) =>
    futureMonthRow(snapshot, addMonths(target, offset + 1), policies),
  );
  const pressure = futurePressureReserve(futureRows, buffer);
  if (pressure.reserveCents > 0) {
    deductions.push({
      reason: "future_pressure",
      label: "Pressão dos próximos meses",
      amountCents: pressure.reserveCents,
      detail: pressure.shortfalls
        .map((row) => `${row.key} fica ${formatCurrency(row.shortfallCents)} abaixo da margem`)
        .join("; "),
    });
  }
  const blind = futureRows.filter((row) => !row.hasIncome).map((row) => row.key);
  if (blind.length > 0) {
    assumptions.push(`Sem receita conhecida em ${blind.join(", ")}: a pressão desses meses não foi calculada.`);
  }
  if (lookahead === 0) {
    assumptions.push("O Safe-to-Spend não olha para os meses seguintes (lookahead 0).");
  }

  const totalDeductions = deductions.reduce((sum, item) => sum + item.amountCents, 0);
  const safe = overview.accountingRemainingCents - totalDeductions;

  if (monthIndex(target) !== monthIndex(snapshot.current)) {
    assumptions.push(`Calculado para ${monthKey(target)}, não para o mês corrente.`);
  }

  return {
    month: target.month,
    year: target.year,
    key: monthKey(target),
    safeToSpendCents: safe,
    accountingRemainingCents: overview.accountingRemainingCents,
    reliableRemainingCents: overview.reliableRemainingCents,
    totalIncomeCents: overview.income.totalCents,
    reliableIncomeCents: overview.income.reliableCents,
    committedCents: overview.committedCents,
    deductions,
    futurePressure: pressure.shortfalls,
    assumptions,
    status: !overview.hasIncome
      ? "unknown"
      : classifyMonthHealth({
          estimatedRemainingCents: safe,
          overdueCents: overview.overdueCents,
          dueSoonCount: 0,
        }),
    isEstimated: overview.isEstimated,
    policy: {
      minimumMonthEndBufferCents: buffer,
      reliableIncomeRules: { ...policies.reliableIncomeRules },
      lookaheadMonths: lookahead,
      protectGoals: policies.safeToSpend.protectGoals,
    },
  };
}

function pct(value: number) {
  return `${Math.round(value * 100)}%`;
}
