import { and, eq } from "drizzle-orm";
import { db } from "@/db/client";
import { alerts } from "@/db/schema";
import { calculateInternalAlerts } from "@/lib/calculations/alerts";
import { getBillsByMonth } from "@/lib/bills";
import { getInvoicesByMonth } from "@/lib/cards";
import { getSettingsForUser } from "@/lib/settings";

export type NotificationItem = {
  id: string;
  title: string;
  message: string;
  severity: "info" | "warning" | "danger";
  read: boolean;
};

export type NotificationData = {
  items: NotificationItem[];
  unreadCount: number;
};

const SEVERITY_ORDER = { danger: 0, warning: 1, info: 2 } as const;

// A read receipt is keyed by the payable + alert kind, ignoring the trigger date,
// so reading "vence hoje" doesn't re-notify the same day, but a status change
// (e.g. it becomes "vencida") surfaces a fresh, unread alert.
export function alertKey(entityType: string, entityId: string, alertType: string) {
  return `${entityType}:${entityId}:${alertType}`;
}

type MonthRef = { id: string } | null;

export async function getNotifications(
  userId: string,
  month: MonthRef,
): Promise<NotificationData> {
  if (!db || !month) {
    return { items: [], unreadCount: 0 };
  }

  const [settings, bills, invoices, readRows] = await Promise.all([
    getSettingsForUser(userId),
    getBillsByMonth(month.id),
    getInvoicesByMonth(month.id),
    db
      .select({
        alertType: alerts.alertType,
        entityType: alerts.entityType,
        entityId: alerts.entityId,
      })
      .from(alerts)
      .where(and(eq(alerts.userId, userId), eq(alerts.isRead, true))),
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

  const items = computed
    .map((alert) => ({
      id: alert.id,
      title: alert.title,
      message: alert.message,
      severity: alert.severity,
      read: readKeys.has(alertKey(alert.entityType, alert.entityId, alert.type)),
    }))
    .sort((a, b) => SEVERITY_ORDER[a.severity] - SEVERITY_ORDER[b.severity]);

  return { items, unreadCount: items.filter((item) => !item.read).length };
}
