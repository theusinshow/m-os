import { and, asc, desc, eq, sql } from "drizzle-orm";
import { db } from "@/db/client";
import { billCategories, creditCardExpenses, creditCards, months } from "@/db/schema";

export async function getCardById(userId: string, cardId: string) {
  if (!db) {
    return null;
  }

  const [card] = await db
    .select()
    .from(creditCards)
    .where(and(eq(creditCards.id, cardId), eq(creditCards.userId, userId)))
    .limit(1);

  return card ?? null;
}

export async function getCardExpenses(userId: string, cardId: string, monthId: string) {
  if (!db) {
    return [];
  }

  return db
    .select({
      id: creditCardExpenses.id,
      description: creditCardExpenses.description,
      amountCents: creditCardExpenses.amountCents,
      purchaseDate: creditCardExpenses.purchaseDate,
      installmentId: creditCardExpenses.installmentId,
      installmentNumber: creditCardExpenses.installmentNumber,
      installmentTotal: creditCardExpenses.installmentTotal,
      categoryName: billCategories.name,
      isSubscription: sql<boolean>`${creditCardExpenses.subscriptionId} is not null`,
    })
    .from(creditCardExpenses)
    .leftJoin(billCategories, eq(creditCardExpenses.categoryId, billCategories.id))
    .where(
      and(
        eq(creditCardExpenses.userId, userId),
        eq(creditCardExpenses.cardId, cardId),
        eq(creditCardExpenses.monthId, monthId),
      ),
    )
    .orderBy(asc(creditCardExpenses.purchaseDate), asc(creditCardExpenses.createdAt));
}

export async function getCardExpenseHistory(userId: string, cardId: string) {
  if (!db) {
    return [];
  }

  return db
    .select({
      id: creditCardExpenses.id,
      description: creditCardExpenses.description,
      amountCents: creditCardExpenses.amountCents,
      purchaseDate: creditCardExpenses.purchaseDate,
      installmentId: creditCardExpenses.installmentId,
      installmentNumber: creditCardExpenses.installmentNumber,
      installmentTotal: creditCardExpenses.installmentTotal,
      month: months.month,
      year: months.year,
    })
    .from(creditCardExpenses)
    .innerJoin(months, eq(creditCardExpenses.monthId, months.id))
    .where(and(eq(creditCardExpenses.userId, userId), eq(creditCardExpenses.cardId, cardId)))
    .orderBy(
      desc(months.year),
      desc(months.month),
      asc(creditCardExpenses.purchaseDate),
      asc(creditCardExpenses.createdAt),
    );
}
