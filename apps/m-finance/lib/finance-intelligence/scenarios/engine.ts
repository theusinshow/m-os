import type { MonthHealth } from "@/db/schema";
import { forecastMainIncome } from "@/lib/calculations/forecast";
import { classifyMonthHealth } from "@/lib/calculations/month-health";
import { installmentImpactCents } from "@/lib/calculations/simulator";
import {
  addMonths,
  monthIndex,
  monthKey,
  parseMonthKey,
  sameMonth,
} from "@/lib/finance-intelligence/dates";
import { goalSummary } from "@/lib/finance-intelligence/kernel/goals";
import { monthOverview, reliableShare } from "@/lib/finance-intelligence/kernel/overview";
import { monthlyEquivalentCents } from "@/lib/finance-intelligence/kernel/subscriptions";
import type { ResolvedPolicies } from "@/lib/finance-intelligence/policies";
import { normalizeForMatch, sanitizeLabel } from "@/lib/finance-intelligence/sanitize";
import type { ScenarioChange } from "@/lib/finance-intelligence/scenarios/types";
import { formatCurrency } from "@/lib/formatters/currency";
import type { FinanceSnapshot, IncomeType, MonthParts } from "@/lib/finance-intelligence/types";

type IncomePart = { incomeType: IncomeType; amountCents: number; received: boolean };

type WorkingMonth = MonthParts & {
  key: string;
  incomes: IncomePart[];
  commitmentsCents: number;
  installmentsCents: number;
};

export type ScenarioMonth = MonthParts & {
  key: string;
  incomeCents: number;
  reliableIncomeCents: number;
  commitmentsCents: number;
  installmentsCents: number;
  /** Renda confiável − compromissos − margem − metas protegidas, NESTE mês. */
  safeToSpendCents: number;
  /** Renda confiável − compromissos, acumulado desde o mês atual. */
  cumulativeCents: number;
  health: MonthHealth;
  hasIncome: boolean;
  base: { safeToSpendCents: number; commitmentsCents: number; reliableIncomeCents: number };
  deltaSafeToSpendCents: number;
};

export type ScenarioResult = {
  kind: "projection";
  horizonMonths: number;
  assumptions: ScenarioChange[];
  policy: {
    minimumMonthEndBufferCents: number;
    maxInstallmentCommitmentCents: number | null;
    reliableIncomeRules: ResolvedPolicies["reliableIncomeRules"];
    protectGoals: boolean;
  };
  months: ScenarioMonth[];
  worstMonth: { key: string; safeToSpendCents: number } | null;
  baseWorstMonth: { key: string; safeToSpendCents: number } | null;
  totalAddedCommitmentCents: number;
  firstNegativeCumulativeMonth: string | null;
  warnings: string[];
  method: string;
};

const METHOD =
  "Projeção determinística sobre o M-Finance: faturas e NF futuras estimadas pela média das 3 últimas; " +
  "Safe-to-Spend de cada mês = renda confiável − compromissos − margem mínima (− metas protegidas). " +
  "Nada foi gravado.";

function resolveMonth(value: string | undefined, fallback: MonthParts): MonthParts {
  return (value && parseMonthKey(value)) || fallback;
}

function inRange(target: MonthParts, from: MonthParts, until: MonthParts | null) {
  const index = monthIndex(target);
  return index >= monthIndex(from) && (until === null || index <= monthIndex(until));
}

/** As receitas de um mês, com o que é recebido e o que é estimado. */
function incomeParts(snapshot: FinanceSnapshot, target: MonthParts): IncomePart[] {
  const parts: IncomePart[] = snapshot.incomes
    .filter((row) => sameMonth(row, target))
    .map((row) => ({ incomeType: row.incomeType, amountCents: row.amountCents, received: row.received }));
  const estimated = forecastMainIncome(snapshot.incomes, target, snapshot.current);
  if (estimated.source === "estimated") {
    parts.push({ incomeType: "main", amountCents: estimated.amountCents, received: false });
  }
  return parts;
}

function baseMonths(snapshot: FinanceSnapshot, horizon: number, policies: ResolvedPolicies): WorkingMonth[] {
  return Array.from({ length: horizon }, (_, offset) => {
    const parts = addMonths(snapshot.current, offset);
    const overview = monthOverview(snapshot, parts, policies);
    return {
      ...parts,
      key: monthKey(parts),
      incomes: incomeParts(snapshot, parts),
      commitmentsCents: overview.committedCents,
      installmentsCents: overview.installmentsCents,
    };
  });
}

