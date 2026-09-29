import { safeToSpend } from "@/lib/finance-intelligence/kernel/safe-to-spend";
import { getStoredInsights } from "@/lib/finance-intelligence/observer/run";
import { loadFinanceSnapshot } from "@/lib/finance-intelligence/snapshot/load";

/**
 * O que o dashboard mostra da camada de inteligência: o Safe-to-Spend com a
 * decomposição e os insights vivos.
 *
 * Falhar aqui não pode derrubar o dashboard — ele existia antes disto e
 * continua sendo o cockpit. Sem a migration 0016, por exemplo, os insights
 * somem e o resto da tela segue igual.
 */
export async function getDashboardIntelligence(userId: string) {
  try {
    const snapshot = await loadFinanceSnapshot(userId);
    const insights = await getStoredInsights(userId, 6).catch(() => []);
    return { safeToSpend: safeToSpend(snapshot), insights, policies: snapshot.policies };
  } catch {
    return null;
  }
}

export type DashboardIntelligence = NonNullable<Awaited<ReturnType<typeof getDashboardIntelligence>>>;
