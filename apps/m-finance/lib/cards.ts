import { and, asc, desc, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { creditCardInvoices, creditCards } from "@/db/schema";
import { derivePayableStatus } from "@/lib/status";

export async function getManagedCreditCards(userId: string) {
  if (!db) {
    return [];
  }

  return db
    .select({
      id: creditCards.id,
      name: creditCards.name,
      cardType: creditCards.cardType,
      dueDay: creditCards.dueDay,
      isActive: creditCards.isActive,
    })
    .from(creditCards)
    .where(eq(creditCards.userId, userId))
    .orderBy(desc(creditCards.isActive), asc(creditCards.name));
}

export async function getCreditCards(userId: string) {
  if (!db) {
    return [];
  }

  const cards = await db
    .select()
    .from(creditCards)
    .where(and(eq(creditCards.userId, userId), eq(creditCards.isActive, true)))
    .orderBy(asc(creditCards.name));

  const uniqueCards = new Map<string, (typeof cards)[number]>();

  for (const card of cards) {
    const key = `${card.name.trim().toLowerCase()}-${card.cardType}`;

    if (!uniqueCards.has(key)) {
      uniqueCards.set(key, card);
    }
  }

  return Array.from(uniqueCards.values());
}

export async function getInvoiceForCardMonth(userId: string, cardId: string, monthId: string) {
  if (!db) {
    return null;
  }

  const [row] = await db
    .select({
      id: creditCardInvoices.id,
      amountCents: creditCardInvoices.amountCents,
      dueDate: creditCardInvoices.dueDate,
      status: creditCardInvoices.status,
    })
    .from(creditCardInvoices)
    .where(
      and(
        eq(creditCardInvoices.userId, userId),
        eq(creditCardInvoices.cardId, cardId),
        eq(creditCardInvoices.monthId, monthId),
      ),
    )
    .limit(1);

  if (!row) {
    return null;
  }

  return { ...row, status: derivePayableStatus(row.status, row.dueDate) };
}

export async function getInvoicesByMonth(monthId: string) {
  if (!db) {
    return [];
  }

  const rows = await db
    .select({
      id: creditCardInvoices.id,
      name: creditCards.name,
      amountCents: creditCardInvoices.amountCents,
      dueDate: creditCardInvoices.dueDate,
      status: creditCardInvoices.status,
      cardType: creditCards.cardType,
    })
    .from(creditCardInvoices)
    .innerJoin(creditCards, eq(creditCardInvoices.cardId, creditCards.id))
    .where(eq(creditCardInvoices.monthId, monthId))
    .orderBy(asc(creditCardInvoices.dueDate));

  return rows.map((invoice) => ({
    ...invoice,
    status: derivePayableStatus(invoice.status, invoice.dueDate),
  }));
}
