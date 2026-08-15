"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { db } from "@/db/client";
import { pluggyItems } from "@/db/schema";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getCardById } from "@/lib/card-expenses";
import { listAccounts } from "@/lib/openfinance/pluggy";
import { syncPluggyItem } from "@/lib/openfinance/sync";

/**
 * Called once the Pluggy Connect widget returns an `itemId`. Reads the item's
 * credit-card accounts and stores one mapping row per account (still unlinked
 * to a local card). Returns the accounts so the UI can render the linking step.
 */
export async function registerPluggyItem(itemId: string) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!db || !appUser) {
    return { ok: false as const, error: "Usuário interno não configurado." };
  }

  // Fetch every account so we can tell the user *what* the connection exposed
  // when there's no credit card (e.g. Mercado Pago wallets come back as BANK).
  let accounts;
  try {
    accounts = await listAccounts(itemId);
  } catch (error) {
    return { ok: false as const, error: error instanceof Error ? error.message : "Erro Pluggy." };
  }

  console.log(
    `[pluggy] item ${itemId} → ${accounts.length} conta(s):`,
    accounts.map((a) => `${a.type}:${a.marketingName || a.name}`).join(" | ") || "(nenhuma)",
  );

  const creditAccounts = accounts.filter((a) => a.type === "CREDIT");
  if (creditAccounts.length === 0) {
    const types = [...new Set(accounts.map((a) => a.type))].join(", ") || "nenhuma";
    return {
      ok: false as const,
      error: `Essa conexão não tem cartão de crédito (contas encontradas: ${types}). O app importa apenas contas do tipo CREDIT. Se a fatura ainda estava sincronizando, aguarde alguns segundos e tente de novo.`,
    };
  }

  for (const account of creditAccounts) {
    await db
      .insert(pluggyItems)
      .values({
        userId: appUser.id,
        itemId,
        accountId: account.id,
        connectorName: account.marketingName || account.name,
        status: "pending",
      })
      .onConflictDoUpdate({
        target: [pluggyItems.userId, pluggyItems.itemId],
        set: { accountId: account.id, updatedAt: new Date() },
      });
  }

  revalidatePath("/app/settings");
  return {
    ok: true as const,
    accounts: creditAccounts.map((a) => ({ id: a.id, name: a.marketingName || a.name })),
  };
}

/**
 * Links a Pluggy credit account to a local card and runs the first sync.
 */
export async function linkPluggyAccountToCard(itemId: string, accountId: string, cardId: string) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!db || !appUser) {
    return { ok: false as const, error: "Usuário interno não configurado." };
  }

  const card = await getCardById(appUser.id, cardId);
  if (!card) {
    return { ok: false as const, error: "Cartão não encontrado." };
  }

  await db
    .update(pluggyItems)
    .set({ cardId, accountId, status: "updating", updatedAt: new Date() })
    .where(and(eq(pluggyItems.userId, appUser.id), eq(pluggyItems.itemId, itemId)));

  try {
    const results = await syncPluggyItem(appUser.id, itemId);
    revalidatePath(`/app/cards/${cardId}`);
    revalidatePath("/app/cards");
    revalidatePath("/app/dashboard");
    revalidatePath("/app/settings");
    return { ok: true as const, results };
  } catch (error) {
    return { ok: false as const, error: error instanceof Error ? error.message : "Erro no sync." };
  }
}

/**
 * Form-friendly "sync now" for one connection. Throws on failure so ToastForm
 * surfaces the error toast.
 */
export async function syncConnection(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const itemId = String(formData.get("itemId") ?? "");
  if (!db || !appUser || !itemId) {
    throw new Error("Não foi possível sincronizar.");
  }

  await syncPluggyItem(appUser.id, itemId);
  revalidatePath("/app/settings");
  revalidatePath("/app/cards");
  revalidatePath("/app/dashboard");
}

/**
 * Removes a connection mapping locally. Imported expenses are kept (they're now
 * part of the user's history); only the bank link is dropped.
 */
export async function unlinkConnection(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const itemId = String(formData.get("itemId") ?? "");
  if (!db || !appUser || !itemId) {
    throw new Error("Não foi possível desvincular.");
  }

  await db
    .delete(pluggyItems)
    .where(and(eq(pluggyItems.userId, appUser.id), eq(pluggyItems.itemId, itemId)));
  revalidatePath("/app/settings");
}
