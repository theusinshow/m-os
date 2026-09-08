import { db } from "@/db/client";
import { bills, recurrenceRules } from "@/db/schema";
import { composeMonthDate } from "@/lib/due-date";
import { ensureConsecutiveMonthsForUser } from "@/lib/months";

/**
 * Quantos meses uma recorrência materializa de uma vez. Doze é o horizonte que
 * o app já usava no bridge do M/OS e no WhatsApp; o número mora aqui agora
 * porque as três portas de entrada passaram a compartilhar a mesma função.
 */
export const RECURRING_PREGENERATE_MONTHS = 12;

type NamedBill = { name: string };

/**
 * Normaliza o nome para comparar recorrência com o que já existe no mês
 * seguinte. "  ÁGUA " e "agua" são a mesma conta para quem digitou.
 */
function recurrenceKey(name: string) {
  return name
    .trim()
    .toLocaleLowerCase("pt-BR")
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "");
}

/**
 * As recorrências do mês ativo que ainda não têm contraparte no mês seguinte.
 *
 * É o que decide se o card "Revisar recorrências" aparece. A condição anterior
 * era "o mês seguinte ainda não existe", e uma série parcelada de 22 meses
 * criava linhas de mês até 2028 — o card sumia para sempre e nenhuma
 * recorrência voltava a ser gerada.
 */
export function pendingRecurrences<T extends NamedBill>(
  recurringBills: T[],
  nextMonthBills: NamedBill[],
): T[] {
  const existing = new Set(nextMonthBills.map((bill) => recurrenceKey(bill.name)));
  return recurringBills.filter((bill) => !existing.has(recurrenceKey(bill.name)));
}

/**
 * Cria a regra em `recurrence_rules` e materializa os próximos meses como
 * contas ligadas a ela.
 *
 * Existiam três cópias disto — bridge do M/OS, executor do WhatsApp e nada no
 * app, que só marcava `isRecurring` numa linha solta e prometia uma repetição
 * que nunca acontecia.
 */
export async function createRecurringBillSeries({
  userId,
  name,
  amountCents,
  dueDay,
  startMonth,
  startYear,
  categoryId = null,
  notes = null,
  whatsappPendingActionId = null,
  months = RECURRING_PREGENERATE_MONTHS,
}: {
  userId: string;
  name: string;
  amountCents: number;
  dueDay: number;
  startMonth: number;
  startYear: number;
  categoryId?: string | null;
  notes?: string | null;
  whatsappPendingActionId?: string | null;
  months?: number;
}) {
  if (!db) {
    throw new Error("Banco de dados indisponível no momento.");
  }

  const [rule] = await db
    .insert(recurrenceRules)
    .values({
      userId,
      name,
      categoryId,
      defaultAmountCents: amountCents,
      dueDay,
      isVariableAmount: false,
      isActive: true,
      notes,
    })
    .returning();

  if (!rule) {
    throw new Error("Não consegui criar a regra de recorrência agora.");
  }

  const targetMonths = await ensureConsecutiveMonthsForUser(
    userId,
    startMonth,
    startYear,
    months,
  );

  const created = await db
    .insert(bills)
    .values(
      targetMonths.map((month) => ({
        userId,
        monthId: month.id,
        categoryId,
        recurrenceRuleId: rule.id,
        name,
        amountCents,
        dueDate: composeMonthDate(month.year, month.month, dueDay),
        isRecurring: true,
        status: "pending" as const,
        notes,
        whatsappPendingActionId,
      })),
    )
    .returning({ id: bills.id });

  return { rule, billIds: created.map((row) => row.id), months: targetMonths.length };
}
