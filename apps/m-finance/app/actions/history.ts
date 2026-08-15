"use server";

import { revalidatePath } from "next/cache";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser } from "@/lib/active-month";
import { writeMonthSnapshot } from "@/lib/snapshot";

export async function saveCurrentMonthSnapshot() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    throw new Error("Banco ou usuário interno não configurado.");
  }

  // Snapshot the month the user is viewing, not always the calendar month.
  const currentMonth = await getActiveMonthForUser(appUser.id);

  if (!currentMonth) {
    throw new Error("Crie o mês atual antes de salvar histórico.");
  }

  await writeMonthSnapshot(appUser.id, currentMonth.id);

  revalidatePath("/app/history");
  revalidatePath("/app/dashboard");
}
