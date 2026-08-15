"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { creditCardInvoices, creditCards, months } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser } from "@/lib/active-month";
import { parseCurrencyToCents } from "@/lib/money";
import { composeMonthDate, parseDueDay } from "@/lib/due-date";
import { invoiceSchema } from "@/lib/validators/invoice";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

export async function createInvoice(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const currentMonth = await getActiveMonthForUser(appUser.id);

  if (!currentMonth) {
    return errorState("Crie o mês atual antes de cadastrar fatura.");
  }

  const parsed = invoiceSchema.safeParse({
    cardId: formData.get("cardId"),
    amountCents: parseCurrencyToCents(formData.get("amount")),
    dueDay: parseDueDay(formData.get("dueDay")),
    notes: formData.get("notes") || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount", cardId: "cardId" }),
    );
  }

  const payload = parsed.data;

  // The due date defaults to the card's own due day, so adding a monthly
  // invoice is just "pick card + value".
  const [card] = await db
    .select({ dueDay: creditCards.dueDay })
    .from(creditCards)
    .where(and(eq(creditCards.id, payload.cardId), eq(creditCards.userId, appUser.id)))
    .limit(1);

  if (!card) {
    return errorState("Cartão não encontrado.");
  }

  const dueDate = composeMonthDate(
    currentMonth.year,
    currentMonth.month,
    payload.dueDay ?? card.dueDay,
  );

  await db
    .insert(creditCardInvoices)
    .values({
      userId: appUser.id,
      monthId: currentMonth.id,
      cardId: payload.cardId,
      amountCents: payload.amountCents,
      dueDate,
      status: "pending",
      notes: payload.notes ?? null,
    })
    .onConflictDoUpdate({
      target: [creditCardInvoices.cardId, creditCardInvoices.monthId],
      set: {
        amountCents: payload.amountCents,
        dueDate,
        status: "pending",
        notes: payload.notes ?? null,
        updatedAt: new Date(),
      },
    });

  revalidatePath("/app/dashboard");
  revalidatePath("/app/cards");
  revalidatePath("/app/calendar");
  return successState("Fatura adicionada.");
}

export async function markInvoiceAsPaid(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const invoiceId = String(formData.get("invoiceId") ?? "");

  if (!db || !appUser || !invoiceId) {
    throw new Error("Não foi possível marcar a fatura como paga.");
  }

  await db
    .update(creditCardInvoices)
    .set({
      status: "paid",
      paidAt: new Date(),
      updatedAt: new Date(),
    })
    .where(and(eq(creditCardInvoices.id, invoiceId), eq(creditCardInvoices.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/cards");
  revalidatePath("/app/calendar");
}

export async function markInvoiceAsPending(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const invoiceId = String(formData.get("invoiceId") ?? "");

  if (!db || !appUser || !invoiceId) {
    throw new Error("Não foi possível reabrir a fatura.");
  }

  await db
    .update(creditCardInvoices)
    .set({
      status: "pending",
      paidAt: null,
      updatedAt: new Date(),
    })
    .where(and(eq(creditCardInvoices.id, invoiceId), eq(creditCardInvoices.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/cards");
  revalidatePath("/app/calendar");
}

export async function updateInvoice(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const invoiceId = String(formData.get("invoiceId") ?? "");
  const amountCents = parseCurrencyToCents(formData.get("amount"));
  const dueDay = parseDueDay(formData.get("dueDay"));

  if (!db || !appUser || !invoiceId) {
    return errorState("Não foi possível editar a fatura.");
  }

  if (amountCents <= 0) {
    return errorState("Revise os campos destacados.", {
      amount: "Informe um valor maior que zero.",
    });
  }

  const [invoiceMonth] = await db
    .select({ month: months.month, year: months.year })
    .from(creditCardInvoices)
    .innerJoin(months, eq(creditCardInvoices.monthId, months.id))
    .where(and(eq(creditCardInvoices.id, invoiceId), eq(creditCardInvoices.userId, appUser.id)))
    .limit(1);

  await db
    .update(creditCardInvoices)
    .set({
      amountCents,
      ...(dueDay && invoiceMonth
        ? { dueDate: composeMonthDate(invoiceMonth.year, invoiceMonth.month, dueDay) }
        : {}),
      updatedAt: new Date(),
    })
    .where(and(eq(creditCardInvoices.id, invoiceId), eq(creditCardInvoices.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/cards");
  revalidatePath("/app/calendar");
  return successState("Fatura atualizada.");
}

export async function deleteInvoice(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const invoiceId = String(formData.get("invoiceId") ?? "");

  if (!db || !appUser || !invoiceId) {
    throw new Error("Não foi possível excluir a fatura.");
  }

  await db
    .delete(creditCardInvoices)
    .where(and(eq(creditCardInvoices.id, invoiceId), eq(creditCardInvoices.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/cards");
  revalidatePath("/app/calendar");
}
