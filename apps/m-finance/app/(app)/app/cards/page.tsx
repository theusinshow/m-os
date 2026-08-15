import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { CardManager } from "@/components/cards/card-manager";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { InvoicesByCardChart } from "@/components/charts/invoices-by-card-chart";
import { PageHeading } from "@/components/page-heading";
import { InvoiceFormCard } from "@/components/cards/invoice-form-card";
import { InvoiceSummaryCard } from "@/components/dashboard/invoice-summary-card";
import { requireUser } from "@/lib/auth/guard";
import { getCreditCards, getInvoicesByMonth, getManagedCreditCards } from "@/lib/cards";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser, isViewingCurrentMonth } from "@/lib/active-month";

export default async function CardsPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const currentMonth = appUser ? await getActiveMonthForUser(appUser.id) : null;
  const viewingCurrent = await isViewingCurrentMonth();
  const cards = appUser ? await getCreditCards(appUser.id) : [];
  const managedCards = appUser ? await getManagedCreditCards(appUser.id) : [];
  const invoices = currentMonth ? await getInvoicesByMonth(currentMonth.id) : [];
  const invoicesByCard = Object.values(
    invoices.reduce<Record<string, { name: string; value: number; isBusiness: boolean }>>(
      (acc, invoice) => {
        acc[invoice.name] ??= {
          name: invoice.name,
          value: 0,
          isBusiness: invoice.cardType === "business",
        };
        acc[invoice.name].value += invoice.amountCents;
        return acc;
      },
      {},
    ),
  );

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Cartões" title="Faturas simples" />

      {!currentMonth && viewingCurrent ? <CreateCurrentMonthCard /> : null}

      <CardManager cards={managedCards} />

      {currentMonth ? <InvoiceFormCard cards={cards} /> : null}

      {invoicesByCard.length > 0 ? (
        <DashboardCard description="Total de fatura por cartão neste mês." title="Faturas por cartão">
          <InvoicesByCardChart data={invoicesByCard} />
        </DashboardCard>
      ) : null}

      <InvoiceSummaryCard invoices={invoices} />
    </div>
  );
}
