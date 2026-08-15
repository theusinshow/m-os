import { and, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { monthlySnapshots } from "@/db/schema";
import { getDashboardSummary } from "@/lib/calculations/dashboard";
import { getBillsByMonth } from "@/lib/bills";
import { getInvoicesByMonth } from "@/lib/cards";
import { getIncomesByMonth } from "@/lib/incomes";

/**
 * Computes the month summary and upserts its monthly_snapshot. Shared by the
 * manual "save snapshot" action and the automatic snapshot taken when a new
 * month is generated, so history accrues without the user remembering to save.
 */
export async function writeMonthSnapshot(userId: string, monthId: string) {
  if (!db) return;

  const [incomes, bills, invoices] = await Promise.all([
    getIncomesByMonth(monthId),
    getBillsByMonth(monthId),
    getInvoicesByMonth(monthId),
  ]);
  const summary = getDashboardSummary({ incomes, bills, invoices });

  const existing = await db
    .select({ id: monthlySnapshots.id })
    .from(monthlySnapshots)
    .where(and(eq(monthlySnapshots.userId, userId), eq(monthlySnapshots.monthId, monthId)))
    .limit(1);

  const values = {
    userId,
    monthId,
    totalIncomeCents: summary.totalIncomeCents,
    totalBillsCents: summary.totalBillsCents,
    totalInvoicesCents: summary.totalInvoicesCents,
    totalPaidCents: summary.totalPaidCents,
    totalPendingCents: summary.totalPendingCents,
    totalOverdueCents: summary.totalOverdueCents,
    estimatedRemainingCents: summary.estimatedRemainingCents,
    monthHealth: summary.monthHealth,
    updatedAt: new Date(),
  };

  if (existing[0]) {
    await db.update(monthlySnapshots).set(values).where(eq(monthlySnapshots.id, existing[0].id));
  } else {
    await db.insert(monthlySnapshots).values(values);
  }
}
