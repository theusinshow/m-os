import { and, desc, eq, sql } from "drizzle-orm";
import { z } from "zod";
import { db } from "@/db/client";
import {
  bills,
  creditCardExpenses,
  creditCardInvoices,
  months,
  recurrenceRules,
  whatsappPendingActions,
} from "@/db/schema";
import { getCardById } from "@/lib/card-expenses";
import { composeMonthDate } from "@/lib/due-date";
import { formatCurrency } from "@/lib/formatters/currency";
import { syncInvoiceTotal } from "@/lib/invoice-sync";
import { ensureConsecutiveMonthsForUser, getCurrentMonthForUser } from "@/lib/months";
import { updateWhatsappPendingActionStatus } from "@/lib/whatsapp/audit";

// Quantos meses à frente uma recorrência criada pelo WhatsApp já materializa
// na primeira confirmação. Espelha a ideia de "series" do app e mantém o usuário
// com as próximas contas já visíveis sem depender de um job mensal.
const RECURRING_PREGENERATE_MONTHS = 12;

const cardExpensePayloadSchema = z.object({
  amountCents: z.number().int().positive(),
  description: z.string().trim().min(1),
  cardId: z.string().uuid(),
  cardName: z.string().trim().min(1),
  purchaseDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).nullable(),
  paymentType: z.enum(["cash", "installment"]),
  installments: z.number().int().min(2).max(60).nullable(),
});

const billPayloadSchema = z.object({
  amountCents: z.number().int().positive(),
  description: z.string().trim().min(1),
  dueDay: z.number().int().min(1).max(31).nullable(),
  isRecurring: z.boolean(),
});

const markPaidPayloadSchema = z.object({
  description: z.string().trim().min(1),
});

const markInvoicePaidPayloadSchema = z.object({
  cardId: z.string().uuid(),
  cardName: z.string().trim().min(1),
});

const cancelLastPayloadSchema = z.object({});

const editLastPayloadSchema = z.object({
  newAmountCents: z.number().int().positive().nullable(),
  newDescription: z.string().trim().min(1).nullable(),
});

type PendingAction = {
  id: string;
  userId: string;
  actionType: string;
  payload: unknown;
  phone: string;
};

export async function executeWhatsappPendingAction(action: PendingAction) {
  if (action.actionType === "create_bill") {
    return executeCreateBill(action);
  }
  if (action.actionType === "mark_bill_paid") {
    return executeMarkBillPaid(action);
  }
  if (action.actionType === "mark_invoice_paid") {
    return executeMarkInvoicePaid(action);
  }
  if (action.actionType === "cancel_last_action") {
    return executeCancelLastAction(action);
  }
  if (action.actionType === "edit_last_action") {
    return executeEditLastAction(action);
  }

  if (action.actionType !== "create_card_expense") {
    return "Essa ação ainda não tem execução implementada.";
  }

  if (!db) {
    return "Banco de dados indisponível no momento.";
  }

  const parsed = cardExpensePayloadSchema.safeParse(action.payload);
  if (!parsed.success) {
    return "A ação pendente está inválida. Cancele e envie o lançamento novamente.";
  }

  const payload = parsed.data;
  const [card, month] = await Promise.all([
    getCardById(action.userId, payload.cardId),
    getCurrentMonthForUser(action.userId),
  ]);

  if (!card) return "Cartão não encontrado. Cancele e envie o lançamento novamente.";
  if (!month) return "Crie o mês atual no app antes de lançar compras pelo WhatsApp.";

  const installmentTotal =
    payload.paymentType === "installment" ? (payload.installments ?? 1) : 1;
  const targetMonths =
    installmentTotal > 1
      ? await ensureConsecutiveMonthsForUser(action.userId, month.month, month.year, installmentTotal)
      : [month];
  const baseAmount = Math.floor(payload.amountCents / installmentTotal);
  const remainder = payload.amountCents - baseAmount * installmentTotal;
  const installmentId = installmentTotal > 1 ? crypto.randomUUID() : null;

  await db.transaction(async (tx) => {
    await tx.insert(creditCardExpenses).values(
      targetMonths.map((targetMonth, index) => ({
        userId: action.userId,
        cardId: payload.cardId,
        monthId: targetMonth.id,
        description: payload.description,
        amountCents: baseAmount + (index < remainder ? 1 : 0),
        purchaseDate: payload.purchaseDate,
        installmentId,
        installmentNumber: installmentId ? index + 1 : null,
        installmentTotal: installmentId ? installmentTotal : null,
        whatsappPendingActionId: action.id,
      })),
    );

    for (const targetMonth of targetMonths) {
      await syncInvoiceTotal(tx, action.userId, payload.cardId, targetMonth, card.dueDay);
    }
  });

  await updateWhatsappPendingActionStatus(action.id, "confirmed");

  return [
    "Compra lançada.",
    `Cartão: ${card.name}`,
    `Valor: ${formatCurrency(payload.amountCents)}`,
    `Descrição: ${payload.description}`,
    installmentTotal > 1 ? `Parcelas: ${installmentTotal}x` : null,
  ]
    .filter(Boolean)
    .join("\n");
}

