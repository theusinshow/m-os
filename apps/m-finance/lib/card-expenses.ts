import { and, asc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { creditCardExpenses, creditCards } from "@/db/schema";

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
    .select()
    .from(creditCardExpenses)
    .where(
      and(
        eq(creditCardExpenses.userId, userId),
        eq(creditCardExpenses.cardId, cardId),
        eq(creditCardExpenses.monthId, monthId),
      ),
    )
    .orderBy(asc(creditCardExpenses.purchaseDate), asc(creditCardExpenses.createdAt));
}
