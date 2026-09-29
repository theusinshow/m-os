"use server";

import { revalidatePath } from "next/cache";
import { requireUser } from "@/lib/auth/guard";
import { resetFinancialPolicy, setFinancialPolicy } from "@/lib/domain/finance-actions/entries";
import { runFinancialObserver, setInsightStatus } from "@/lib/finance-intelligence/observer/run";
import { policiesFromForm } from "@/lib/finance-intelligence/policy-form";
import { errorState, successState, type FormState } from "@/lib/form-state";
import { getAppUserBySupabaseId } from "@/lib/months";

async function ownerId() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!appUser) throw new Error("Usuário interno não configurado.");
  return appUser.id;
}

export async function acknowledgeInsight(formData: FormData) {
  const userId = await ownerId();
  await setInsightStatus(userId, String(formData.get("insightId") ?? ""), "acknowledged");
  revalidatePath("/app/dashboard");
}

export async function resolveInsight(formData: FormData) {
  const userId = await ownerId();
  await setInsightStatus(userId, String(formData.get("insightId") ?? ""), "resolved");
  revalidatePath("/app/dashboard");
}

/** Roda o Observer agora, em vez de esperar o cron do dia. */
export async function refreshInsights() {
  const userId = await ownerId();
  await runFinancialObserver(userId);
  revalidatePath("/app/dashboard");
}

export async function savePolicies(_prev: FormState, formData: FormData): Promise<FormState> {
  const userId = await ownerId();
  const parsed = policiesFromForm(formData);
  if (Object.keys(parsed.errors).length > 0) {
    return errorState("Revise os campos destacados.", parsed.errors);
  }

  for (const entry of parsed.set) {
    const result = await setFinancialPolicy(userId, entry.key, entry.value, "user");
    if (!result.ok) return errorState(result.message);
  }
  for (const key of parsed.reset) {
    await resetFinancialPolicy(userId, key);
  }

  revalidatePath("/app/settings");
  revalidatePath("/app/dashboard");
  return successState("Políticas salvas. O Safe-to-Spend já usa as novas regras.");
}
