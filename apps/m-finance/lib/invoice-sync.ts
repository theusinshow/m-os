import { and, eq, sql } from "drizzle-orm";
import { creditCardExpenses, creditCardInvoices } from "@/db/schema";
import type { db } from "@/db/client";
import { composeMonthDate } from "@/lib/due-date";

type Tx = Parameters<Parameters<NonNullable<typeof db>["transaction"]>[0]>[0];
type MonthRecord = { id: string; month: number; year: number };

/**
 * Keeps the card's invoice for a month in sync with the sum of its expense
 * items: derives the total when items exist, removes the invoice when none do.
 *
 * Shared by the manual expense actions and the Open Finance sync so both write
 * invoices identically.
 */
export async function syncInvoiceTotal(
  tx: Tx,
  userId: string,
  cardId: string,
  month: MonthRecord,
  dueDay: number,
) {
  const [row] = await tx
    .select({
      total: sql<number>`coalesce(sum(${creditCardExpenses.amountCents}), 0)::int`,
    })
    .from(creditCardExpenses)
    .where(
      and(
        eq(creditCardExpenses.userId, userId),
        eq(creditCardExpenses.cardId, cardId),
        eq(creditCardExpenses.monthId, month.id),
      ),
    );

  const total = Number(row?.total ?? 0);

  if (total > 0) {
    await tx
      .insert(creditCardInvoices)
      .values({
        userId,
        cardId,
        monthId: month.id,
        amountCents: total,
        dueDate: composeMonthDate(month.year, month.month, dueDay),
        status: "pending",
      })
      .onConflictDoUpdate({
        target: [creditCardInvoices.cardId, creditCardInvoices.monthId],
        set: { amountCents: total, updatedAt: new Date() },
      });
  } else {
    await tx
      .delete(creditCardInvoices)
      .where(
        and(
          eq(creditCardInvoices.userId, userId),
          eq(creditCardInvoices.cardId, cardId),
          eq(creditCardInvoices.monthId, month.id),
        ),
      );
  }
}