async function executeCreateBill(action: PendingAction) {
  if (!db) {
    return "Banco de dados indisponível no momento.";
  }

  const parsed = billPayloadSchema.safeParse(action.payload);
  if (!parsed.success) {
    return "A ação pendente está inválida. Cancele e envie o lançamento novamente.";
  }

  const payload = parsed.data;
  const month = await getCurrentMonthForUser(action.userId);

  if (!month) {
    return "Crie o mês atual no app antes de lançar despesas avulsas pelo WhatsApp.";
  }

  // Sem dia de vencimento, a conta cai no fim do mês para não nascer vencida.
  const dueDay = payload.dueDay ?? 31;

  // Recorrência real: cria a regra em recurrence_rules e materializa os
  // próximos meses como contas vinculadas. Exige dueDay porque a regra precisa
  // de um dia fixo; sem ele, mantemos o comportamento antigo (flag only).
  if (payload.isRecurring && payload.dueDay) {
    const [rule] = await db
      .insert(recurrenceRules)
      .values({
        userId: action.userId,
        name: payload.description,
        defaultAmountCents: payload.amountCents,
        dueDay: payload.dueDay,
        isVariableAmount: false,
        isActive: true,
      })
      .returning();

    if (!rule) {
      return "Não consegui criar a regra de recorrência agora.";
    }

    const targetMonths = await ensureConsecutiveMonthsForUser(
      action.userId,
      month.month,
      month.year,
      RECURRING_PREGENERATE_MONTHS,
    );

    const recurringDueDay = payload.dueDay;
    await db.insert(bills).values(
      targetMonths.map((targetMonth) => ({
        userId: action.userId,
        monthId: targetMonth.id,
        recurrenceRuleId: rule.id,
        name: payload.description,
        amountCents: payload.amountCents,
        dueDate: composeMonthDate(targetMonth.year, targetMonth.month, recurringDueDay),
        isRecurring: true,
        status: "pending" as const,
        whatsappPendingActionId: action.id,
      })),
    );

    await updateWhatsappPendingActionStatus(action.id, "confirmed");

    return [
      "Despesa recorrente lançada.",
      `Valor: ${formatCurrency(payload.amountCents)}`,
      `Descrição: ${payload.description}`,
      `Vencimento: dia ${payload.dueDay}`,
      `Próximos ${RECURRING_PREGENERATE_MONTHS} meses criados.`,
    ]
      .filter(Boolean)
      .join("\n");
  }

  const dueDate = composeMonthDate(month.year, month.month, dueDay);

  await db.insert(bills).values({
    userId: action.userId,
    monthId: month.id,
    name: payload.description,
    amountCents: payload.amountCents,
    dueDate,
    isRecurring: payload.isRecurring,
    status: "pending",
    whatsappPendingActionId: action.id,
  });

  await updateWhatsappPendingActionStatus(action.id, "confirmed");

  return [
    "Despesa lançada.",
    `Valor: ${formatCurrency(payload.amountCents)}`,
    `Descrição: ${payload.description}`,
    `Vencimento: ${dueDate.split("-").reverse().join("/")}`,
    payload.isRecurring ? "Recorrente: sim (apenas este mês)" : null,
  ]
    .filter(Boolean)
    .join("\n");
}

