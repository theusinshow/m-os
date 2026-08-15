"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { incomes } from "@/db/schema";
import { requireUser } from "@/lib/auth/guard";
import { db } from "@/db/client";
import { incomeSchema } from "@/lib/validators/income";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser } from "@/lib/active-month";
import { parseCurrencyToCents } from "@/lib/money";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

export async function createIncome(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const currentMonth = await getActiveMonthForUser(appUser.id);

  if (!currentMonth) {
    return errorState("Crie o mês atual antes de cadastrar receita.");
  }

  const parsed = incomeSchema.safeParse({
    name: formData.get("name"),
    amountCents: parseCurrencyToCents(formData.get("amount")),
    incomeType: formData.get("incomeType"),
    expectedDate: String(formData.get("expectedDate") ?? "") || undefined,
    received: formData.get("received") === "on",
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount" }),
    );
  }

  const payload = parsed.data;

  await db.insert(incomes).values({
    userId: appUser.id,
    monthId: currentMonth.id,
    name: payload.name,
    amountCents: payload.amountCents,
    incomeType: payload.incomeType,
    expectedDate: payload.expectedDate ?? null,
    received: payload.received,
  });

  revalidatePath("/app/dashboard");
  return successState("Receita adicionada.");
}

export async function updateIncome(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const incomeId = String(formData.get("incomeId") ?? "");

  if (!db || !appUser || !incomeId) {
    return errorState("Não foi possível editar a receita.");
  }

  const parsed = incomeSchema.safeParse({
    name: formData.get("name"),
    amountCents: parseCurrencyToCents(formData.get("amount")),
    incomeType: formData.get("incomeType"),
    expectedDate: String(formData.get("expectedDate") ?? "") || undefined,
    received: formData.get("received") === "on",
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount" }),
    );
  }

  const payload = parsed.data;

  await db
    .update(incomes)
    .set({
      name: payload.name,
      amountCents: payload.amountCents,
      incomeType: payload.incomeType,
      expectedDate: payload.expectedDate ?? null,
      received: payload.received,
      updatedAt: new Date(),
    })
    .where(and(eq(incomes.id, incomeId), eq(incomes.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  return successState("Receita atualizada.");
}

export async function deleteIncome(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const incomeId = String(formData.get("incomeId") ?? "");

  if (!db || !appUser || !incomeId) {
    throw new Error("Não foi possível excluir a receita.");
  }

  await db.delete(incomes).where(and(eq(incomes.id, incomeId), eq(incomes.userId, appUser.id)));

  revalidatePath("/app/dashboard");
}
