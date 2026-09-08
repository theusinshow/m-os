import { and, eq, sql } from "drizzle-orm";
import { creditCardExpenses, creditCardInvoices } from "@/db/schema";
import type { db } from "@/db/client";
import { composeMonthDate } from "@/lib/due-date";
import { nextInvoiceTotal } from "@/lib/calculations/invoice-breakdown";

type Tx = Parameters<Parameters<NonNullable<typeof db>["transaction"]>[0]>[0];
/** A leitura da soma vale dentro ou fora de transação. */
type Queryable = Tx | NonNullable<typeof db>;
type MonthRecord = { id: string; month: number; year: number };

/**
 * Reconcilia a fatura do mês depois de mexer nas compras do cartão.
 *
 * Antes esta função **era** o total: fatura = soma das compras, e sem compras
 * a fatura era apagada. Isso deixava as duas coisas incompatíveis — quem
 * digitava R$ 1.027 e depois classificava uma compra de R$ 120 via a fatura
 * virar R$ 120, com R$ 907 evaporando. Agora o total é o que o cartão cobra e
 * as compras explicam parte dele; a regra de quem manda mora em
 * `nextInvoiceTotal`.
 */
export async function syncInvoiceTotal(
  tx: Queryable,
  userId: string,
  cardId: string,
  month: MonthRecord,
  dueDay: number,
  /** A soma das compras ANTES desta operação, para saber se o total era derivado. */
  previousSum = 0,
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

  const newSum = Number(row?.total ?? 0);

  const [invoice] = await tx
    .select({ id: creditCardInvoices.id, amountCents: creditCardInvoices.amountCents })
    .from(creditCardInvoices)
    .where(
      and(
        eq(creditCardInvoices.userId, userId),
        eq(creditCardInvoices.cardId, cardId),
        eq(creditCardInvoices.monthId, month.id),
      ),
    )
    .limit(1);

  const total = nextInvoiceTotal({
    currentTotal: invoice?.amountCents ?? null,
    previousSum,
    newSum,
  });

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
    return;
  }

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

/** A soma das compras do cartão no mês, lida antes de alterá-las. */
export async function sumCardExpenses(
  tx: Queryable,
  userId: string,
  cardId: string,
  monthId: string,
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
        eq(creditCardExpenses.monthId, monthId),
      ),
    );

  return Number(row?.total ?? 0);
}