async function executeMarkBillPaid(action: PendingAction) {
  if (!db) {
    return "Banco de dados indisponível no momento.";
  }

  const parsed = markPaidPayloadSchema.safeParse(action.payload);
  if (!parsed.success) {
    return "A ação pendente está inválida. Cancele e envie novamente.";
  }

  const payload = parsed.data;
  const month = await getCurrentMonthForUser(action.userId);
  if (!month) {
    return "Crie o mês atual no app antes de marcar contas pelo WhatsApp.";
  }

  // Busca contas do mês atual que casam com a descrição (começa com o termo),
  // ainda não pagas. Casamento por prefixo aceita "luz" -> "Conta de luz".
  const candidates = await db
    .select({
      id: bills.id,
      name: bills.name,
      amountCents: bills.amountCents,
      dueDate: bills.dueDate,
      status: bills.status,
    })
    .from(bills)
    .where(
      and(
        eq(bills.userId, action.userId),
        eq(bills.monthId, month.id),
        sql`${bills.status} <> 'paid'`,
        sql`${bills.name} ILIKE ${"%" + payload.description + "%"}`,
      ),
    )
    .orderBy(bills.dueDate);

  if (candidates.length === 0) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return `Não encontrei conta pendente chamada "${payload.description}" no mês atual.`;
  }

  if (candidates.length > 1) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    const list = candidates
      .slice(0, 5)
      .map((c) => `• ${c.name} — ${formatCurrency(c.amountCents)} — ${c.dueDate}`)
      .join("\n");
    return `Encontrei mais de uma conta com "${payload.description}". Seja mais específico:\n\n${list}`;
  }

  const target = candidates[0];
  await db
    .update(bills)
    .set({ status: "paid", paidAt: new Date(), updatedAt: new Date() })
    .where(and(eq(bills.id, target.id), eq(bills.userId, action.userId)));

  await updateWhatsappPendingActionStatus(action.id, "confirmed");

  return [
    "Conta marcada como paga.",
    `Conta: ${target.name}`,
    `Valor: ${formatCurrency(target.amountCents)}`,
    `Vencimento: ${target.dueDate.split("-").reverse().join("/")}`,
  ].join("\n");
}

async function executeMarkInvoicePaid(action: PendingAction) {
  if (!db) {
    return "Banco de dados indisponível no momento.";
  }

  const parsed = markInvoicePaidPayloadSchema.safeParse(action.payload);
  if (!parsed.success) {
    return "A ação pendente está inválida. Cancele e envie novamente.";
  }

  const payload = parsed.data;
  const month = await getCurrentMonthForUser(action.userId);
  if (!month) {
    return "Crie o mês atual no app antes de marcar faturas pelo WhatsApp.";
  }

  const [invoice] = await db
    .select()
    .from(creditCardInvoices)
    .where(
      and(
        eq(creditCardInvoices.userId, action.userId),
        eq(creditCardInvoices.cardId, payload.cardId),
        eq(creditCardInvoices.monthId, month.id),
      ),
    )
    .limit(1);

  if (!invoice) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return `Não encontrei fatura de "${payload.cardName}" no mês atual.`;
  }

  if (invoice.status === "paid") {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return `A fatura de "${payload.cardName}" já está marcada como paga.`;
  }

  await db
    .update(creditCardInvoices)
    .set({ status: "paid", paidAt: new Date(), updatedAt: new Date() })
    .where(eq(creditCardInvoices.id, invoice.id));

  await updateWhatsappPendingActionStatus(action.id, "confirmed");

  return [
    "Fatura marcada como paga.",
    `Cartão: ${payload.cardName}`,
    `Valor: ${formatCurrency(invoice.amountCents)}`,
  ].join("\n");
}

