"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { subscriptions } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { parseCurrencyToCents } from "@/lib/money";
import { subscriptionSchema } from "@/lib/validators/subscription";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

export async function addSubscription(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!db || !appUser) {
    return errorState("Não foi possível salvar.");
  }

  const reminderRaw = String(formData.get("reminderDaysBefore") ?? "").trim();

  const parsed = subscriptionSchema.safeParse({
    name: formData.get("name"),
    amountCents: parseCurrencyToCents(formData.get("amount")),
    nextChargeDate: String(formData.get("nextChargeDate") ?? ""),
    cycle: formData.get("cycle") ?? "monthly",
    isTrial: formData.get("isTrial") === "on",
    reminderDaysBefore: reminderRaw === "" ? 1 : Number(reminderRaw),
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount" }),
    );
  }

  const data = parsed.data;

  await db.insert(subscriptions).values({
    userId: appUser.id,
    name: data.name,
    amountCents: data.amountCents,
    nextChargeDate: data.nextChargeDate,
    cycle: data.cycle,
    status: data.isTrial ? "trial" : "active",
    reminderDaysBefore: data.reminderDaysBefore,
  });

  revalidatePath("/app/subscriptions");
  return successState(data.isTrial ? "Teste grátis salvo." : "Assinatura salva.");
}

export async function cancelSubscription(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const id = String(formData.get("id") ?? "");
  if (!db || !appUser || !id) {
    throw new Error("Não foi possível cancelar.");
  }

  await db
    .update(subscriptions)
    .set({ status: "canceled", updatedAt: new Date() })
    .where(and(eq(subscriptions.id, id), eq(subscriptions.userId, appUser.id)));

  revalidatePath("/app/subscriptions");
}

export async function deleteSubscription(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const id = String(formData.get("id") ?? "");
  if (!db || !appUser || !id) {
    throw new Error("Não foi possível excluir.");
  }

  await db
    .delete(subscriptions)
    .where(and(eq(subscriptions.id, id), eq(subscriptions.userId, appUser.id)));

  revalidatePath("/app/subscriptions");
}
