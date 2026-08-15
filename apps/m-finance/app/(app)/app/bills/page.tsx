import { BillFormCard } from "@/components/bills/bill-form-card";
import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { getBillCategories, getBillsByMonth } from "@/lib/bills";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser, isViewingCurrentMonth } from "@/lib/active-month";

export default async function BillsPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const currentMonth = appUser ? await getActiveMonthForUser(appUser.id) : null;
  const viewingCurrent = await isViewingCurrentMonth();
  const categories = appUser ? await getBillCategories(appUser.id) : [];
  const bills = currentMonth ? await getBillsByMonth(currentMonth.id) : [];

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Despesas" title="Despesas do mês" />

      {!currentMonth && viewingCurrent ? <CreateCurrentMonthCard /> : null}

      {currentMonth ? <BillFormCard bills={bills} categories={categories} /> : null}
    </div>
  );
}