async function executeCancelLastAction(action: PendingAction) {
  if (!db) {
    return "Banco de dados indisponível no momento.";
  }

  const parsed = cancelLastPayloadSchema.safeParse(action.payload ?? {});
  if (!parsed.success) {
    return "A ação pendente está inválida. Cancele e envie novamente.";
  }

  // Localiza a última pendência confirmada pelo WhatsApp, do mesmo usuário e
  // telefone, que tenha sido uma ação de escrita (não resolve_card_expense).
  const [lastConfirmed] = await db
    .select()
    .from(whatsappPendingActions)
    .where(
      and(
        eq(whatsappPendingActions.userId, action.userId),
        eq(whatsappPendingActions.phone, action.phone),
        eq(whatsappPendingActions.status, "confirmed"),
        sql`${whatsappPendingActions.actionType} IN ('create_card_expense', 'create_bill', 'mark_bill_paid', 'mark_invoice_paid')`,
      ),
    )
    .orderBy(desc(whatsappPendingActions.confirmedAt))
    .limit(1);

  if (!lastConfirmed) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return "Não encontrei nenhum lançamento recente para desfazer.";
  }

  // Evita desfazer duas vezes a mesma ação. Se já existe um cancel_last_action
  // confirmado apontando para essa mesma pendência, bloqueia.
  const [alreadyReverted] = await db
    .select({ id: whatsappPendingActions.id })
    .from(whatsappPendingActions)
    .where(
      and(
        eq(whatsappPendingActions.userId, action.userId),
        eq(whatsappPendingActions.status, "confirmed"),
        sql`${whatsappPendingActions.payload}->>'revertedActionId' = ${lastConfirmed.id}`,
      ),
    )
    .limit(1);

  if (alreadyReverted) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return "Esse lançamento já foi desfeito antes.";
  }

  const revertResult = await revertConfirmedAction(lastConfirmed);
  if (!revertResult.ok) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return revertResult.message;
  }

  await db
    .update(whatsappPendingActions)
    .set({
      status: "cancelled",
      cancelledAt: new Date(),
      updatedAt: new Date(),
      payload: sql`jsonb_set(${whatsappPendingActions.payload}, '{revertedBy}', to_jsonb(${action.id}::text))`,
    })
    .where(eq(whatsappPendingActions.id, lastConfirmed.id));

  await updateWhatsappPendingActionStatus(action.id, "confirmed");

  return [
    "Lançamento desfeito.",
    `Ação revertida: ${lastConfirmed.actionType}`,
    revertResult.message,
  ]
    .filter(Boolean)
    .join("\n");
}

