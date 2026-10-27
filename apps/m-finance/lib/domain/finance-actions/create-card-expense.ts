import { db } from "@/db/client";
import { creditCardExpenses } from "@/db/schema";
import { sumCardExpenses, syncInvoiceTotal } from "@/lib/invoice-sync";
import { ensureConsecutiveMonthsForUser } from "@/lib/months";
import { fail, ok, type DomainResult, type MonthRecord } from "@/lib/domain/finance-actions/result";

export type CreateCardExpenseInput = {
  userId: string;
  card: { id: string; name: string; dueDay: number };
  /** O mês da primeira parcela (ou da compra à vista). */
  month: MonthRecord;
  description: string;
  amountCents: number;
  purchaseDate: string | null;
  /** 1 = à vista. */
  installments: number;
  whatsappPendingActionId?: string | null;
};

export type CreatedCardExpense = {
  expenseIds: string[];
  installmentId: string | null;
  installments: number;
  months: MonthRecord[];
};

/**
 * Parte um total em N parcelas que somam exatamente o total: o resto vai para
 * as primeiras, um centavo cada.
 */
export function splitInstallments(totalCents: number, installments: number) {
  const base = Math.floor(totalCents / installments);
  const remainder = totalCents - base * installments;
  return Array.from({ length: installments }, (_, index) => base + (index < remainder ? 1 : 0));
}

/**
 * Lança uma compra no cartão — à vista ou parcelada — e reconcilia a fatura de
 * cada mês tocado.
 *
 * Existia duas vezes: na web (sem transação) e no WhatsApp (com). Agora é uma,
 * sempre em transação: a compra e o total da fatura mudam juntos ou não mudam.
 * A soma anterior de cada mês é lida antes de inserir, porque é ela que diz se
 * o total da fatura nasceu das compras ou foi digitado (`nextInvoiceTotal`).
 */
export async function createCardExpense(
  input: CreateCardExpenseInput,
): Promise<DomainResult<CreatedCardExpense>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const database = db;

  const installments = Math.max(1, Math.round(input.installments));
  if (installments > 1 && input.amountCents < installments) {
    return fail("invalid", "O valor total é muito baixo para essa quantidade de parcelas.");
  }

  const targetMonths =
    installments > 1
      ? await ensureConsecutiveMonthsForUser(input.userId, input.month.month, input.month.year, installments)
      : [input.month];
  const amounts = splitInstallments(input.amountCents, installments);
  const installmentId = installments > 1 ? crypto.randomUUID() : null;

  const expenseIds = await database.transaction(async (tx) => {
    const previousSums = new Map<string, number>();
    for (const month of targetMonths) {
      previousSums.set(month.id, await sumCardExpenses(tx, input.userId, input.card.id, month.id));
    }

    const created = await tx
      .insert(creditCardExpenses)
      .values(
        targetMonths.map((month, index) => ({
          userId: input.userId,
          cardId: input.card.id,
          monthId: month.id,
          description: input.description,
          amountCents: amounts[index],
          purchaseDate: input.purchaseDate,
          installmentId,
          installmentNumber: installmentId ? index + 1 : null,
          installmentTotal: installmentId ? installments : null,
          ...(input.whatsappPendingActionId ? { whatsappPendingActionId: input.whatsappPendingActionId } : {}),
        })),
      )
      .returning({ id: creditCardExpenses.id });

    for (const month of targetMonths) {
      await syncInvoiceTotal(tx, input.userId, input.card.id, month, input.card.dueDay, previousSums.get(month.id) ?? 0);
    }

    return created.map((row) => row.id);
  });

  return ok({ expenseIds, installmentId, installments, months: targetMonths });
}
