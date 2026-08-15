"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { billCategories, settings } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { categorySchema, settingsSchema } from "@/lib/validators/settings";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

function slugify(value: string) {
  return value
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
}

export async function updateSettings(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const parsed = settingsSchema.safeParse({
    alertDaysBefore: formData.get("alertDaysBefore"),
  });

  if (!parsed.success) {
    return errorState("Revise os campos destacados.", fieldErrorsFromZod(parsed.error));
  }

  const payload = parsed.data;

  await db
    .insert(settings)
    .values({ userId: appUser.id, alertDaysBefore: payload.alertDaysBefore })
    .onConflictDoUpdate({
      target: settings.userId,
      set: { alertDaysBefore: payload.alertDaysBefore, updatedAt: new Date() },
    });

  revalidatePath("/app/settings");
  revalidatePath("/app/dashboard");
  return successState("Preferências salvas.");
}

export async function createCategory(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const parsed = categorySchema.safeParse({ name: formData.get("name") });

  if (!parsed.success) {
    return errorState("Revise os campos destacados.", fieldErrorsFromZod(parsed.error));
  }

  const payload = parsed.data;
  const slug = slugify(payload.name);

  if (!slug) {
    return errorState("Revise os campos destacados.", {
      name: "Informe um nome de categoria válido.",
    });
  }

  await db
    .insert(billCategories)
    .values({ userId: appUser.id, name: payload.name, slug })
    .onConflictDoUpdate({
      target: [billCategories.userId, billCategories.slug],
      set: { name: payload.name, isArchived: false, updatedAt: new Date() },
    });

  revalidatePath("/app/settings");
  revalidatePath("/app/bills");
  return successState("Categoria adicionada.");
}

export async function setCategoryArchived(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const categoryId = String(formData.get("categoryId") ?? "");
  const isArchived = formData.get("isArchived") === "true";

  if (!db || !appUser || !categoryId) {
    throw new Error("Não foi possível atualizar a categoria.");
  }

  await db
    .update(billCategories)
    .set({ isArchived, updatedAt: new Date() })
    .where(and(eq(billCategories.id, categoryId), eq(billCategories.userId, appUser.id)));

  revalidatePath("/app/settings");
  revalidatePath("/app/bills");
}
