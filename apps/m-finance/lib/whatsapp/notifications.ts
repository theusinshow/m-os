import { and, asc, eq, lte, ne, sql } from "drizzle-orm";
import { db } from "@/db/client";
import { bills, creditCardInvoices, creditCards, settings } from "@/db/schema";
import { env, isTwilioConfigured } from "@/lib/env";
import { formatCurrency } from "@/lib/formatters/currency";
import { getCurrentMonthForUser } from "@/lib/months";
import { getWhatsappMonthlySummary } from "@/lib/finance/whatsapp-summary";
import { getWhatsappOwnerUser } from "@/lib/whatsapp/auth";
import {
  isWithinWhatsappWindow,
  sendWhatsappNotification,
  wasWhatsappNotificationSent,
} from "@/lib/whatsapp/twilio-outbound";

type NotificationResult = { sent: boolean; reason: string };

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

function formatIsoDate(iso: string) {
  const [year, month, day] = iso.split("-");
  if (!year || !month || !day) return iso;
  return `${day}/${month}/${year}`;
}

function relativeDueLabel(dueDate: string, today: string) {
  const days = diffDays(dueDate, today);
  if (days < 0) return "vencido";
  if (days === 0) return "hoje";
  if (days === 1) return "amanhã";
  return `em ${days} dias`;
}

async function getAlertDaysBefore(userId: string) {
  if (!db) return 3;
  const [row] = await db
    .select({ alertDaysBefore: settings.alertDaysBefore })
    .from(settings)
    .where(eq(settings.userId, userId))
    .limit(1);
  return row?.alertDaysBefore ?? 3;
}

/**
 * Notificação diária de vencimentos: reúne contas e faturas do mês atual cujo
 * vencimento cai hoje, já venceu ou está dentro da janela de alerta do usuário.
 * Respeita a janela de 24h do WhatsApp e é idempotente por dia.
 */
export async function runWhatsappDueReminders(): Promise<NotificationResult> {
  if (!db || !isTwilioConfigured() || !env.whatsappAllowedPhone) {
    return { sent: false, reason: "not_configured" };
  }

  const user = await getWhatsappOwnerUser();
  if (!user) return { sent: false, reason: "no_owner" };

  const phone = env.whatsappAllowedPhone;
  const today = todayInSaoPaulo();
  const notificationKey = `due-reminders-${today}`;

  if (await wasWhatsappNotificationSent(notificationKey)) {
    return { sent: false, reason: "already_sent" };
  }
  if (!(await isWithinWhatsappWindow(phone))) {
    return { sent: false, reason: "window_closed" };
  }

  const month = await getCurrentMonthForUser(user.id);
  if (!month) return { sent: false, reason: "no_current_month" };

  const alertDaysBefore = await getAlertDaysBefore(user.id);
  const horizon = addDays(today, alertDaysBefore);

  const dueBills = await db
    .select({
      name: bills.name,
      amountCents: bills.amountCents,
      dueDate: bills.dueDate,
    })
    .from(bills)
    .where(
      and(
        eq(bills.userId, user.id),
        eq(bills.monthId, month.id),
        lte(bills.dueDate, horizon),
        ne(bills.status, sql`'paid'`),
      ),
    )
    .orderBy(asc(bills.dueDate));

  const dueInvoices = await db
    .select({
      name: creditCards.name,
      amountCents: creditCardInvoices.amountCents,
      dueDate: creditCardInvoices.dueDate,
    })
    .from(creditCardInvoices)
    .innerJoin(creditCards, eq(creditCardInvoices.cardId, creditCards.id))
    .where(
      and(
        eq(creditCardInvoices.userId, user.id),
        eq(creditCardInvoices.monthId, month.id),
        lte(creditCardInvoices.dueDate, horizon),
        ne(creditCardInvoices.status, sql`'paid'`),
      ),
    )
    .orderBy(asc(creditCardInvoices.dueDate));

  const items = [
    ...dueBills.map((bill) => ({ type: "Conta", ...bill })),
    ...dueInvoices.map((invoice) => ({ type: "Fatura", ...invoice })),
  ].sort((a, b) => a.dueDate.localeCompare(b.dueDate));

  if (items.length === 0) {
    return { sent: false, reason: "no_due_items" };
  }

  const lines = items.map(
    (item) =>
      `• ${item.type}: ${item.name} — ${formatCurrency(item.amountCents)} — ${formatIsoDate(item.dueDate)} (${relativeDueLabel(item.dueDate, today)})`,
  );
  const body = ["Vencimentos próximos:", "", ...lines].join("\n");

  const result = await sendWhatsappNotification({ userId: user.id, to: phone, body, notificationKey });
  return { sent: result.ok, reason: result.ok ? "sent" : result.error };
}

/**
 * Resumo semanal enviado às segundas-feiras. Reaproveita o resumo mensal já
 * usado pelo comando `resumo`. Também respeita a janela de 24h e é idempotente.
 */
export async function runWhatsappWeeklySummary(): Promise<NotificationResult> {
  if (!db || !isTwilioConfigured() || !env.whatsappAllowedPhone) {
    return { sent: false, reason: "not_configured" };
  }

  // Segunda-feira = dia 1 no horário local de São Paulo.
  const weekday = new Intl.DateTimeFormat("en-US", {
    timeZone: "America/Sao_Paulo",
    weekday: "short",
  }).format(new Date());
  if (weekday !== "Mon") {
    return { sent: false, reason: "not_monday" };
  }

  const user = await getWhatsappOwnerUser();
  if (!user) return { sent: false, reason: "no_owner" };

  const phone = env.whatsappAllowedPhone;
  const today = todayInSaoPaulo();
  const notificationKey = `weekly-summary-${today}`;

  if (await wasWhatsappNotificationSent(notificationKey)) {
    return { sent: false, reason: "already_sent" };
  }
  if (!(await isWithinWhatsappWindow(phone))) {
    return { sent: false, reason: "window_closed" };
  }

  const summary = await getWhatsappMonthlySummary(user.id);
  const body = `Resumo da semana:\n\n${summary}`;

  const result = await sendWhatsappNotification({ userId: user.id, to: phone, body, notificationKey });
  return { sent: result.ok, reason: result.ok ? "sent" : result.error };
}
