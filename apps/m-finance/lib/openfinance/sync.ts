import { and, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { creditCardExpenses, pluggyItems } from "@/db/schema";
import { getCardById } from "@/lib/card-expenses";
import { ensureMonthForUser } from "@/lib/months";
import { syncInvoiceTotal } from "@/lib/invoice-sync";
import { listTransactions, type PluggyTransaction } from "@/lib/openfinance/pluggy";

const DEFAULT_LOOKBACK_DAYS = 90;

type MonthRecord = { id: string; month: number; year: number };

function isoDaysAgo(days: number) {
  const d = new Date();
  d.setDate(d.getDate() - days);
  return d.toISOString().slice(0, 10);
}

function monthPartsFromDate(d: Date) {
  return { month: d.getUTCMonth() + 1, year: d.getUTCFullYear() };
}

function isoDate(d: Date) {
  return d.toISOString().slice(0, 10);
}

/**
 * For the spike we treat outflows (negative amounts) as card purchases and skip
 * positive entries (invoice payments, refunds, cashback). Returns the expense
 * amount in positive cents, or null when the row should be ignored.
 */
function expenseCents(tx: PluggyTransaction): number | null {
  if (tx.amount >= 0) return null;
  const cents = Math.round(Math.abs(tx.amount) * 100);
  return cents > 0 ? cents : null;
}

type SyncResult = {
  cardId: string;
  imported: number;
  skipped: number;
};

/**
 * Pulls transactions for every mapped credit-card account under a Pluggy item
 * and upserts them into local expenses (deduped by provider id), then rebuilds
 * the affected invoices. Safe to re-run: a second sync updates in place.
 */
export async function syncPluggyItem(appUserId: string, itemId: string): Promise<SyncResult[]> {
  if (!db) {
    throw new Error("Database is not configured.");
  }
  const database = db;

  const mappings = await database
    .select()
    .from(pluggyItems)
    .where(and(eq(pluggyItems.userId, appUserId), eq(pluggyItems.itemId, itemId)));

  const results: SyncResult[] = [];

  for (const mapping of mappings) {
    if (!mapping.cardId || !mapping.accountId) continue;

    const card = await getCardById(appUserId, mapping.cardId);
    if (!card) continue;

    const from = mapping.lastSyncedAt
      ? mapping.lastSyncedAt.toISOString().slice(0, 10)
      : isoDaysAgo(DEFAULT_LOOKBACK_DAYS);

    let transactions: PluggyTransaction[];
    try {
      transactions = await listTransactions(mapping.accountId, from);
    } catch (error) {
      await database
        .update(pluggyItems)
        .set({
          status: "error",
          error: error instanceof Error ? error.message : String(error),
          updatedAt: new Date(),
        })
        .where(eq(pluggyItems.id, mapping.id));
      continue;
    }

    // Resolve (and create) every month touched once, so we don't hit the DB per
    // transaction and can resync each invoice exactly once at the end.
    const monthCache = new Map<string, MonthRecord>();
    let imported = 0;
    let skipped = 0;

    for (const tx of transactions) {
      const cents = expenseCents(tx);
      if (cents === null) {
        skipped += 1;
        continue;
      }

      const txDate = new Date(tx.date);
      const parts = monthPartsFromDate(txDate);
      const key = `${parts.year}-${parts.month}`;
      let month = monthCache.get(key);
      if (!month) {
        const row = await ensureMonthForUser(appUserId, parts.month, parts.year);
        month = { id: row.id, month: row.month, year: row.year };
        monthCache.set(key, month);
      }

      await database
        .insert(creditCardExpenses)
        .values({
          userId: appUserId,
          cardId: mapping.cardId,
          monthId: month.id,
          description: tx.description || tx.descriptionRaw || "Compra importada",
          amountCents: cents,
          purchaseDate: isoDate(txDate),
          source: "openfinance",
          externalId: tx.id,
        })
        .onConflictDoUpdate({
          target: [creditCardExpenses.userId, creditCardExpenses.externalId],
          set: {
            cardId: mapping.cardId,
            monthId: month.id,
            description: tx.description || tx.descriptionRaw || "Compra importada",
            amountCents: cents,
            purchaseDate: isoDate(txDate),
            updatedAt: new Date(),
          },
        });

      imported += 1;
    }

    // Rebuild invoices for each touched month inside a transaction.
    await database.transaction(async (tx) => {
      for (const month of monthCache.values()) {
        await syncInvoiceTotal(tx, appUserId, mapping.cardId!, month, card.dueDay);
      }
    });

    await database
      .update(pluggyItems)
      .set({ status: "ready", error: null, lastSyncedAt: new Date(), updatedAt: new Date() })
      .where(eq(pluggyItems.id, mapping.id));

    results.push({ cardId: mapping.cardId, imported, skipped });
  }

  return results;
}
