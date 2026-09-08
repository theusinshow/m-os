"use server";

import { cookies } from "next/headers";
import { revalidatePath } from "next/cache";
import { ACTIVE_MONTH_COOKIE, parseMonthValue } from "@/lib/active-month";

export async function setActiveMonth(value: string) {
  if (!parseMonthValue(value)) {
    return;
  }

  const store = await cookies();
  store.set(ACTIVE_MONTH_COOKIE, value, {
    path: "/",
    // O mês escolhido é estado de navegação, não preferência. Guardado por um
    // ano, o app reabria semanas depois num mês vazio do passado e mostrava
    // tudo zerado como se fosse a vida real. Seis horas cobrem a sessão.
    maxAge: 60 * 60 * 6,
    sameSite: "lax",
  });

  // The selected month drives every month-scoped page, so refresh the shell.
  revalidatePath("/app", "layout");
}
