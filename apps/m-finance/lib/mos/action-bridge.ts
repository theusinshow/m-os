import { z } from "zod";
import { db } from "@/db/client";
import { bills, recurrenceRules } from "@/db/schema";
import { composeMonthDate } from "@/lib/due-date";
import { ensureConsecutiveMonthsForUser, getCurrentMonthForUser } from "@/lib/months";

const RECURRING_PREGENERATE_MONTHS = 12;

const billPayloadSchema = z.object({
  amountCents: z.number().int().positive(),
  description: z.string().trim().min(1),
  dueDay: z.number().int().min(1).max(31).nullable(),
  isRecurring: z.boolean(),
});

export type MosActionResult =
  | { ok: true; billId: string }
  | { ok: false; error: string };

/**
 * Cria uma conta a partir de uma acao proposta pelo Hermes e confirmada no
 * M/OS. Espelha `executeCreateBill` de `lib/whatsapp/action-executor.ts`,
 * sem o acoplamento com `whatsappPendingActions` — esta acao nao nasceu de
 * uma mensagem de WhatsApp, e forcar uma linha pendente so para satisfazer a
 * foreign key seria inventar um registro que nao existe.
 */
export async function createBillFromMosAction(
  userId: string,
  rawArgs: unknown,
): Promise<MosActionResult> {
  if (!db) {
    return { ok: false, error: "Banco de dados indisponível no momento." };
  }

  const parsed = billPayloadSchema.safeParse(rawArgs);
  if (!parsed.success) {
    return { ok: false, error: "Os argumentos da ação não batem com o esperado." };
  }

  const payload = parsed.data;
  const month = await getCurrentMonthForUser(userId);
  if (!month) {
    return { ok: false, error: "Crie o mês atual no app antes de lançar despesas por aqui." };
  }

  if (payload.isRecurring && payload.dueDay) {
    const [rule] = await db
      .insert(recurrenceRules)
      .values({
        userId,
        name: payload.description,
        defaultAmountCents: payload.amountCents,
        dueDay: payload.dueDay,
        isVariableAmount: false,
        isActive: true,
      })
      .returning();

    if (!rule) {
      return { ok: false, error: "Não consegui criar a regra de recorrência agora." };
    }

    const targetMonths = await ensureConsecutiveMonthsForUser(
      userId,
      month.month,
      month.year,
      RECURRING_PREGENERATE_MONTHS,
    );
    const recurringDueDay = payload.dueDay;

    const created = await db
      .insert(bills)
      .values(
        targetMonths.map((targetMonth) => ({
          userId,
          monthId: targetMonth.id,
          recurrenceRuleId: rule.id,
          name: payload.description,
          amountCents: payload.amountCents,
          dueDate: composeMonthDate(targetMonth.year, targetMonth.month, recurringDueDay),
          isRecurring: true,
          status: "pending" as const,
        })),
      )
      .returning({ id: bills.id });

    return { ok: true, billId: created[0]?.id ?? rule.id };
  }

  const dueDay = payload.dueDay ?? 31;
  const dueDate = composeMonthDate(month.year, month.month, dueDay);

  const [created] = await db
    .insert(bills)
    .values({
      userId,
      monthId: month.id,
      name: payload.description,
      amountCents: payload.amountCents,
      dueDate,
      isRecurring: payload.isRecurring,
      status: "pending",
    })
    .returning({ id: bills.id });

  if (!created) {
    return { ok: false, error: "Não consegui gravar a conta agora." };
  }

  return { ok: true, billId: created.id };
}