async function executeEditLastAction(action: PendingAction) {
  if (!db) {
    return "Banco de dados indisponível no momento.";
  }

  const parsed = editLastPayloadSchema.safeParse(action.payload ?? {});
  if (!parsed.success) {
    return "A ação pendente está inválida. Cancele e envie novamente.";
  }

  const edit = parsed.data;
  if (!edit.newAmountCents && !edit.newDescription) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return "Nada para editar. Diga o novo valor ou descrição.";
  }

  // Localiza a última pendência confirmada de escrita (mesma lógica do cancel).
  const [lastConfirmed] = await db
    .select()
    .from(whatsappPendingActions)
    .where(
      and(
        eq(whatsappPendingActions.userId, action.userId),
        eq(whatsappPendingActions.phone, action.phone),
        eq(whatsappPendingActions.status, "confirmed"),
        sql`${whatsappPendingActions.actionType} IN ('create_card_expense', 'create_bill')`,
      ),
    )
    .orderBy(desc(whatsappPendingActions.confirmedAt))
    .limit(1);

  if (!lastConfirmed) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return "Não encontrei nenhum lançamento recente para editar.";
  }

  // Evita editar a mesma pendência duas vezes.
  const [alreadyEdited] = await db
    .select({ id: whatsappPendingActions.id })
    .from(whatsappPendingActions)
    .where(
      and(
        eq(whatsappPendingActions.userId, action.userId),
        eq(whatsappPendingActions.status, "confirmed"),
        sql`${whatsappPendingActions.payload}->>'editedActionId' = ${lastConfirmed.id}`,
      ),
    )
    .limit(1);

  if (alreadyEdited) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return "Esse lançamento já foi editado antes.";
  }

  const editResult = await applyEditToConfirmedAction(lastConfirmed, edit);
  if (!editResult.ok) {
    await updateWhatsappPendingActionStatus(action.id, "cancelled");
    return editResult.message;
  }

  await db
    .update(whatsappPendingActions)
    .set({
      payload: sql`jsonb_set(${whatsappPendingActions.payload}, '{editedBy}', to_jsonb(${action.id}::text))`,
      updatedAt: new Date(),
    })
    .where(eq(whatsappPendingActions.id, lastConfirmed.id));

  await updateWhatsappPendingActionStatus(action.id, "confirmed");

  return [
    "Lançamento editado.",
    editResult.message,
  ]
    .filter(Boolean)
    .join("\n");
}

