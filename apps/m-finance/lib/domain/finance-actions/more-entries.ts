import { and, eq, isNull } from "drizzle-orm";
import { db } from "@/db/client";
import {
  billCategories,
  budgets,
  creditCardInvoices,
  creditCards,
  goalContributions,
  goals,
  incomes,
  subscriptions,
} from "@/db/schema";
import { composeMonthDate } from "@/lib/due-date";
import { fail, ok, type DomainResult, type MonthRecord } from "@/lib/domain/finance-actions/result";
import { normalizeForMatch } from "@/lib/finance-intelligence/sanitize";

/**
 * O resto do que a tela do M-Finance faz, como serviço de domínio: o Hermes
 * (pela Action API) e a interface web chamam as MESMAS funções.
 *
 * As que tocam entidade existente aceitam o nome que o preview mostrou e
 * recusam (`stale_preview`) se ele não bater mais — mesma trava de
 * `markBillPaid`.
 */

function sameName(expected: string | undefined, actual: string) {
  return !expected || normalizeForMatch(expected) === normalizeForMatch(actual);
}

// ---------------------------------------------------------------- assinatura

export async function updateSubscriptionEntry(
  userId: string,
  subscriptionId: string,
  patch: {
    amountCents?: number;
    nextChargeDate?: string;
    cycle?: "once" | "monthly" | "yearly";
    reminderDaysBefore?: number;
  },
  expectation: { name?: string } = {},
): Promise<DomainResult<{ id: string; name: string }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const [row] = await db
    .select({ id: subscriptions.id, name: subscriptions.name, status: subscriptions.status })
    .from(subscriptions)
    .where(and(eq(subscriptions.id, subscriptionId), eq(subscriptions.userId, userId)))
    .limit(1);
  if (!row) return fail("not_found", "Assinatura não encontrada.");
  if (!sameName(expectation.name, row.name)) {
    return fail("stale_preview", `A assinatura mudou desde o preview ("${row.name}"). Peça de novo.`);
  }
  if (Object.values(patch).every((value) => value === undefined)) {
    return fail("invalid", "A edição não muda nada.");
  }
  await db
    .update(subscriptions)
    .set({
      ...(patch.amountCents !== undefined ? { amountCents: patch.amountCents } : {}),
      ...(patch.nextChargeDate !== undefined
        ? // Nova data de cobrança: o lembrete daquela data ainda não foi enviado.
          { nextChargeDate: patch.nextChargeDate, lastNotifiedFor: null }
        : {}),
      ...(patch.cycle !== undefined ? { cycle: patch.cycle } : {}),
      ...(patch.reminderDaysBefore !== undefined ? { reminderDaysBefore: patch.reminderDaysBefore } : {}),
      updatedAt: new Date(),
    })
    .where(and(eq(subscriptions.id, subscriptionId), eq(subscriptions.userId, userId)));
  return ok({ id: row.id, name: row.name });
}

/** Cancela, não apaga: o histórico continua e o custo some das projeções. */
export async function cancelSubscriptionEntry(
  userId: string,
  subscriptionId: string,
  expectation: { name?: string } = {},
): Promise<DomainResult<{ id: string; name: string }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const [row] = await db
    .select({ id: subscriptions.id, name: subscriptions.name, status: subscriptions.status })
    .from(subscriptions)
    .where(and(eq(subscriptions.id, subscriptionId), eq(subscriptions.userId, userId)))
    .limit(1);
  if (!row) return fail("not_found", "Assinatura não encontrada.");
  if (!sameName(expectation.name, row.name)) {
    return fail("stale_preview", `A assinatura mudou desde o preview ("${row.name}"). Peça de novo.`);
  }
  if (row.status === "canceled") return fail("already_paid", `"${row.name}" já está cancelada.`);
  await db
    .update(subscriptions)
    .set({ status: "canceled", updatedAt: new Date() })
    .where(and(eq(subscriptions.id, subscriptionId), eq(subscriptions.userId, userId)));
  return ok({ id: row.id, name: row.name });
}

// ------------------------------------------------------------------- receita

/**
 * Marca uma receita como recebida. É o que muda a renda "a receber" para
 * renda de fato — e, com isso, o peso dela no Safe-to-Spend.
 */