function scenarioPolicies(base: ResolvedPolicies, changes: ScenarioChange[]): ResolvedPolicies {
  const policies: ResolvedPolicies = {
    ...base,
    reliableIncomeRules: { ...base.reliableIncomeRules },
    safeToSpend: { ...base.safeToSpend },
  };
  for (const change of changes) {
    if (change.type !== "set_policy_temporary") continue;
    if (change.minimumMonthEndBufferCents !== undefined) {
      policies.minimumMonthEndBufferCents = change.minimumMonthEndBufferCents;
    }
    if (change.maxInstallmentCommitmentCents !== undefined) {
      policies.maxInstallmentCommitmentCents = change.maxInstallmentCommitmentCents;
    }
    if (change.reliableIncomeRules) policies.reliableIncomeRules = { ...change.reliableIncomeRules };
  }
  return policies;
}

/**
 * Aplica uma mudança aos meses de trabalho. Devolve os avisos que ela produziu
 * — um nome de conta que não bate com nada é um aviso, não um erro: o resto do
 * cenário continua valendo e a pessoa precisa saber o que foi ignorado.
 */
function applyChange(
  snapshot: FinanceSnapshot,
  months: WorkingMonth[],
  change: ScenarioChange,
): string[] {
  const current = snapshot.current;
  const warnings: string[] = [];
  const at = (parts: MonthParts) => months.find((row) => sameMonth(row, parts));

  switch (change.type) {
    case "one_time_expense": {
      const month = at(resolveMonth(change.month, current));
      if (month) month.commitmentsCents += change.amountCents;
      else warnings.push(`"${sanitizeLabel(change.label)}" cai fora do horizonte simulado.`);
      break;
    }
    case "installment_purchase": {
      const start = resolveMonth(change.startMonth, current);
      const down = change.downPaymentCents ?? 0;
      const financed = change.totalCents - down;
      const { firstCents, restCents } = installmentImpactCents(financed, change.installments);
      const startMonth = at(start);
      if (startMonth && down > 0) startMonth.commitmentsCents += down;
      let outside = 0;
      for (let index = 0; index < change.installments; index += 1) {
        const month = at(addMonths(start, index));
        const amount = index === 0 ? firstCents : restCents;
        if (!month) {
          outside += 1;
          continue;
        }
        month.commitmentsCents += amount;
        if (change.installments > 1) month.installmentsCents += amount;
      }
      if (outside > 0) {
        warnings.push(`${outside} parcela(s) de "${sanitizeLabel(change.label)}" ficam depois do horizonte.`);
      }
      break;
    }
    case "recurring_expense": {
      const from = resolveMonth(change.fromMonth, current);
      const until = change.untilMonth ? parseMonthKey(change.untilMonth) : null;
      for (const month of months) {
        if (inRange(month, from, until)) month.commitmentsCents += change.amountCents;
      }
      break;
    }
    case "remove_expense": {
      const from = resolveMonth(change.fromMonth, current);
      const target = change.target;
      if (target.kind === "subscription") {
        const subscription = snapshot.subscriptions.find((row) => row.id === target.id);
        if (!subscription) {
          warnings.push(`Assinatura ${target.id} não encontrada; nada removido.`);
          break;
        }
        const monthly = monthlyEquivalentCents(subscription);
        if (subscription.cycle === "yearly") {
          warnings.push(
            `"${sanitizeLabel(subscription.name)}" é anual; removida pelo equivalente mensal (${formatCurrency(monthly)}).`,
          );
        }
        for (const month of months) {
          if (inRange(month, from, null)) month.commitmentsCents = Math.max(0, month.commitmentsCents - monthly);
        }
      } else if (target.kind === "bill") {
        const wanted = normalizeForMatch(target.name);
        let found = false;
        for (const month of months) {
          if (!inRange(month, from, null)) continue;
          const matching = snapshot.bills.filter(
            (bill) => sameMonth(bill, month) && normalizeForMatch(bill.name).includes(wanted),
          );
          for (const bill of matching) {
            found = true;
            month.commitmentsCents = Math.max(0, month.commitmentsCents - bill.amountCents);
            if (bill.seriesId) month.installmentsCents = Math.max(0, month.installmentsCents - bill.amountCents);
          }
        }
        if (!found) warnings.push(`Nenhuma conta "${sanitizeLabel(target.name)}" no horizonte; nada removido.`);
      } else {
        for (const month of months) {
          if (inRange(month, from, null)) {
            month.commitmentsCents = Math.max(0, month.commitmentsCents - target.amountCents);
          }
        }
      }
      break;
    }
    case "change_income": {
      const from = resolveMonth(change.fromMonth, current);
      const until = change.untilMonth ? parseMonthKey(change.untilMonth) : null;
      for (const month of months) {
        if (!inRange(month, from, until)) continue;
        if (change.percent !== undefined) {
          for (const part of month.incomes) {
            if (change.incomeType === "all" || part.incomeType === change.incomeType) {
              part.amountCents = Math.max(0, Math.round(part.amountCents * (1 + change.percent / 100)));
            }
          }
        } else if (change.deltaCents !== undefined && change.incomeType !== "all") {
          const total = month.incomes
            .filter((part) => part.incomeType === change.incomeType)
            .reduce((sum, part) => sum + part.amountCents, 0);
          const next = Math.max(0, total + change.deltaCents);
          // Reescreve o tipo inteiro numa parte só, não recebida: é futuro.
          month.incomes = month.incomes.filter((part) => part.incomeType !== change.incomeType);
          if (next > 0) month.incomes.push({ incomeType: change.incomeType, amountCents: next, received: false });
        }
      }
      break;
    }
    case "remove_income": {
      const from = resolveMonth(change.fromMonth, current);
      for (const month of months) {
        if (!inRange(month, from, null)) continue;
        month.incomes = month.incomes.filter(
          (part) => change.incomeType !== "all" && part.incomeType !== change.incomeType,
        );
      }
      break;
    }
    case "add_income": {
      const from = resolveMonth(change.fromMonth, current);
      const until = change.recurring
        ? change.untilMonth
          ? parseMonthKey(change.untilMonth)
          : null
        : from;
      for (const month of months) {
        if (inRange(month, from, until)) {
          month.incomes.push({ incomeType: change.incomeType, amountCents: change.amountCents, received: false });
        }
      }
      break;
    }
    case "pay_off_installments": {
      const payMonth = resolveMonth(change.month, current);
      const future = snapshot.cardExpenses.filter(
        (row) => row.installmentId === change.installmentId && monthIndex(row) > monthIndex(payMonth),
      );
      if (future.length === 0) {
        warnings.push("Parcelamento não encontrado ou sem parcelas futuras; nada antecipado.");
        break;
      }
      let total = 0;
      for (const row of future) {
        total += row.amountCents;
        const month = at(row);
        if (month) {
          month.commitmentsCents = Math.max(0, month.commitmentsCents - row.amountCents);
          month.installmentsCents = Math.max(0, month.installmentsCents - row.amountCents);
        }
      }
      const target = at(payMonth);
      if (target) target.commitmentsCents += total;
      warnings.push(
        "Antecipação sem desconto: o valor das parcelas futuras entra inteiro no mês escolhido.",
      );
      break;
    }
    case "set_policy_temporary":
      break;
  }

  return warnings;
}

