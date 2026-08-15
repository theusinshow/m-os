import { eq } from "drizzle-orm";
import { db } from "@/db/client";
import { creditCards, pluggyItems } from "@/db/schema";

export type PluggyConnection = {
  id: string;
  itemId: string;
  accountId: string | null;
  connectorName: string | null;
  cardId: string | null;
  cardName: string | null;
  status: string;
  lastSyncedAt: Date | null;
  error: string | null;
};

/** Lists the user's Pluggy connections (one row per linked credit account). */
export async function getPluggyConnections(userId: string): Promise<PluggyConnection[]> {
  if (!db) {
    return [];
  }

  return db
    .select({
      id: pluggyItems.id,
      itemId: pluggyItems.itemId,
      accountId: pluggyItems.accountId,
      connectorName: pluggyItems.connectorName,
      cardId: pluggyItems.cardId,
      cardName: creditCards.name,
      status: pluggyItems.status,
      lastSyncedAt: pluggyItems.lastSyncedAt,
      error: pluggyItems.error,
    })
    .from(pluggyItems)
    .leftJoin(creditCards, eq(pluggyItems.cardId, creditCards.id))
    .where(eq(pluggyItems.userId, userId))
    .orderBy(pluggyItems.createdAt);
}
