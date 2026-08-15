"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { creditCards } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { cardSchema } from "@/lib/validators/card";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

function revalidateCardSurfaces() {
  revalidatePath("/app/cards");
  revalidatePath("/app/settings");
  revalidatePath("/app/dashboard");
}

export async function createCard(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const parsed = cardSchema.safeParse({
    name: formData.get("name"),
    cardType: formData.get("cardType"),
    dueDay: formData.get("dueDay"),
  });

  if (!parsed.success) {
    return errorState("Revise os campos destacados.", fieldErrorsFromZod(parsed.error));
  }

  const payload = parsed.data;

  await db.insert(creditCards).values({
    userId: appUser.id,
    name: payload.name,
    cardType: payload.cardType,
    dueDay: payload.dueDay,
  });

  revalidateCardSurfaces();
  return successState("Cartão adicionado.");
}

export async function updateCard(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const cardId = String(formData.get("cardId") ?? "");

  if (!db || !appUser || !cardId) {
    return errorState("Não foi possível editar o cartão.");
  }

  const parsed = cardSchema.safeParse({
    name: formData.get("name"),
    cardType: formData.get("cardType"),
    dueDay: formData.get("dueDay"),
  });

  if (!parsed.success) {
    return errorState("Revise os campos destacados.", fieldErrorsFromZod(parsed.error));
  }

  const payload = parsed.data;

  await db
    .update(creditCards)
    .set({
      name: payload.name,
      cardType: payload.cardType,
      dueDay: payload.dueDay,
      updatedAt: new Date(),
    })
    .where(and(eq(creditCards.id, cardId), eq(creditCards.userId, appUser.id)));

  revalidateCardSurfaces();
  return successState("Cartão atualizado.");
}

export async function setCardActive(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const cardId = String(formData.get("cardId") ?? "");
  const isActive = formData.get("isActive") === "true";

  if (!db || !appUser || !cardId) {
    throw new Error("Não foi possível atualizar o cartão.");
  }

  await db
    .update(creditCards)
    .set({ isActive, updatedAt: new Date() })
    .where(and(eq(creditCards.id, cardId), eq(creditCards.userId, appUser.id)));

  revalidateCardSurfaces();
}
