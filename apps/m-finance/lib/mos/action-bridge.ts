import { z } from "zod";
import { db } from "@/db/client";
import { createBillEntries, scheduleFromFlags } from "@/lib/domain/finance-actions/create-bill";
import { getCurrentMonthForUser } from "@/lib/months";

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
 * Cria uma conta a partir de uma ação proposta pelo Hermes e confirmada no
 * M/OS.
 *
 * A regra de escrita é `createBillEntries`, a mesma do WhatsApp e da tela —
 * sem o acoplamento com `whatsappPendingActions`: esta ação não nasceu de uma
 * mensagem de WhatsApp, e forçar uma linha pendente só para satisfazer a
 * foreign key seria inventar um registro que não existe.
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

  // Recorrente com dia vira regra + próximos meses; sem dia, uma conta marcada.
  const created = await createBillEntries({
    userId,
    month,
    name: payload.description,
    amountCents: payload.amountCents,
    dueDay: payload.dueDay,
    schedule: scheduleFromFlags(payload.isRecurring, payload.dueDay),
  });

  if (!created.ok) {
    return { ok: false, error: created.message };
  }

  return { ok: true, billId: created.value.billIds[0] ?? created.value.ruleId ?? "" };
}
