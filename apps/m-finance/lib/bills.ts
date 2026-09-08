import { and, asc, eq, isNotNull } from "drizzle-orm";
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
      seriesId: bills.seriesId,
      seriesNumber: bills.seriesNumber,
      seriesTotal: bills.seriesTotal,
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

/**
 * Todas as parcelas de todas as séries do usuário, de qualquer mês.
 *
 * O dashboard vive num mês só, e um parcelamento não cabe num mês: para dizer
 * quanto ainda falta de um financiamento de 22 vezes é preciso olhar a série
 * inteira.
 */
export async function getInstallmentBillsForUser(userId: string) {
  if (!db) {
    return [];
  }

  const rows = await db
    .select({
      name: bills.name,
      amountCents: bills.amountCents,
      seriesId: bills.seriesId,
      seriesNumber: bills.seriesNumber,
      seriesTotal: bills.seriesTotal,
      dueDate: bills.dueDate,
      status: bills.status,
    })
    .from(bills)
    .where(and(eq(bills.userId, userId), isNotNull(bills.seriesId)))
    .orderBy(asc(bills.dueDate));

  return rows.map((bill) => ({
    ...bill,
    status: derivePayableStatus(bill.status, bill.dueDate),
  }));
}
