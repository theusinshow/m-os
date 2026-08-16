"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { creditCardExpenses, months } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { ensureConsecutiveMonthsForUser, getAppUserBySupabaseId } from "@/lib/months";
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
    paymentType: formData.get("paymentType") ?? "cash",
    installments: formData.get("installments")
      ? Number(formData.get("installments"))
      : undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount" }),
    );
  }

  const payload = parsed.data;
  const installmentTotal =
    payload.paymentType === "installment" ? (payload.installments ?? 1) : 1;
  const targetMonths =
    installmentTotal > 1
      ? await ensureConsecutiveMonthsForUser(
          appUser.id,
          month.month,
          month.year,
          installmentTotal,
        )
      : [month];
  const baseAmount = Math.floor(payload.amountCents / installmentTotal);
  const remainder = payload.amountCents - baseAmount * installmentTotal;
  const installmentId = installmentTotal > 1 ? crypto.randomUUID() : null;

  await db.transaction(async (tx) => {
    await tx.insert(creditCardExpenses).values(
      targetMonths.map((targetMonth, index) => ({
        userId: appUser.id,
        cardId,
        monthId: targetMonth.id,
        description: payload.description,
        amountCents: baseAmount + (index < remainder ? 1 : 0),
        purchaseDate: payload.purchaseDate ?? null,
        installmentId,
        installmentNumber: installmentId ? index + 1 : null,
        installmentTotal: installmentId ? installmentTotal : null,
      })),
    );

    for (const targetMonth of targetMonths) {
      await syncInvoiceTotal(tx, appUser.id, cardId, targetMonth, card.dueDay);
    }
  });

  revalidateCardSurfaces(cardId);
  return successState(
    installmentTotal > 1
      ? `Compra parcelada em ${installmentTotal} vezes.`
      : "Compra lançada.",
  );
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
  const [expenseMonth] = await db
    .select({ id: months.id, month: months.month, year: months.year })
    .from(creditCardExpenses)
    .innerJoin(months, eq(creditCardExpenses.monthId, months.id))
    .where(
      and(
        eq(creditCardExpenses.id, expenseId),
        eq(creditCardExpenses.userId, appUser.id),
      ),
    )
    .limit(1);

  await db.transaction(async (tx) => {
    await tx
      .delete(creditCardExpenses)
      .where(
        and(eq(creditCardExpenses.id, expenseId), eq(creditCardExpenses.userId, appUser.id)),
      );
    if (card && expenseMonth) {
      await syncInvoiceTotal(tx, appUser.id, cardId, expenseMonth, card.dueDay);
    }
  });

  revalidateCardSurfaces(cardId);
}

export async function deleteCardExpenseSeries(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const installmentId = String(formData.get("installmentId") ?? "");
  const cardId = String(formData.get("cardId") ?? "");

  if (!db || !appUser || !installmentId || !cardId) {
    throw new Error("Não foi possível excluir o parcelamento.");
  }

  const card = await getCardById(appUser.id, cardId);
  if (!card) {
    throw new Error("Cartão não encontrado.");
  }

  const affectedMonths = await db
    .select({ id: months.id, month: months.month, year: months.year })
    .from(creditCardExpenses)
    .innerJoin(months, eq(creditCardExpenses.monthId, months.id))
    .where(
      and(
        eq(creditCardExpenses.installmentId, installmentId),
        eq(creditCardExpenses.userId, appUser.id),
        eq(creditCardExpenses.cardId, cardId),
      ),
    );

  await db.transaction(async (tx) => {
    await tx
      .delete(creditCardExpenses)
      .where(
        and(
          eq(creditCardExpenses.installmentId, installmentId),
          eq(creditCardExpenses.userId, appUser.id),
          eq(creditCardExpenses.cardId, cardId),
        ),
      );

    for (const affectedMonth of affectedMonths) {
      await syncInvoiceTotal(tx, appUser.id, cardId, affectedMonth, card.dueDay);
    }
  });

  revalidateCardSurfaces(cardId);
}
