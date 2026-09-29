import { daysBetween } from "@/lib/finance-intelligence/dates";
import { sanitizeLabel } from "@/lib/finance-intelligence/sanitize";
import type { FinanceSnapshot, SnapshotSubscription } from "@/lib/finance-intelligence/types";

/** O custo mensal equivalente: anual dividido por 12; cobrança única não recorre. */
export function monthlyEquivalentCents(subscription: Pick<SnapshotSubscription, "amountCents" | "cycle">) {
  if (subscription.cycle === "monthly") return subscription.amountCents;
  if (subscription.cycle === "yearly") return Math.round(subscription.amountCents / 12);
  return 0;
}

/**
 * Assinaturas vivas e o que elas custam por mês.
 *
 * Teste grátis entra separado: ele ainda não custa nada, mas vai custar na data
 * de conversão — e é exatamente essa data que o usuário esquece.
 */
export function subscriptionSummary(snapshot: FinanceSnapshot) {
  const live = snapshot.subscriptions.filter((row) => row.status !== "canceled");
  const items = live
    .map((row) => ({
      id: row.id,
      name: sanitizeLabel(row.name),
      amountCents: row.amountCents,
      cycle: row.cycle,
      status: row.status as "trial" | "active",
      nextChargeDate: row.nextChargeDate,
      daysUntilCharge: daysBetween(snapshot.today, row.nextChargeDate),
      monthlyEquivalentCents: monthlyEquivalentCents(row),
    }))
    .sort((a, b) => b.monthlyEquivalentCents - a.monthlyEquivalentCents);

  const active = items.filter((item) => item.status === "active");
  const trials = items.filter((item) => item.status === "trial");

  return {
    activeMonthlyCents: active.reduce((sum, item) => sum + item.monthlyEquivalentCents, 0),
    /** Quanto passa a custar por mês se todo teste grátis converter. */
    trialsMonthlyCents: trials.reduce((sum, item) => sum + item.monthlyEquivalentCents, 0),
    activeCount: active.length,
    trialCount: trials.length,
    canceledCount: snapshot.subscriptions.length - live.length,
    items,
  };
}
