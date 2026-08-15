import { desc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { monthlySnapshots, months } from "@/db/schema";

export async function getMonthlySnapshots(userId: string) {
  if (!db) {
    return [];
  }

  return db
    .select({
      id: monthlySnapshots.id,
      month: months.month,
      year: months.year,
      totalIncomeCents: monthlySnapshots.totalIncomeCents,
      totalBillsCents: monthlySnapshots.totalBillsCents,
      totalInvoicesCents: monthlySnapshots.totalInvoicesCents,
      totalPaidCents: monthlySnapshots.totalPaidCents,
      totalPendingCents: monthlySnapshots.totalPendingCents,
      totalOverdueCents: monthlySnapshots.totalOverdueCents,
      estimatedRemainingCents: monthlySnapshots.estimatedRemainingCents,
      monthHealth: monthlySnapshots.monthHealth,
      updatedAt: monthlySnapshots.updatedAt,
    })
    .from(monthlySnapshots)
    .innerJoin(months, eq(monthlySnapshots.monthId, months.id))
    .where(eq(monthlySnapshots.userId, userId))
    .orderBy(desc(months.year), desc(months.month));
}
