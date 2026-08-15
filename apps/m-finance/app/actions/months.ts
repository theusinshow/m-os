"use server";

import { revalidatePath } from "next/cache";
import { createCurrentMonthForUser, getAppUserBySupabaseId } from "@/lib/months";
import { requireUser } from "@/lib/auth/guard";

export async function createCurrentMonth() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!appUser) {
    throw new Error("Usuário interno não encontrado.");
  }

  await createCurrentMonthForUser(appUser.id);
  revalidatePath("/app/dashboard");
}
