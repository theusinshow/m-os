import { and, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { financialPolicies, goals, incomes, subscriptions } from "@/db/schema";
import { fail, ok, type DomainResult, type MonthRecord } from "@/lib/domain/finance-actions/result";
import { parsePolicyValue, type PolicyKey } from "@/lib/finance-intelligence/policies";

// ------------------------------------------------------------------ receita

export type CreateIncomeInput = {
  userId: string;
  month: MonthRecord;
  name: string;
  amountCents: number;
  incomeType: "main" | "extra" | "freelance";
  expectedDate: string | null;
  received: boolean;
  notes?: string | null;
};

/** Lança uma receita no mês em que ela ENTRA — não no mês em que foi digitada. */
export async function createIncomeEntry(input: CreateIncomeInput): Promise<DomainResult<{ id: string }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const [created] = await db
    .insert(incomes)
    .values({
      userId: input.userId,
      monthId: input.month.id,
      name: input.name,
      amountCents: input.amountCents,
      incomeType: input.incomeType,
      expectedDate: input.expectedDate,
      received: input.received,
      ...(input.notes ? { notes: input.notes } : {}),
    })
    .returning({ id: incomes.id });
  return created ? ok({ id: created.id }) : fail("write_failed", "Não consegui gravar a receita agora.");
}

// --------------------------------------------------------------- assinatura

export type CreateSubscriptionInput = {
  userId: string;
  name: string;
  amountCents: number;
  nextChargeDate: string;
  cycle: "once" | "monthly" | "yearly";
  isTrial: boolean;
  reminderDaysBefore: number;
};

export async function createSubscriptionEntry(
  input: CreateSubscriptionInput,
): Promise<DomainResult<{ id: string }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const [created] = await db
    .insert(subscriptions)
    .values({
      userId: input.userId,
      name: input.name,
      amountCents: input.amountCents,
      nextChargeDate: input.nextChargeDate,
      cycle: input.cycle,
      status: input.isTrial ? "trial" : "active",
      reminderDaysBefore: input.reminderDaysBefore,
    })
    .returning({ id: subscriptions.id });
  return created ? ok({ id: created.id }) : fail("write_failed", "Não consegui gravar a assinatura agora.");
}

// --------------------------------------------------------------------- meta

export type GoalFields = {
  name: string;
  targetAmountCents: number;
  currentAmountCents: number;
  deadline: string | null;
  priority: "low" | "medium" | "high";
  notes?: string | null;
};

/**
 * Meta acompanha progresso até o alvo: o valor guardado é limitado ao alvo e
 * ela nasce concluída quando já chegou lá.
 */
export function normalizeGoalAmounts(fields: Pick<GoalFields, "targetAmountCents" | "currentAmountCents">) {
  return {
    reached: fields.currentAmountCents >= fields.targetAmountCents,
    currentAmountCents: Math.min(fields.currentAmountCents, fields.targetAmountCents),
  };
}

export async function createGoalEntry(userId: string, fields: GoalFields): Promise<DomainResult<{ id: string }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const { reached, currentAmountCents } = normalizeGoalAmounts(fields);
  const [created] = await db
    .insert(goals)
    .values({
      userId,
      name: fields.name,
      targetAmountCents: fields.targetAmountCents,
      currentAmountCents,
      deadline: fields.deadline,
      priority: fields.priority,
      status: reached ? "completed" : "active",
      notes: fields.notes ?? null,
    })
    .returning({ id: goals.id });
  return created ? ok({ id: created.id }) : fail("write_failed", "Não consegui gravar a meta agora.");
}

/**
 * Edita uma meta. Editar não despausa nem desarquiva: status muda por gesto
 * próprio. Só o par ativo↔concluída acompanha o valor guardado.
 *
 * `patch` aceita edição parcial (o Hermes muda um campo por vez); campo
 * ausente fica como está.
 */
export async function updateGoalEntry(
  userId: string,
  goalId: string,
  patch: Partial<GoalFields>,
  expectation: { name?: string } = {},
): Promise<DomainResult<{ id: string; name: string; status: string }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");

  const [existing] = await db
    .select()
    .from(goals)
    .where(and(eq(goals.id, goalId), eq(goals.userId, userId)))
    .limit(1);
  if (!existing) return fail("not_found", "Meta não encontrada.");
  if (expectation.name && expectation.name.trim() !== existing.name.trim()) {
    return fail("stale_preview", `A meta mudou de nome desde o preview ("${existing.name}"). Peça de novo.`);
  }

  const merged: GoalFields = {
    name: patch.name ?? existing.name,
    targetAmountCents: patch.targetAmountCents ?? existing.targetAmountCents,
    currentAmountCents: patch.currentAmountCents ?? existing.currentAmountCents,
    deadline: patch.deadline !== undefined ? patch.deadline : existing.deadline,
    priority: patch.priority ?? existing.priority,
    notes: patch.notes !== undefined ? patch.notes : existing.notes,
  };
  const { reached, currentAmountCents } = normalizeGoalAmounts(merged);
  const status =
    existing.status === "active" || existing.status === "completed"
      ? reached
        ? "completed"
        : "active"
      : existing.status;

  await db
    .update(goals)
    .set({
      name: merged.name,
      targetAmountCents: merged.targetAmountCents,
      currentAmountCents,
      deadline: merged.deadline,
      priority: merged.priority,
      status,
      notes: merged.notes ?? null,
      updatedAt: new Date(),
    })
    .where(and(eq(goals.id, goalId), eq(goals.userId, userId)));

  return ok({ id: goalId, name: merged.name, status });
}

// ------------------------------------------------------------------ política

/**
 * Grava uma política. O valor passa pelo schema da chave AQUI, no servidor —
 * o preview do M/OS também valida, mas quem decide o que entra no banco é o
 * M-Finance.
 */
export async function setFinancialPolicy(
  userId: string,
  key: PolicyKey,
  value: unknown,
  source: "user" | "hermes" | "whatsapp",
): Promise<DomainResult<{ key: PolicyKey; value: unknown }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  const parsed = parsePolicyValue(key, value);
  if (!parsed.success) return fail("invalid", `Valor inválido para ${key}.`);

  await db
    .insert(financialPolicies)
    .values({ userId, key, value: parsed.data, source, active: true })
    .onConflictDoUpdate({
      target: [financialPolicies.userId, financialPolicies.key],
      set: { value: parsed.data, source, active: true, updatedAt: new Date() },
    });
  return ok({ key, value: parsed.data });
}

/** Volta ao padrão: desativa, sem apagar o histórico da escolha. */
export async function resetFinancialPolicy(userId: string, key: PolicyKey): Promise<DomainResult<{ key: PolicyKey }>> {
  if (!db) return fail("db_unavailable", "Banco de dados indisponível no momento.");
  await db
    .update(financialPolicies)
    .set({ active: false, updatedAt: new Date() })
    .where(and(eq(financialPolicies.userId, userId), eq(financialPolicies.key, key)));
  return ok({ key });
}
