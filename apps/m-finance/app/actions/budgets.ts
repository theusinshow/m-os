"use server";

import { revalidatePath } from "next/cache";
import { and, eq } from "drizzle-orm";
import { budgets } from "@/db/schema";
import { requireUser } from "@/lib/auth/guard";
import { db } from "@/db/client";
import { budgetSchema } from "@/lib/validators/budget";
import { setBudgetEntry } from "@/lib/domain/finance-actions/more-entries";
import { parseCurrencyToCents } from "@/lib/money";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser } from "@/lib/active-month";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

export async function createBudget(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const currentMonth = await getActiveMonthForUser(appUser.id);
  if (!currentMonth) {
    return errorState("Crie o mês atual antes de cadastrar orçamento.");
  }

  const budgetType = String(formData.get("budgetType") ?? "total");
  const categoryId = String(formData.get("categoryId") ?? "");
  const cardId = String(formData.get("cardId") ?? "");

  const parsed = budgetSchema.safeParse({
    budgetType,
    limitCents: parseCurrencyToCents(formData.get("limit")),
    categoryId: categoryId || undefined,
    cardId: cardId || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { limitCents: "limit" }),
    );
  }

  const payload = parsed.data;

  // Mesmo serviço do Hermes (`m-finance.set_budget`). Existindo orçamento
  // desse tipo no mês, o limite é ajustado em vez de recusar a gravação.
  const result = await setBudgetEntry({
    userId: appUser.id,
    month: currentMonth,
    budgetType: payload.budgetType,
    limitCents: payload.limitCents,
    categoryId: payload.categoryId ?? null,
    cardId: payload.cardId ?? null,
  });
  if (!result.ok) {
    return errorState(result.message);
  }

  revalidatePath("/app/budgets");
  revalidatePath("/app/dashboard");
  return successState(result.value.created ? "Orçamento adicionado." : "Orçamento já existia; limite ajustado.");
}

export async function updateBudget(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const budgetId = String(formData.get("budgetId") ?? "");

  if (!db || !appUser || !budgetId) {
    return errorState("Não foi possível editar o orçamento.");
  }

  const parsed = budgetSchema.safeParse({
    budgetType: String(formData.get("budgetType") ?? "total"),
    limitCents: parseCurrencyToCents(formData.get("limit")),
    categoryId: String(formData.get("categoryId") ?? "") || undefined,
    cardId: String(formData.get("cardId") ?? "") || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { limitCents: "limit" }),
    );
  }

  const payload = parsed.data;

  await db
    .update(budgets)
    .set({
      budgetType: payload.budgetType,
      categoryId: payload.categoryId ?? null,
      cardId: payload.cardId ?? null,
      limitCents: payload.limitCents,
      updatedAt: new Date(),
    })
    .where(and(eq(budgets.id, budgetId), eq(budgets.userId, appUser.id)));

  revalidatePath("/app/budgets");
  revalidatePath("/app/dashboard");
  return successState("Orçamento atualizado.");
}

export async function deleteBudget(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const budgetId = String(formData.get("budgetId") ?? "");

  if (!db || !appUser || !budgetId) {
    throw new Error("Não foi possível excluir o orçamento.");
  }

  await db
    .delete(budgets)
    .where(and(eq(budgets.id, budgetId), eq(budgets.userId, appUser.id)));

  revalidatePath("/app/budgets");
  revalidatePath("/app/dashboard");
}
