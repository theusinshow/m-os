import type { MonthHealth, PaymentType, RiskLevel } from "@/db/schema";
import { classifyMonthHealth } from "@/lib/calculations/month-health";

export type SimulationMonth = {
  month: number;
  year: number;
  baselineRemainingCents: number;
  impactCents: number;
  remainingWithCents: number;
  health: MonthHealth;
};

export type SimulationResult = {
  monthlyImpactCents: number;
  affectedMonths: number;
  riskLevel: RiskLevel;
  recommendation: string;
  months: SimulationMonth[];
};

const monthLabels = [
  "janeiro",
  "fevereiro",
  "março",
  "abril",
  "maio",
  "junho",
  "julho",
  "agosto",
  "setembro",
  "outubro",
  "novembro",
  "dezembro",
];

export function monthName(month: number) {
  return monthLabels[month - 1] ?? "";
}

// Month health maps directly onto purchase risk — we reuse the exact thresholds
// the dashboard already applies, so a "tight" month means the same thing here.
const HEALTH_TO_RISK: Record<MonthHealth, RiskLevel> = {
  positive: "safe",
  fair: "controlled",
  tight: "tight",
  negative: "critical",
};

const RISK_ORDER: RiskLevel[] = ["safe", "controlled", "tight", "critical"];

function worstRisk(levels: RiskLevel[]): RiskLevel {
  return levels.reduce(
    (worst, level) => (RISK_ORDER.indexOf(level) > RISK_ORDER.indexOf(worst) ? level : worst),
    "safe" as RiskLevel,
  );
}

function addMonths(month: number, year: number, offset: number) {
  const zeroBased = month - 1 + offset;
  return {
    month: (((zeroBased % 12) + 12) % 12) + 1,
    year: year + Math.floor(zeroBased / 12),
  };
}

/**
 * Splits a total across N installments so the parts sum back exactly to the
 * total (the remainder lands on the first installment), avoiding rounding drift.
 */
export function installmentImpactCents(totalCents: number, installments: number) {
  const base = Math.floor(totalCents / installments);
  const remainder = totalCents - base * installments;
  return { firstCents: base + remainder, restCents: base };
}

function buildRecommendation(
  riskLevel: RiskLevel,
  months: SimulationMonth[],
  paymentType: PaymentType,
  installments: number,
): string {
  const worstMonth = months.reduce((worst, current) =>
    current.remainingWithCents < worst.remainingWithCents ? current : worst,
  );
  const worstLabel = `${monthName(worstMonth.month)} de ${worstMonth.year}`;
  const canSplitMore = paymentType === "cash" || installments < 6;

  switch (riskLevel) {
    case "safe":
      return "Compra tranquila. Sua margem segue confortável em todos os meses afetados.";
    case "controlled":
      return `Compra possível, mas reduz a margem. Evite assumir outra parcela no mesmo período de ${worstLabel}.`;
    case "tight":
      return canSplitMore
        ? `Compra arriscada: ${worstLabel} fica apertado. Diluir em mais parcelas reduz o impacto mensal.`
        : `Compra arriscada: ${worstLabel} fica apertado. Reveja outras despesas desse período antes de confirmar.`;
    case "critical":
      return `Compra não recomendada agora: ${worstLabel} fica no negativo. Adie ou reduza o valor da compra.`;
  }
}

/**
 * Projects the purchase impact over the affected months against a flat monthly
 * baseline (income − recurring bills − invoices from the current month). The
 * baseline is intentionally constant: future months don't exist yet, so we
 * assume recurrences repeat. The UI states this assumption.
 */
export function computeSimulation({
  totalAmountCents,
  paymentType,
  installments,
  startMonth,
  startYear,
  baselineRemainingCents,
}: {
  totalAmountCents: number;
  paymentType: PaymentType;
  installments: number;
  startMonth: number;
  startYear: number;
  baselineRemainingCents: number;
}): SimulationResult {
  const affectedMonths = paymentType === "installment" ? Math.max(installments, 1) : 1;

  const impacts: number[] = [];
  if (paymentType === "installment") {
    const { firstCents, restCents } = installmentImpactCents(totalAmountCents, affectedMonths);
    for (let index = 0; index < affectedMonths; index += 1) {
      impacts.push(index === 0 ? firstCents : restCents);
    }
  } else {
    impacts.push(totalAmountCents);
  }

  const months: SimulationMonth[] = impacts.map((impactCents, index) => {
    const { month, year } = addMonths(startMonth, startYear, index);
    const remainingWithCents = baselineRemainingCents - impactCents;
    return {
      month,
      year,
      baselineRemainingCents,
      impactCents,
      remainingWithCents,
      health: classifyMonthHealth({
        estimatedRemainingCents: remainingWithCents,
        overdueCents: 0,
        dueSoonCount: 0,
      }),
    };
  });

  const riskLevel = worstRisk(months.map((month) => HEALTH_TO_RISK[month.health]));
  const monthlyImpactCents = impacts[0] ?? 0;
  const recommendation = buildRecommendation(riskLevel, months, paymentType, affectedMonths);

  return { monthlyImpactCents, affectedMonths, riskLevel, recommendation, months };
}
