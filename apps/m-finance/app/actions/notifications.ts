"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { alerts } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getActiveMonthForUser } from "@/lib/active-month";
import { calculateInternalAlerts } from "@/lib/calculations/alerts";
import { getBillsByMonth } from "@/lib/bills";
import { getInvoicesByMonth } from "@/lib/cards";
import { getAppUserBySupabaseId } from "@/lib/months";
import { alertKey } from "@/lib/notifications";
import { getSettingsForUser } from "@/lib/settings";

export async function markAllNotificationsRead() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!db || !appUser) return;

  const month = await getActiveMonthForUser(appUser.id);
  if (!month) return;

  const [settings, bills, invoices, readRows] = await Promise.all([
    getSettingsForUser(appUser.id),
    getBillsByMonth(month.id),
    getInvoicesByMonth(month.id),
    db
      .select({
        alertType: alerts.alertType,
        entityType: alerts.entityType,
        entityId: alerts.entityId,
      })
      .from(alerts)
      .where(and(eq(alerts.userId, appUser.id), eq(alerts.isRead, true))),
  ]);

  const readKeys = new Set(
    readRows.map((row) => alertKey(row.entityType, row.entityId, row.alertType)),
  );

  const computed = calculateInternalAlerts(
    [
      ...bills.map((bill) => ({
        id: bill.id,
        type: "bill" as const,
        title: bill.name,
        amountCents: bill.amountCents,
        dueDate: bill.dueDate,
        status: bill.status,
      })),
      ...invoices.map((invoice) => ({
        id: invoice.id,
        type: "invoice" as const,
        title: invoice.name,
        amountCents: invoice.amountCents,
        dueDate: invoice.dueDate,
        status: invoice.status,
      })),
    ],
    { daysBefore: settings?.alertDaysBefore ?? 3 },
  );

  const triggerDate = new Date().toISOString().slice(0, 10);
  const toInsert = computed
    .filter((alert) => !readKeys.has(alertKey(alert.entityType, alert.entityId, alert.type)))
    .map((alert) => ({
      userId: appUser.id,
      monthId: month.id,
      alertType: alert.type,
      entityType: alert.entityType,
      entityId: alert.entityId,
      title: alert.title,
      message: alert.message,
      severity: alert.severity,
      isRead: true,
      triggerDate,
    }));

  if (toInsert.length > 0) {
    await db.insert(alerts).values(toInsert);
  }

  revalidatePath("/app", "layout");
}