export async function markIncomeReceived(
  userId: string,
  incomeId: string,
  expectation: { amountCents?: number; name?: string } = {},
): Promise<DomainResult<{ id: string; name: string; amountCents: number }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const [row] = await db
    .select({ id: incomes.id, name: incomes.name, amountCents: incomes.amountCents, received: incomes.received })
    .from(incomes)
    .where(and(eq(incomes.id, incomeId), eq(incomes.userId, userId)))
    .limit(1);
  if (!row) return fail("not_found", "Receita não encontrada.");
  if (!sameName(expectation.name, row.name)) {
    return fail("stale_preview", `A receita mudou desde o preview ("${row.name}"). Peça de novo.`);
  }
  if (expectation.amountCents != null && expectation.amountCents !== row.amountCents) {
    return fail("stale_preview", `O valor de "${row.name}" mudou desde o preview. Peça de novo.`);
  }
  if (row.received) return fail("already_paid", `"${row.name}" já está marcada como recebida.`);
  await db
    .update(incomes)
    .set({ received: true, updatedAt: new Date() })
    .where(and(eq(incomes.id, incomeId), eq(incomes.userId, userId)));
  return ok({ id: row.id, name: row.name, amountCents: row.amountCents });
}

// ---------------------------------------------------------------------- meta

/**
 * Guarda dinheiro numa meta. O progresso é limitado ao alvo, e só a meta
 * ATIVA conclui sozinha — uma meta pausada que chega ao alvo continua pausada,
 * porque a pausa foi uma escolha.
 */
export async function addGoalContribution(
  userId: string,
  goalId: string,
  amountCents: number,
  contributionDate: string,
  expectation: { name?: string } = {},
): Promise<DomainResult<{ id: string; name: string; status: string; currentAmountCents: number }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const database = db;
  const [goal] = await database
    .select({
      name: goals.name,
      currentAmountCents: goals.currentAmountCents,
      targetAmountCents: goals.targetAmountCents,
      status: goals.status,
    })
    .from(goals)
    .where(and(eq(goals.id, goalId), eq(goals.userId, userId)))
    .limit(1);
  if (!goal) return fail("not_found", "Meta não encontrada.");
  if (!sameName(expectation.name, goal.name)) {
    return fail("stale_preview", `A meta mudou desde o preview ("${goal.name}"). Peça de novo.`);
  }
  if (goal.status === "archived") return fail("invalid", `"${goal.name}" está arquivada.`);

  const reached = goal.currentAmountCents + amountCents >= goal.targetAmountCents;
  const currentAmountCents = Math.min(goal.currentAmountCents + amountCents, goal.targetAmountCents);
  const status = reached && goal.status === "active" ? "completed" : goal.status;

  await database.transaction(async (tx) => {
    await tx.insert(goalContributions).values({ userId, goalId, amountCents, contributionDate });
    await tx
      .update(goals)
      .set({ currentAmountCents, status, updatedAt: new Date() })
      .where(and(eq(goals.id, goalId), eq(goals.userId, userId)));
  });
  return ok({ id: goalId, name: goal.name, status, currentAmountCents });
}

export const GOAL_STATUSES = ["active", "paused", "completed", "archived"] as const;
export type GoalStatusValue = (typeof GOAL_STATUSES)[number];

export async function setGoalStatusEntry(
  userId: string,
  goalId: string,
  status: GoalStatusValue,
  expectation: { name?: string } = {},
): Promise<DomainResult<{ id: string; name: string; status: GoalStatusValue }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const [goal] = await db
    .select({ name: goals.name })
    .from(goals)
    .where(and(eq(goals.id, goalId), eq(goals.userId, userId)))
    .limit(1);
  if (!goal) return fail("not_found", "Meta não encontrada.");
  if (!sameName(expectation.name, goal.name)) {
    return fail("stale_preview", `A meta mudou desde o preview ("${goal.name}"). Peça de novo.`);
  }
  await db
    .update(goals)
    .set({ status, updatedAt: new Date() })
    .where(and(eq(goals.id, goalId), eq(goals.userId, userId)));
  return ok({ id: goalId, name: goal.name, status });
}

// -------------------------------------------------------------------- fatura

/**
 * O valor REAL da fatura de um cartão num mês — o "confirmar valor" da tela de
 * cartões. Cria a fatura (que até aqui era estimada) ou corrige a lançada. O
 * vencimento sai do dia do cartão quando não vem outro.
 *
 * Fatura já paga não tem o valor trocado por aqui: mudar o total do que já foi
 * pago reescreveria o histórico.
 */