function reliableOf(incomes: IncomePart[], rules: ResolvedPolicies["reliableIncomeRules"]) {
  return incomes.reduce(
    (sum, part) => sum + reliableShare(part.amountCents, part.incomeType, part.received, rules),
    0,
  );
}

function evaluate(
  months: WorkingMonth[],
  policies: ResolvedPolicies,
  goalReserveCents: number,
) {
  let cumulative = 0;
  return months.map((month) => {
    const incomeCents = month.incomes.reduce((sum, part) => sum + part.amountCents, 0);
    const reliableIncomeCents = reliableOf(month.incomes, policies.reliableIncomeRules);
    cumulative += reliableIncomeCents - month.commitmentsCents;
    return {
      month: month.month,
      year: month.year,
      key: month.key,
      incomeCents,
      reliableIncomeCents,
      commitmentsCents: month.commitmentsCents,
      installmentsCents: month.installmentsCents,
      safeToSpendCents:
        reliableIncomeCents - month.commitmentsCents - policies.minimumMonthEndBufferCents - goalReserveCents,
      cumulativeCents: cumulative,
      health: classifyMonthHealth({
        estimatedRemainingCents: incomeCents - month.commitmentsCents,
        overdueCents: 0,
        dueSoonCount: 0,
      }),
      hasIncome: incomeCents > 0,
    };
  });
}

function worst(rows: { key: string; safeToSpendCents: number; hasIncome: boolean }[]) {
  return rows
    .filter((row) => row.hasIncome)
    .reduce<{ key: string; safeToSpendCents: number } | null>(
      (lowest, row) =>
        lowest === null || row.safeToSpendCents < lowest.safeToSpendCents
          ? { key: row.key, safeToSpendCents: row.safeToSpendCents }
          : lowest,
      null,
    );
}

/**
 * Simula mudanças temporárias sobre o estado real, mês a mês.
 *
 * A base e o cenário passam pela MESMA conta (`evaluate`), com as mesmas
 * funções do kernel por baixo — a diferença entre as duas colunas é só o que o
 * cenário mudou. O resultado é projeção e se declara projeção.
 */
