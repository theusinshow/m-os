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
    maxAge: 60 * 60 * 24 * 365,
    sameSite: "lax",
  });

  // The selected month drives every month-scoped page, so refresh the shell.
  revalidatePath("/app", "layout");
}
