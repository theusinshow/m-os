import { and, eq, sql } from "drizzle-orm";
import { db } from "@/db/client";
import { bills, creditCardInvoices } from "@/db/schema";
import { fail, ok, type DomainResult } from "@/lib/domain/finance-actions/result";

export type PaidBill = { id: string; name: string; amountCents: number; dueDate: string };
export type PaidInvoice = { id: string; cardId: string; amountCents: number; dueDate: string };

/**
 * O que o preview mostrou. Se o banco mudou entre o cartão e o "confirmar", a
 * ação recusa em vez de agir sobre outra coisa — marcar como paga uma conta de
 * R$ 300 quando o cartão dizia R$ 120 é exatamente o erro que o preview existe
 * para impedir.
 */
export type PreviewExpectation = { amountCents?: number | null };

/**
 * As contas não pagas de um mês que contêm o termo — o "paguei a luz" do
 * WhatsApp. A decisão de qual delas pagar é de quem chama: uma, paga; várias,
 * pergunta.
 */
export async function findUnpaidBillsByName(userId: string, monthId: string, term: string) {
  if (!db) return [];
  return db
    .select({ id: bills.id, name: bills.name, amountCents: bills.amountCents, dueDate: bills.dueDate })
    .from(bills)
    .where(
      and(
        eq(bills.userId, userId),
        eq(bills.monthId, monthId),
        sql`${bills.status} <> 'paid'`,
        sql`${bills.name} ILIKE ${"%" + term + "%"}`,
      ),
    )
    .orderBy(bills.dueDate);
}

/** Marca UMA conta como paga, pelo id. */
export async function markBillPaid(
  userId: string,
  billId: string,
  expectation: PreviewExpectation = {},
  now = new Date(),
): Promise<DomainResult<PaidBill>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");

  const [bill] = await db
    .select({ id: bills.id, name: bills.name, amountCents: bills.amountCents, dueDate: bills.dueDate, status: bills.status })
    .from(bills)
    .where(and(eq(bills.id, billId), eq(bills.userId, userId)))
    .limit(1);

  if (!bill) return fail("not_found", "Conta não encontrada no M-Finance.");
  if (bill.status === "paid") return fail("already_paid", `A conta "${bill.name}" já está marcada como paga.`);
  if (expectation.amountCents != null && expectation.amountCents !== bill.amountCents) {
    return fail("stale_preview", `O valor de "${bill.name}" mudou desde o preview. Peça de novo.`);
  }

  await db
    .update(bills)
    .set({ status: "paid", paidAt: now, updatedAt: now })
    .where(and(eq(bills.id, bill.id), eq(bills.userId, userId)));

  return ok({ id: bill.id, name: bill.name, amountCents: bill.amountCents, dueDate: bill.dueDate });
}

/**
 * Marca UMA fatura como paga — pelo id da fatura, ou pelo cartão + mês. Só
 * fatura LANÇADA: a estimada não existe no banco, e pagar um palpite seria
 * gravar um valor que o banco nunca cobrou.
 */
export async function markInvoicePaid(
  userId: string,
  target: { invoiceId: string } | { cardId: string; monthId: string },
  expectation: PreviewExpectation = {},
  now = new Date(),
): Promise<DomainResult<PaidInvoice>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");

  const where =
    "invoiceId" in target
      ? and(eq(creditCardInvoices.id, target.invoiceId), eq(creditCardInvoices.userId, userId))
      : and(
          eq(creditCardInvoices.userId, userId),
          eq(creditCardInvoices.cardId, target.cardId),
          eq(creditCardInvoices.monthId, target.monthId),
        );

  const [invoice] = await db
    .select({
      id: creditCardInvoices.id,
      cardId: creditCardInvoices.cardId,
      amountCents: creditCardInvoices.amountCents,
      dueDate: creditCardInvoices.dueDate,
      status: creditCardInvoices.status,
    })
    .from(creditCardInvoices)
    .where(where)
    .limit(1);

  if (!invoice) return fail("not_found", "Fatura não encontrada — ela ainda não foi lançada neste mês.");
  if (invoice.status === "paid") return fail("already_paid", "Essa fatura já está marcada como paga.");
  if (expectation.amountCents != null && expectation.amountCents !== invoice.amountCents) {
    return fail("stale_preview", "O valor da fatura mudou desde o preview. Peça de novo.");
  }

  await db
    .update(creditCardInvoices)
    .set({ status: "paid", paidAt: now, updatedAt: now })
    .where(eq(creditCardInvoices.id, invoice.id));

  return ok({ id: invoice.id, cardId: invoice.cardId, amountCents: invoice.amountCents, dueDate: invoice.dueDate });
}
