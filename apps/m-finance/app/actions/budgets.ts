"use server";

import { revalidatePath } from "next/cache";
import { and, eq } from "drizzle-orm";
import { budgets } from "@/db/schema";
import { requireUser } from "@/lib/auth/guard";
import { db } from "@/db/client";
import { budgetSchema } from "@/lib/validators/budget";
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

  try {
    await db.insert(budgets).values({
      userId: appUser.id,
      monthId: currentMonth.id,
      budgetType: payload.budgetType,
      categoryId: payload.categoryId ?? null,
      cardId: payload.cardId ?? null,
      limitCents: payload.limitCents,
    });
  } catch {
    return errorState("Já existe um orçamento desse tipo para este mês.");
  }

  revalidatePath("/app/budgets");
  revalidatePath("/app/dashboard");
  return successState("Orçamento adicionado.");
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
