"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { creditCardExpenses } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser } from "@/lib/active-month";
import { getCardById } from "@/lib/card-expenses";
import { parseCurrencyToCents } from "@/lib/money";
import { syncInvoiceTotal } from "@/lib/invoice-sync";
import { cardExpenseSchema } from "@/lib/validators/card-expense";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

function revalidateCardSurfaces(cardId: string) {
  revalidatePath(`/app/cards/${cardId}`);
  revalidatePath("/app/cards");
  revalidatePath("/app/dashboard");
  revalidatePath("/app/calendar");
}

export async function addCardExpense(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const cardId = String(formData.get("cardId") ?? "");

  if (!db || !appUser || !cardId) {
    return errorState("Não foi possível registrar a compra.");
  }

  const month = await getActiveMonthForUser(appUser.id);
  if (!month) {
    return errorState("Crie o mês atual antes de lançar compras.");
  }

  const card = await getCardById(appUser.id, cardId);
  if (!card) {
    return errorState("Cartão não encontrado.");
  }

  const parsed = cardExpenseSchema.safeParse({
    description: formData.get("description"),
    amountCents: parseCurrencyToCents(formData.get("amount")),
    purchaseDate: String(formData.get("purchaseDate") ?? "") || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount" }),
    );
  }

  const payload = parsed.data;

  await db.transaction(async (tx) => {
    await tx.insert(creditCardExpenses).values({
      userId: appUser.id,
      cardId,
      monthId: month.id,
      description: payload.description,
      amountCents: payload.amountCents,
      purchaseDate: payload.purchaseDate ?? null,
    });
    await syncInvoiceTotal(tx, appUser.id, cardId, month, card.dueDay);
  });

  revalidateCardSurfaces(cardId);
  return successState("Compra lançada.");
}

export async function deleteCardExpense(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const expenseId = String(formData.get("expenseId") ?? "");
  const cardId = String(formData.get("cardId") ?? "");

  if (!db || !appUser || !expenseId || !cardId) {
    throw new Error("Não foi possível excluir a compra.");
  }

  const card = await getCardById(appUser.id, cardId);
  const month = await getActiveMonthForUser(appUser.id);

  await db.transaction(async (tx) => {
    await tx
      .delete(creditCardExpenses)
      .where(
        and(eq(creditCardExpenses.id, expenseId), eq(creditCardExpenses.userId, appUser.id)),
      );
    if (card && month) {
      await syncInvoiceTotal(tx, appUser.id, cardId, month, card.dueDay);
    }
  });

  revalidateCardSurfaces(cardId);
}
