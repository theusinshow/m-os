import { and, asc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { subscriptions } from "@/db/schema";

export async function getSubscriptionsForUser(userId: string) {
  if (!db) {
    return [];
  }

  return db
    .select()
    .from(subscriptions)
    .where(eq(subscriptions.userId, userId))
    .orderBy(asc(subscriptions.nextChargeDate));
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
