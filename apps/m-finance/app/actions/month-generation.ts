"use server";

import { revalidatePath } from "next/cache";
import { bills, months } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId, getNextMonthParts } from "@/lib/months";
import { getActiveMonthForUser } from "@/lib/active-month";
import { parseCurrencyToCents } from "@/lib/money";
import { composeMonthDate, parseDueDay } from "@/lib/due-date";
import { writeMonthSnapshot } from "@/lib/snapshot";

function field(formData: FormData, name: string) {
  return String(formData.get(name) ?? "").trim();
}

export async function generateNextMonthFromReview(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    throw new Error("Banco ou usuário interno não configurado.");
  }

  // Snapshot the month being left behind so the history series accrues on its
  // own — the user no longer has to remember to save it before moving on.
  const leavingMonth = await getActiveMonthForUser(appUser.id);
  if (leavingMonth) {
    await writeMonthSnapshot(appUser.id, leavingMonth.id);
  }

  const next = getNextMonthParts();
  const [nextMonth] = await db
    .insert(months)
    .values({
      userId: appUser.id,
      month: next.month,
      year: next.year,
    })
    .onConflictDoUpdate({
      target: [months.userId, months.month, months.year],
      set: {
        updatedAt: new Date(),
      },
    })
    .returning();

  const sourceIds = formData.getAll("sourceBillId").map(String);
  const reviewedBills = sourceIds
    .filter((id) => formData.get(`include-${id}`) === "on")
    .map((id) => ({
      userId: appUser.id,
      monthId: nextMonth.id,
      categoryId: field(formData, `categoryId-${id}`) || null,
      name: field(formData, `name-${id}`),
      amountCents: parseCurrencyToCents(formData.get(`amount-${id}`)),
      dueDate: composeMonthDate(
        next.year,
        next.month,
        parseDueDay(formData.get(`dueDay-${id}`)) ?? 31,
      ),
      isRecurring: true,
      status: "pending" as const,
    }))
    .filter((bill) => bill.name && bill.amountCents > 0);

  if (reviewedBills.length > 0) {
    await db.insert(bills).values(reviewedBills);
  }

  revalidatePath("/app/dashboard");
  revalidatePath("/app/bills");
  revalidatePath("/app/calendar");
  revalidatePath("/app/history");
}