export async function upsertInvoiceAmount(input: {
  userId: string;
  cardId: string;
  month: MonthRecord;
  amountCents: number;
  dueDay?: number | null;
  notes?: string | null;
  expectation?: { cardName?: string };
}): Promise<DomainResult<{ cardName: string; amountCents: number; dueDate: string; created: boolean }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const [card] = await db
    .select({ name: creditCards.name, dueDay: creditCards.dueDay })
    .from(creditCards)
    .where(and(eq(creditCards.id, input.cardId), eq(creditCards.userId, input.userId)))
    .limit(1);
  if (!card) return fail("not_found", "Cartão não encontrado.");
  if (!sameName(input.expectation?.cardName, card.name)) {
    return fail("stale_preview", `O cartão mudou desde o preview ("${card.name}"). Peça de novo.`);
  }

  const [existing] = await db
    .select({ id: creditCardInvoices.id, status: creditCardInvoices.status })
    .from(creditCardInvoices)
    .where(
      and(
        eq(creditCardInvoices.userId, input.userId),
        eq(creditCardInvoices.cardId, input.cardId),
        eq(creditCardInvoices.monthId, input.month.id),
      ),
    )
    .limit(1);
  if (existing?.status === "paid") {
    return fail("already_paid", `A fatura ${card.name} deste mês já está paga; o valor não muda por aqui.`);
  }

  const dueDate = composeMonthDate(input.month.year, input.month.month, input.dueDay ?? card.dueDay);
  await db
    .insert(creditCardInvoices)
    .values({
      userId: input.userId,
      monthId: input.month.id,
      cardId: input.cardId,
      amountCents: input.amountCents,
      dueDate,
      status: "pending",
      notes: input.notes ?? null,
    })
    .onConflictDoUpdate({
      target: [creditCardInvoices.cardId, creditCardInvoices.monthId],
      set: { amountCents: input.amountCents, dueDate, status: "pending", notes: input.notes ?? null, updatedAt: new Date() },
    });
  return ok({ cardName: card.name, amountCents: input.amountCents, dueDate, created: !existing });
}

// ---------------------------------------------------------------- orçamento

/**
 * Cria ou ajusta o orçamento de um mês. Um por tipo e referência: "o total de
 * setembro", "Mercado em setembro", "o Nubank em setembro". Existindo, muda o
 * limite; não existindo, cria. A categoria vem por id ou pelo nome — o Hermes
 * fala o nome, e dois nomes iguais fazem a ação recusar em vez de escolher.
 */
export async function setBudgetEntry(input: {
  userId: string;
  month: MonthRecord;
  budgetType: "total" | "category" | "card";
  limitCents: number;
  categoryId?: string | null;
  categoryName?: string | null;
  cardId?: string | null;
}): Promise<DomainResult<{ id: string; label: string; created: boolean }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");

  let categoryId: string | null = null;
  let label = "Total do mês";
  if (input.budgetType === "category") {
    const rows = await db
      .select({ id: billCategories.id, name: billCategories.name })
      .from(billCategories)
      .where(and(eq(billCategories.userId, input.userId), eq(billCategories.isArchived, false)));
    const match = input.categoryId
      ? rows.filter((row) => row.id === input.categoryId)
      : rows.filter((row) => normalizeForMatch(row.name) === normalizeForMatch(input.categoryName ?? ""));
    if (match.length === 0) return fail("not_found", "Categoria não encontrada.");
    if (match.length > 1) return fail("invalid", "Mais de uma categoria com esse nome; use o id.");
    categoryId = match[0].id;
    label = match[0].name;
  }
  let cardId: string | null = null;
  if (input.budgetType === "card") {
    const [card] = input.cardId
      ? await db
          .select({ id: creditCards.id, name: creditCards.name })
          .from(creditCards)
          .where(and(eq(creditCards.id, input.cardId), eq(creditCards.userId, input.userId)))
          .limit(1)
      : [];
    if (!card) return fail("not_found", "Cartão não encontrado.");
    cardId = card.id;
    label = card.name;
  }

  const [existing] = await db
    .select({ id: budgets.id })
    .from(budgets)
    .where(
      and(
        eq(budgets.userId, input.userId),
        eq(budgets.monthId, input.month.id),
        eq(budgets.budgetType, input.budgetType),
        categoryId ? eq(budgets.categoryId, categoryId) : isNull(budgets.categoryId),
        cardId ? eq(budgets.cardId, cardId) : isNull(budgets.cardId),
      ),
    )
    .limit(1);

  if (existing) {
    await db
      .update(budgets)
      .set({ limitCents: input.limitCents, updatedAt: new Date() })
      .where(eq(budgets.id, existing.id));
    return ok({ id: existing.id, label, created: false });
  }

  const [created] = await db
    .insert(budgets)
    .values({
      userId: input.userId,
      monthId: input.month.id,
      budgetType: input.budgetType,
      categoryId,
      cardId,
      limitCents: input.limitCents,
    })
    .returning({ id: budgets.id });
  return created ? ok({ id: created.id, label, created: true }) : fail("write_failed", "Não consegui gravar o orçamento.");
}
