import { and, asc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { billCategories, bills } from "@/db/schema";
import { derivePayableStatus } from "@/lib/status";

export async function getBillCategories(userId: string) {
  if (!db) {
    return [];
  }

  return db
    .select()
    .from(billCategories)
    .where(and(eq(billCategories.userId, userId), eq(billCategories.isArchived, false)))
    .orderBy(asc(billCategories.name));
}

export async function getBillsByMonth(monthId: string) {
  if (!db) {
    return [];
  }

  const rows = await db
    .select({
      id: bills.id,
      categoryId: bills.categoryId,
      name: bills.name,
      amountCents: bills.amountCents,
      dueDate: bills.dueDate,
      isRecurring: bills.isRecurring,
      status: bills.status,
      categoryName: billCategories.name,
    })
    .from(bills)
    .leftJoin(billCategories, eq(bills.categoryId, billCategories.id))
    .where(eq(bills.monthId, monthId))
    .orderBy(asc(bills.dueDate));

  return rows.map((bill) => ({
    ...bill,
    status: derivePayableStatus(bill.status, bill.dueDate),
  }));
}

export async function getRecurringBillsByMonth(monthId: string) {
  const rows = await getBillsByMonth(monthId);

  return rows.filter((bill) => bill.isRecurring);
}