async function applyEditToConfirmedAction(
  confirmed: { id: string; userId: string; actionType: string; payload: unknown },
  edit: { newAmountCents: number | null; newDescription: string | null },
): Promise<{ ok: true; message: string } | { ok: false; message: string }> {
  if (!db) return { ok: false, message: "Banco indisponível." };

  const updates: Record<string, unknown> = { updatedAt: new Date() };
  if (edit.newAmountCents) updates.amountCents = edit.newAmountCents;
  if (edit.newDescription) updates.description = edit.newDescription;

  if (confirmed.actionType === "create_card_expense") {
    const parsed = cardExpensePayloadSchema.safeParse(confirmed.payload);
    if (!parsed.success) return { ok: false, message: "Payload inválido." };

    // Para parcelado, redistribui o novo valor entre as parcelas existentes.
    if (edit.newAmountCents) {
      const [expense] = await db
        .select({
          installmentId: creditCardExpenses.installmentId,
          installmentTotal: creditCardExpenses.installmentTotal,
        })
        .from(creditCardExpenses)
        .where(
          and(
            eq(creditCardExpenses.userId, confirmed.userId),
            eq(creditCardExpenses.whatsappPendingActionId, confirmed.id),
          ),
        )
        .limit(1);

      if (expense?.installmentId && expense.installmentTotal) {
        const total = expense.installmentTotal;
        const base = Math.floor(edit.newAmountCents / total);
        const remainder = edit.newAmountCents - base * total;
        // Atualiza cada parcela individualmente para redistribuir o resto.
        const parcels = await db
          .select({ id: creditCardExpenses.id, installmentNumber: creditCardExpenses.installmentNumber })
          .from(creditCardExpenses)
          .where(
            and(
              eq(creditCardExpenses.userId, confirmed.userId),
              eq(creditCardExpenses.whatsappPendingActionId, confirmed.id),
            ),
          )
          .orderBy(creditCardExpenses.installmentNumber);

        for (const parcel of parcels) {
          const idx = (parcel.installmentNumber ?? 1) - 1;
          await db
            .update(creditCardExpenses)
            .set({
              amountCents: base + (idx < remainder ? 1 : 0),
              ...(edit.newDescription ? { description: edit.newDescription } : {}),
              updatedAt: new Date(),
            })
            .where(eq(creditCardExpenses.id, parcel.id));
        }

        // Re-sincroniza faturas.
        const card = await getCardById(confirmed.userId, parsed.data.cardId);
        if (card) {
          const affectedMonths = await db
            .select({ id: creditCardExpenses.monthId })
            .from(creditCardExpenses)
            .where(
              and(
                eq(creditCardExpenses.userId, confirmed.userId),
                eq(creditCardExpenses.whatsappPendingActionId, confirmed.id),
              ),
            )
            .groupBy(creditCardExpenses.monthId);

          for (const m of affectedMonths) {
            await db.transaction(async (tx) => {
              const [monthRow] = await tx
                .select()
                .from(months)
                .where(eq(months.id, m.id))
                .limit(1);
              if (monthRow) {
                await syncInvoiceTotal(tx, confirmed.userId, parsed.data.cardId, monthRow, card.dueDay);
              }
            });
          }
        }

        return { ok: true, message: `Compra atualizada: ${edit.newDescription ?? parsed.data.description} — ${formatCurrency(edit.newAmountCents)} em ${expense.installmentTotal}x.` };
      }
    }

    // Sem parcelamento: update direto.
    await db
      .update(creditCardExpenses)
      .set(updates)
      .where(
        and(
          eq(creditCardExpenses.userId, confirmed.userId),
          eq(creditCardExpenses.whatsappPendingActionId, confirmed.id),
        ),
      );

    // Re-sincroniza fatura do mês afetado.
    const card = await getCardById(confirmed.userId, parsed.data.cardId);
    if (card) {
      const affectedMonths = await db
        .select({ id: creditCardExpenses.monthId })
        .from(creditCardExpenses)
        .where(
          and(
            eq(creditCardExpenses.userId, confirmed.userId),
            eq(creditCardExpenses.whatsappPendingActionId, confirmed.id),
          ),
        )
        .groupBy(creditCardExpenses.monthId);

      for (const m of affectedMonths) {
        await db.transaction(async (tx) => {
          const [monthRow] = await tx
            .select()
            .from(months)
            .where(eq(months.id, m.id))
            .limit(1);
          if (monthRow) {
            await syncInvoiceTotal(tx, confirmed.userId, parsed.data.cardId, monthRow, card.dueDay);
          }
        });
      }
    }

    return {
      ok: true,
      message: `Compra atualizada: ${edit.newDescription ?? parsed.data.description}${edit.newAmountCents ? ` — ${formatCurrency(edit.newAmountCents)}` : ""}.`,
    };
  }

  if (confirmed.actionType === "create_bill") {
    const parsed = billPayloadSchema.safeParse(confirmed.payload);
    if (!parsed.success) return { ok: false, message: "Payload inválido." };

    const billUpdates: Record<string, unknown> = { updatedAt: new Date() };
    if (edit.newAmountCents) billUpdates.amountCents = edit.newAmountCents;
    if (edit.newDescription) billUpdates.name = edit.newDescription;

    await db
      .update(bills)
      .set(billUpdates)
      .where(
        and(
          eq(bills.userId, confirmed.userId),
          eq(bills.whatsappPendingActionId, confirmed.id),
        ),
      );

    return {
      ok: true,
      message: `Despesa atualizada: ${edit.newDescription ?? parsed.data.description}${edit.newAmountCents ? ` — ${formatCurrency(edit.newAmountCents)}` : ""}.`,
    };
  }

  return { ok: false, message: "Tipo de ação não suporta edição." };
}

