import { db } from "@/db/client";
import { bills } from "@/db/schema";
import { composeMonthDate } from "@/lib/due-date";
import { ensureConsecutiveMonthsForUser } from "@/lib/months";
import { createRecurringBillSeries } from "@/lib/recurrence";
import { fail, ok, type DomainResult, type MonthRecord } from "@/lib/domain/finance-actions/result";

/**
 * Como a conta se repete.
 *
 * - `once`: uma conta, neste mês.
 * - `fixed`: N meses com `seriesId` (parcelamento de conta).
 * - `ongoing`: regra em `recurrence_rules` + os próximos meses materializados.
 * - `flagged`: uma conta só, marcada `isRecurring` — o legado do WhatsApp e do
 *   M/OS quando a frase diz "recorrente" mas não diz o dia. Sem dia não há
 *   regra possível, e a conta não pode nascer vencida.
 */
export type BillSchedule =
  | { kind: "once" }
  | { kind: "fixed"; months: number }
  | { kind: "ongoing" }
  | { kind: "flagged" };

export type CreateBillInput = {
  userId: string;
  month: MonthRecord;
  name: string;
  amountCents: number;
  /** Sem dia, vence no último dia do mês — nunca nasce vencida. */
  dueDay: number | null;
  schedule: BillSchedule;
  categoryId?: string | null;
  notes?: string | null;
  whatsappPendingActionId?: string | null;
};

export type CreatedBills = {
  billIds: string[];
  /** Quantos meses receberam conta. */
  months: number;
  firstDueDate: string;
  ruleId: string | null;
};

/**
 * Cria conta(s) a pagar. É a regra única que a interface web, o WhatsApp e a
 * Action API do M/OS usam — antes eram três cópias, e a do M/OS e a do
 * WhatsApp já tinham divergido da web no tratamento de "recorrente".
 */
export async function createBillEntries(input: CreateBillInput): Promise<DomainResult<CreatedBills>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");

  const dueDay = input.dueDay ?? 31;
  // Só grava o que veio: o banco já sabe que o resto é nulo.
  const optional = {
    ...(input.categoryId ? { categoryId: input.categoryId } : {}),
    ...(input.notes ? { notes: input.notes } : {}),
    ...(input.whatsappPendingActionId ? { whatsappPendingActionId: input.whatsappPendingActionId } : {}),
  };

  if (input.schedule.kind === "ongoing") {
    try {
      const { rule, billIds, months } = await createRecurringBillSeries({
        userId: input.userId,
        name: input.name,
        amountCents: input.amountCents,
        dueDay,
        startMonth: input.month.month,
        startYear: input.month.year,
        categoryId: input.categoryId ?? null,
        notes: input.notes ?? null,
        whatsappPendingActionId: input.whatsappPendingActionId ?? null,
      });
      return ok({
        billIds,
        months,
        firstDueDate: composeMonthDate(input.month.year, input.month.month, dueDay),
        ruleId: rule.id,
      });
    } catch (error) {
      return fail(
        "write_failed",
        error instanceof Error ? error.message : "Não consegui criar a regra de recorrência agora.",
      );
    }
  }

  if (input.schedule.kind === "fixed" && input.schedule.months > 1) {
    const total = input.schedule.months;
    const targetMonths = await ensureConsecutiveMonthsForUser(
      input.userId,
      input.month.month,
      input.month.year,
      total,
    );
    const seriesId = crypto.randomUUID();
    const created = await db
      .insert(bills)
      .values(
        targetMonths.map((month, index) => ({
          userId: input.userId,
          monthId: month.id,
          name: input.name,
          amountCents: input.amountCents,
          dueDate: composeMonthDate(month.year, month.month, dueDay),
          isRecurring: false,
          seriesId,
          seriesNumber: index + 1,
          seriesTotal: total,
          status: "pending" as const,
          ...optional,
        })),
      )
      .returning({ id: bills.id });
    if (created.length === 0) return fail("write_failed", "Não consegui gravar a conta agora.");
    return ok({
      billIds: created.map((row) => row.id),
      months: total,
      firstDueDate: composeMonthDate(input.month.year, input.month.month, dueDay),
      ruleId: null,
    });
  }

  const dueDate = composeMonthDate(input.month.year, input.month.month, dueDay);
  const [created] = await db
    .insert(bills)
    .values({
      userId: input.userId,
      monthId: input.month.id,
      name: input.name,
      amountCents: input.amountCents,
      dueDate,
      isRecurring: input.schedule.kind === "flagged",
      status: "pending",
      ...optional,
    })
    .returning({ id: bills.id });

  if (!created) return fail("write_failed", "Não consegui gravar a conta agora.");
  return ok({ billIds: [created.id], months: 1, firstDueDate: dueDate, ruleId: null });
}

/**
 * A tradução de `{ dueDay, isRecurring }` — o formato que o WhatsApp e o M/OS
 * falam — para o agendamento. Recorrente COM dia vira regra; recorrente SEM
 * dia vira uma conta marcada.
 */
export function scheduleFromFlags(isRecurring: boolean, dueDay: number | null): BillSchedule {
  if (!isRecurring) return { kind: "once" };
  return dueDay ? { kind: "ongoing" } : { kind: "flagged" };
}
