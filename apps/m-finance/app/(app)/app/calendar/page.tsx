import { EmptyState } from "@/components/empty-state";
import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { FinancialCalendar, type CalendarEvent } from "@/components/calendar/financial-calendar";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { getBillsByMonth } from "@/lib/bills";
import { getInvoicesByMonth } from "@/lib/cards";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser, isViewingCurrentMonth } from "@/lib/active-month";

function getDaysInMonth(month: number, year: number) {
  return new Date(year, month, 0).getDate();
}

function getFirstWeekdayOffset(month: number, year: number) {
  const jsDay = new Date(year, month - 1, 1).getDay();
  return jsDay === 0 ? 6 : jsDay - 1;
}

function getDayFromDate(date: string) {
  return new Date(`${date}T12:00:00`).getDate();
}

export default async function CalendarPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const currentMonth = appUser ? await getActiveMonthForUser(appUser.id) : null;
  const viewingCurrent = await isViewingCurrentMonth();
  const bills = currentMonth ? await getBillsByMonth(currentMonth.id) : [];
  const invoices = currentMonth ? await getInvoicesByMonth(currentMonth.id) : [];
  const daysInMonth = currentMonth ? getDaysInMonth(currentMonth.month, currentMonth.year) : 0;
  const firstOffset = currentMonth ? getFirstWeekdayOffset(currentMonth.month, currentMonth.year) : 0;
  const now = new Date();
  const todayDay =
    currentMonth && now.getFullYear() === currentMonth.year && now.getMonth() + 1 === currentMonth.month
      ? now.getDate()
      : -1;
  const events: CalendarEvent[] = [
    ...bills.map((bill) => ({
      id: bill.id,
      type: "bill" as const,
      title: bill.name,
      amountCents: bill.amountCents,
      day: getDayFromDate(bill.dueDate),
      status: bill.status,
      label: bill.categoryName ?? "Conta",
    })),
    ...invoices.map((invoice) => ({
      id: invoice.id,
      type: "invoice" as const,
      title: invoice.name,
      amountCents: invoice.amountCents,
      day: getDayFromDate(invoice.dueDate),
      status: invoice.status,
      label: invoice.cardType === "business" ? "Fatura PJ" : "Fatura",
    })),
  ].sort((a, b) => a.day - b.day);

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Calendário financeiro" title="Vencimentos do mês" />

      {!currentMonth && viewingCurrent ? <CreateCurrentMonthCard /> : null}

      {currentMonth ? (
        <FinancialCalendar
          daysInMonth={daysInMonth}
          events={events}
          firstOffset={firstOffset}
          todayDay={todayDay}
        />
      ) : (
        <EmptyState
          title="Calendário aguardando mês"
          description="Crie o mês atual para visualizar vencimentos reais de contas e faturas."
        />
      )}
    </div>
  );
}
