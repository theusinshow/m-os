"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { creditCardInvoices, months } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { ensureMonthForUser, getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthParts } from "@/lib/active-month";
import { parseCurrencyToCents } from "@/lib/money";
import { composeMonthDate, parseDueDay } from "@/lib/due-date";
import { invoiceSchema } from "@/lib/validators/invoice";
import { markInvoicePaid } from "@/lib/domain/finance-actions/mark-paid";
import { upsertInvoiceAmount } from "@/lib/domain/finance-actions/more-entries";
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

  // A fatura é do mês que está na tela. Antes, um mês sem linha em `months`
  // caía no mês do calendário em silêncio — confirmar a fatura de dezembro
  // gravava em setembro.
  const active = await getActiveMonthParts();
  const currentMonth = await ensureMonthForUser(appUser.id, active.month, active.year);

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

  // Mesmo serviço do Hermes (`m-finance.set_invoice_amount`): cria ou corrige a
  // fatura do mês, com o vencimento do cartão quando não vem outro.
  const result = await upsertInvoiceAmount({
    userId: appUser.id,
    cardId: payload.cardId,
    month: currentMonth,
    amountCents: payload.amountCents,
    dueDay: payload.dueDay ?? null,
    notes: payload.notes ?? null,
  });
  if (!result.ok) {
    return errorState(result.code === "not_found" ? "Cartão não encontrado." : result.message);
  }

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

  const paid = await markInvoicePaid(appUser.id, { invoiceId });
  if (!paid.ok && paid.code !== "already_paid") {
    throw new Error(paid.message);
  }

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
