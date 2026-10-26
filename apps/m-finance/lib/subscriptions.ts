import { and, asc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { creditCards, subscriptions } from "@/db/schema";

export async function getSubscriptionsForUser(userId: string) {
  if (!db) {
    return [];
  }

  const rows = await db
    .select({ subscription: subscriptions, cardName: creditCards.name })
    .from(subscriptions)
    .leftJoin(creditCards, eq(subscriptions.cardId, creditCards.id))
    .where(eq(subscriptions.userId, userId))
    .orderBy(asc(subscriptions.nextChargeDate));
  return rows.map((row) => ({ ...row.subscription, cardName: row.cardName }));
}

export async function getSubscriptionById(userId: string, id: string) {
  if (!db) {
    return null;
  }

  const [row] = await db
    .select()
    .from(subscriptions)
    .where(and(eq(subscriptions.id, id), eq(subscriptions.userId, userId)))
    .limit(1);

  return row ?? null;
}
