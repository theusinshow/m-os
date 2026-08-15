import { eq, ne } from "drizzle-orm";
import { db } from "@/db/client";
import { subscriptions } from "@/db/schema";
import { sendPushToUser } from "@/lib/push/web-push";
import { formatCurrency } from "@/lib/formatters/currency";

/** Today as yyyy-mm-dd in the user's timezone (charges are local dates). */
function todayInSaoPaulo() {
  return new Intl.DateTimeFormat("en-CA", { timeZone: "America/Sao_Paulo" }).format(new Date());
}

function addDays(iso: string, days: number) {
  const d = new Date(`${iso}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

function diffDays(a: string, b: string) {
  const da = new Date(`${a}T00:00:00Z`).getTime();
  const db2 = new Date(`${b}T00:00:00Z`).getTime();
  return Math.round((da - db2) / 86_400_000);
}

function nextPeriod(iso: string, cycle: "monthly" | "yearly") {
  const d = new Date(`${iso}T00:00:00Z`);
  if (cycle === "monthly") d.setUTCMonth(d.getUTCMonth() + 1);
  else d.setUTCFullYear(d.getUTCFullYear() + 1);
  return d.toISOString().slice(0, 10);
}

/**
 * Daily reminder pass: pushes a heads-up when a subscription/trial is within its
 * reminder window (and hasn't been notified for that charge yet), then rolls
 * recurring subscriptions forward once their charge date has passed.
 */
export async function runSubscriptionReminders() {
  if (!db) return { notified: 0, advanced: 0 };

  const today = todayInSaoPaulo();
  const rows = await db.select().from(subscriptions).where(ne(subscriptions.status, "canceled"));

  let notified = 0;
  let advanced = 0;

  for (const sub of rows) {
    const reminderDate = addDays(sub.nextChargeDate, -sub.reminderDaysBefore);
    const days = diffDays(sub.nextChargeDate, today);

    const inWindow =
      today >= reminderDate &&
      today <= sub.nextChargeDate &&
      sub.lastNotifiedFor !== sub.nextChargeDate;

    if (inWindow) {
      const whenText = days <= 0 ? "hoje" : days === 1 ? "amanhã" : `em ${days} dias`;
      const verb = sub.status === "trial" ? "começa a cobrar" : "será cobrada";
      const delivered = await sendPushToUser(sub.userId, {
        title: `${sub.name} ${verb} ${whenText}`,
        body: `${formatCurrency(sub.amountCents)} ${whenText}. Cancele antes se não quiser continuar.`,
        url: "/app/subscriptions",
        tag: `sub-${sub.id}`,
      });
      if (delivered > 0) {
        await db
          .update(subscriptions)
          .set({ lastNotifiedFor: sub.nextChargeDate, updatedAt: new Date() })
          .where(eq(subscriptions.id, sub.id));
        notified += 1;
      }
    }

    // Charge date passed → advance recurring ones to the next period (trial
    // becomes a paid active subscription).
    if (today > sub.nextChargeDate && (sub.cycle === "monthly" || sub.cycle === "yearly")) {
      let next = nextPeriod(sub.nextChargeDate, sub.cycle);
      while (next < today) next = nextPeriod(next, sub.cycle);
      await db
        .update(subscriptions)
        .set({ nextChargeDate: next, status: "active", updatedAt: new Date() })
        .where(eq(subscriptions.id, sub.id));
      advanced += 1;
    }
  }

  return { notified, advanced };
}
