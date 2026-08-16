"use server";

import { revalidatePath } from "next/cache";
import { and, eq } from "drizzle-orm";
import { bills, months } from "@/db/schema";
import { requireUser } from "@/lib/auth/guard";
import { db } from "@/db/client";
import { billSchema, createBillSchema } from "@/lib/validators/bill";
import { parseCurrencyToCents } from "@/lib/money";
import { composeMonthDate, parseDueDay } from "@/lib/due-date";
import { ensureConsecutiveMonthsForUser, getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser } from "@/lib/active-month";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

export async function createBill(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const currentMonth = await getActiveMonthForUser(appUser.id);

  if (!currentMonth) {
    return errorState("Crie o mês atual antes de cadastrar conta.");
  }

  const categoryId = String(formData.get("categoryId") ?? "");
  const scheduleType = String(formData.get("scheduleType") ?? "once");
  const repeatMonthsRaw = String(formData.get("repeatMonths") ?? "").trim();
  const parsed = createBillSchema.safeParse({
    name: formData.get("name"),
    amountCents: parseCurrencyToCents(formData.get("amount")),
    categoryId: categoryId || undefined,
    dueDay: parseDueDay(formData.get("dueDay")),
    isRecurring: scheduleType === "ongoing",
    scheduleType,
    repeatMonths: repeatMonthsRaw ? Number(repeatMonthsRaw) : undefined,
    notes: formData.get("notes") || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount" }),
    );
  }

  const payload = parsed.data;

  // No day informed defaults to the end of the month, so the bill never looks
  // overdue just because the user skipped the date.
  const occurrenceTotal = payload.scheduleType === "fixed" ? (payload.repeatMonths ?? 1) : 1;
  const targetMonths =
    occurrenceTotal > 1
      ? await ensureConsecutiveMonthsForUser(
          appUser.id,
          currentMonth.month,
          currentMonth.year,
          occurrenceTotal,
        )
      : [currentMonth];
  const seriesId = occurrenceTotal > 1 ? crypto.randomUUID() : null;

  await db.insert(bills).values(
    targetMonths.map((month, index) => ({
      userId: appUser.id,
      monthId: month.id,
      categoryId: payload.categoryId ?? null,
      name: payload.name,
      amountCents: payload.amountCents,
      dueDate: composeMonthDate(month.year, month.month, payload.dueDay ?? 31),
      isRecurring: payload.scheduleType === "ongoing",
      seriesId,
      seriesNumber: seriesId ? index + 1 : null,
      seriesTotal: seriesId ? occurrenceTotal : null,
      status: "pending" as const,
      notes: payload.notes ?? null,
    })),
  );

  revalidatePath("/app/dashboard");
  revalidatePath("/app/bills");
  revalidatePath("/app/calendar");
  return successState(
    occurrenceTotal > 1
      ? `Conta adicionada por ${occurrenceTotal} meses.`
      : "Conta adicionada.",
  );
}

export async function markBillAsPaid(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const billId = String(formData.get("billId") ?? "");

  if (!db || !appUser || !billId) {
    throw new Error("Não foi possível marcar a conta como paga.");
  }

  await db
    .update(bills)
    .set({
      status: "paid",
      paidAt: new Date(),
      updatedAt: new Date(),
    })
    .where(and(eq(bills.id, billId), eq(bills.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/bills");
  revalidatePath("/app/calendar");
}

export async function markBillAsPending(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const billId = String(formData.get("billId") ?? "");

  if (!db || !appUser || !billId) {
    throw new Error("Não foi possível reabrir a conta.");
  }

  await db
    .update(bills)
    .set({
      status: "pending",
      paidAt: null,
      updatedAt: new Date(),
    })
    .where(and(eq(bills.id, billId), eq(bills.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/bills");
  revalidatePath("/app/calendar");
}

export async function updateBill(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const billId = String(formData.get("billId") ?? "");

  if (!db || !appUser || !billId) {
    return errorState("Não foi possível editar a conta.");
  }

  const categoryId = String(formData.get("categoryId") ?? "");
  const parsed = billSchema.safeParse({
    name: formData.get("name"),
    amountCents: parseCurrencyToCents(formData.get("amount")),
    categoryId: categoryId || undefined,
    dueDay: parseDueDay(formData.get("dueDay")),
    isRecurring: formData.get("isRecurring") === "on",
    notes: formData.get("notes") || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount" }),
    );
  }

  const payload = parsed.data;

  // Recompose the due date only when a day is provided; otherwise keep the
  // existing one. The day lives inside the bill's own month.
  const [billMonth] = await db
    .select({ month: months.month, year: months.year })
    .from(bills)
    .innerJoin(months, eq(bills.monthId, months.id))
    .where(and(eq(bills.id, billId), eq(bills.userId, appUser.id)))
    .limit(1);

  await db
    .update(bills)
    .set({
      categoryId: payload.categoryId ?? null,
      name: payload.name,
      amountCents: payload.amountCents,
      isRecurring: payload.isRecurring,
      notes: payload.notes ?? null,
      ...(payload.dueDay && billMonth
        ? { dueDate: composeMonthDate(billMonth.year, billMonth.month, payload.dueDay) }
        : {}),
      updatedAt: new Date(),
    })
    .where(and(eq(bills.id, billId), eq(bills.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/bills");
  revalidatePath("/app/calendar");
  return successState("Conta atualizada.");
}

export async function deleteBill(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const billId = String(formData.get("billId") ?? "");

  if (!db || !appUser || !billId) {
    throw new Error("Não foi possível excluir a conta.");
  }

  await db.delete(bills).where(and(eq(bills.id, billId), eq(bills.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/bills");
  revalidatePath("/app/calendar");
}

export async function deleteBillSeries(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const seriesId = String(formData.get("seriesId") ?? "");

  if (!db || !appUser || !seriesId) {
    throw new Error("Não foi possível excluir a série.");
  }

  await db
    .delete(bills)
    .where(and(eq(bills.seriesId, seriesId), eq(bills.userId, appUser.id)));

  revalidatePath("/app/dashboard");
  revalidatePath("/app/bills");
  revalidatePath("/app/calendar");
}
