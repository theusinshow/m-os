import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { CardManager } from "@/components/cards/card-manager";
import { CardFormDrawer } from "@/components/cards/card-form-drawer";
import { PageHeading } from "@/components/page-heading";
import { InvoiceFormDrawer } from "@/components/cards/invoice-form-drawer";
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

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Cartões" title="Faturas simples">
        <div className="flex flex-wrap items-center gap-2">
          <CardFormDrawer />
          {currentMonth ? <InvoiceFormDrawer cards={cards} /> : null}
        </div>
      </PageHeading>

      {!currentMonth && viewingCurrent ? <CreateCurrentMonthCard /> : null}

      <CardManager cards={managedCards} invoices={invoices} />
    </div>
  );
}