async function revertConfirmedAction(
  confirmed: { id: string; userId: string; actionType: string; payload: unknown },
): Promise<{ ok: true; message: string } | { ok: false; message: string }> {
  if (!db) return { ok: false, message: "Banco indisponível." };

  if (confirmed.actionType === "create_card_expense") {
    const parsed = cardExpensePayloadSchema.safeParse(confirmed.payload);
    if (!parsed.success) return { ok: false, message: "Payload inválido." };

    const card = await getCardById(confirmed.userId, parsed.data.cardId);
    if (!card) return { ok: false, message: "Cartão não encontrado." };

    // Descobre os meses afetados antes de remover, para re-sincronizar faturas.
    const affectedMonths = await db
      .select({ id: creditCardExpenses.monthId })
      .from(creditCardExpenses)
      .where(
        and(
          eq(creditCardExpenses.userId, confirmed.userId),
          eq(creditCardExpenses.whatsappPendingActionId, confirmed.id),
        ),
      )
      .groupBy(creditCardExpenses.monthId);

    const monthIds = affectedMonths.map((m) => m.id);

    await db
      .delete(creditCardExpenses)
      .where(
        and(
          eq(creditCardExpenses.userId, confirmed.userId),
          eq(creditCardExpenses.whatsappPendingActionId, confirmed.id),
        ),
      );

    // Re-sincroniza as faturas dos meses afetados.
    for (const monthId of monthIds) {
      await db.transaction(async (tx) => {
        const [monthRow] = await tx
          .select()
          .from(months)
          .where(eq(months.id, monthId))
          .limit(1);
        if (monthRow) {
          await syncInvoiceTotal(tx, confirmed.userId, parsed.data.cardId, monthRow, card.dueDay);
        }
      });
    }

    return { ok: true, message: `Compra de "${parsed.data.description}" removida.` };
  }

  if (confirmed.actionType === "create_bill") {
    const parsed = billPayloadSchema.safeParse(confirmed.payload);
    if (!parsed.success) return { ok: false, message: "Payload inválido." };

    await db
      .delete(bills)
      .where(
        and(
          eq(bills.userId, confirmed.userId),
          eq(bills.whatsappPendingActionId, confirmed.id),
        ),
      );

    return { ok: true, message: `Despesa "${parsed.data.description}" removida.` };
  }

  if (confirmed.actionType === "mark_bill_paid") {
    // Reabre a conta que foi marcada como paga. O mark_bill_paid não grava
    // whatsappPendingActionId na bill (ela já existia antes), então revertemos
    // pelo payload da pendência: procuramos a conta paga que casa com a
    // descrição e reabrimos.
    const parsed = markPaidPayloadSchema.safeParse(confirmed.payload);
    if (!parsed.success) return { ok: false, message: "Payload inválido." };

    const [updated] = await db
      .update(bills)
      .set({ status: "pending", paidAt: null, updatedAt: new Date() })
      .where(
        and(
          eq(bills.userId, confirmed.userId),
          sql`${bills.name} ILIKE ${"%" + parsed.data.description + "%"}`,
          eq(bills.status, "paid"),
        ),
      )
      .returning({ id: bills.id });

    return updated
      ? { ok: true, message: `Conta "${parsed.data.description}" reaberta.` }
      : { ok: false, message: "Não encontrei a conta marcada como paga." };
  }

  if (confirmed.actionType === "mark_invoice_paid") {
    const parsed = markInvoicePaidPayloadSchema.safeParse(confirmed.payload);
    if (!parsed.success) return { ok: false, message: "Payload inválido." };

    const [updated] = await db
      .update(creditCardInvoices)
      .set({ status: "pending", paidAt: null, updatedAt: new Date() })
      .where(
        and(
          eq(creditCardInvoices.userId, confirmed.userId),
          eq(creditCardInvoices.cardId, parsed.data.cardId),
          eq(creditCardInvoices.status, "paid"),
        ),
      )
      .returning({ id: creditCardInvoices.id });

    return updated
      ? { ok: true, message: `Fatura de "${parsed.data.cardName}" reaberta.` }
      : { ok: false, message: "Não encontrei a fatura marcada como paga." };
  }

  return { ok: false, message: "Tipo de ação não suporta reversão." };
}
