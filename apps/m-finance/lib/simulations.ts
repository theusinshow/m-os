import { desc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { purchaseSimulations } from "@/db/schema";
import { getBillsByMonth } from "@/lib/bills";
import { getInvoicesByMonth } from "@/lib/cards";
import { getIncomesByMonth } from "@/lib/incomes";
import type { SimulationResult } from "@/lib/calculations/simulator";

export type StoredSimulation = {
  id: string;
  name: string;
  totalAmountCents: number;
  paymentType: "cash" | "installment";
  installments: number | null;
  startMonth: number;
  startYear: number;
  monthlyImpactCents: number;
  riskLevel: "safe" | "controlled" | "tight" | "critical";
  recommendation: string;
  result: SimulationResult;
  createdAt: Date;
};

export async function getSimulations(userId: string): Promise<StoredSimulation[]> {
  if (!db) {
    return [];
  }

  const rows = await db
    .select()
    .from(purchaseSimulations)
    .where(eq(purchaseSimulations.userId, userId))
    .orderBy(desc(purchaseSimulations.createdAt));

  return rows.map((row) => ({
    id: row.id,
    name: row.name,
    totalAmountCents: row.totalAmountCents,
    paymentType: row.paymentType,
    installments: row.installments,
    startMonth: row.startMonth,
    startYear: row.startYear,
    monthlyImpactCents: row.monthlyImpactCents,
    riskLevel: row.riskLevel,
    recommendation: row.recommendation,
    result: row.resultPayload as SimulationResult,
    createdAt: row.createdAt,
  }));
}

/**
 * Flat monthly baseline used to project the purchase impact: income minus the
 * recurring obligations of the current month (recurring bills + invoices).
 * Returns null when there's no current month to base the projection on.
 */
export async function getSimulationBaseline(userId: string, currentMonthId: string) {
  const [bills, invoices, incomes] = await Promise.all([
    getBillsByMonth(currentMonthId),
    getInvoicesByMonth(currentMonthId),
    getIncomesByMonth(currentMonthId),
  ]);

  const totalIncomeCents = incomes.reduce((total, income) => total + income.amountCents, 0);
  const recurringBillsCents = bills
    .filter((bill) => bill.isRecurring)
    .reduce((total, bill) => total + bill.amountCents, 0);
  const invoicesCents = invoices.reduce((total, invoice) => total + invoice.amountCents, 0);

  return {
    totalIncomeCents,
    recurringBillsCents,
    invoicesCents,
    baselineRemainingCents: totalIncomeCents - recurringBillsCents - invoicesCents,
  };
}