export function simulateScenario(
  snapshot: FinanceSnapshot,
  changes: ScenarioChange[],
  horizonMonths: number = snapshot.policies.forecastHorizonMonths,
): ScenarioResult {
  const horizon = Math.min(Math.max(horizonMonths, 1), 24);
  const basePolicies = snapshot.policies;
  const policies = scenarioPolicies(basePolicies, changes);

  const goalReserve = (active: ResolvedPolicies) =>
    active.safeToSpend.protectGoals ? goalSummary(snapshot).totalRequiredMonthlyCents : 0;

  const baseRows = evaluate(baseMonths(snapshot, horizon, basePolicies), basePolicies, goalReserve(basePolicies));
  const working = baseMonths(snapshot, horizon, policies);
  const warnings: string[] = [];
  for (const change of changes) warnings.push(...applyChange(snapshot, working, change));
  const rows = evaluate(working, policies, goalReserve(policies));

  const months: ScenarioMonth[] = rows.map((row, index) => ({
    ...row,
    base: {
      safeToSpendCents: baseRows[index].safeToSpendCents,
      commitmentsCents: baseRows[index].commitmentsCents,
      reliableIncomeCents: baseRows[index].reliableIncomeCents,
    },
    deltaSafeToSpendCents: row.safeToSpendCents - baseRows[index].safeToSpendCents,
  }));

  for (const row of months) {
    if (!row.hasIncome) {
      warnings.push(`${row.key}: sem receita conhecida — o mês aparece sem renda.`);
    } else if (row.safeToSpendCents < 0 && row.base.safeToSpendCents >= 0) {
      warnings.push(`${row.key}: o Safe-to-Spend do mês fica negativo (${formatCurrency(row.safeToSpendCents)}).`);
    }
    const cap = policies.maxInstallmentCommitmentCents;
    if (cap !== null && row.installmentsCents > cap) {
      warnings.push(
        `${row.key}: parcelas somam ${formatCurrency(row.installmentsCents)}, acima do teto de ${formatCurrency(cap)}.`,
      );
    }
  }

  const firstNegative = months.find((row) => row.cumulativeCents < 0);
  if (firstNegative) {
    warnings.push(`O saldo acumulado fica negativo a partir de ${firstNegative.key}.`);
  }

  return {
    kind: "projection",
    horizonMonths: horizon,
    assumptions: changes,
    policy: {
      minimumMonthEndBufferCents: policies.minimumMonthEndBufferCents,
      maxInstallmentCommitmentCents: policies.maxInstallmentCommitmentCents,
      reliableIncomeRules: { ...policies.reliableIncomeRules },
      protectGoals: policies.safeToSpend.protectGoals,
    },
    months,
    worstMonth: worst(months),
    baseWorstMonth: worst(baseRows),
    totalAddedCommitmentCents: months.reduce(
      (sum, row) => sum + row.commitmentsCents - row.base.commitmentsCents,
      0,
    ),
    firstNegativeCumulativeMonth: firstNegative?.key ?? null,
    warnings: [...new Set(warnings)],
    method: METHOD,
  };
}

/** Várias alternativas contra a mesma base — "à vista × 6x × esperar". */
export function compareScenarios(
  snapshot: FinanceSnapshot,
  variants: { label: string; changes: ScenarioChange[] }[],
  horizonMonths?: number,
) {
  const results = variants.map((variant) => ({
    label: sanitizeLabel(variant.label),
    result: simulateScenario(snapshot, variant.changes, horizonMonths),
  }));
  const ranked = [...results]
    .filter((entry) => entry.result.worstMonth)
    .sort((a, b) => (b.result.worstMonth?.safeToSpendCents ?? 0) - (a.result.worstMonth?.safeToSpendCents ?? 0));

  return {
    kind: "projection" as const,
    variants: results.map((entry) => ({
      label: entry.label,
      worstMonth: entry.result.worstMonth,
      totalAddedCommitmentCents: entry.result.totalAddedCommitmentCents,
      firstNegativeCumulativeMonth: entry.result.firstNegativeCumulativeMonth,
      warnings: entry.result.warnings,
      months: entry.result.months.map((row) => ({
        key: row.key,
        commitmentsCents: row.commitmentsCents,
        safeToSpendCents: row.safeToSpendCents,
        deltaSafeToSpendCents: row.deltaSafeToSpendCents,
        cumulativeCents: row.cumulativeCents,
      })),
    })),
    /** Pelo pior mês, do mais folgado ao mais apertado. Critério, não recomendação. */
    rankingByWorstMonth: ranked.map((entry) => entry.label),
    method: METHOD,
  };
}
