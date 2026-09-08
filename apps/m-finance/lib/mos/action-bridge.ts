import { z } from "zod";
import { db } from "@/db/client";
import { bills } from "@/db/schema";
import { composeMonthDate } from "@/lib/due-date";
import { getCurrentMonthForUser } from "@/lib/months";
import { createRecurringBillSeries } from "@/lib/recurrence";

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
    // O helper lança; a ponte do M/OS responde com `{ ok: false }` e uma frase
    // que o Hermes sabe mostrar, então a falha é traduzida aqui.
    try {
      const { rule, billIds } = await createRecurringBillSeries({
        userId,
        name: payload.description,
        amountCents: payload.amountCents,
        dueDay: payload.dueDay,
        startMonth: month.month,
        startYear: month.year,
      });

      return { ok: true, billId: billIds[0] ?? rule.id };
    } catch (error) {
      return {
        ok: false,
        error:
          error instanceof Error
            ? error.message
            : "Não consegui criar a regra de recorrência agora.",
      };
    }
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
