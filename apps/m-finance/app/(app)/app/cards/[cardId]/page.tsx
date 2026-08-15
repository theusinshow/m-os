import Link from "next/link";
import { notFound } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { CardDetail } from "@/components/cards/card-detail";
import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser, isViewingCurrentMonth } from "@/lib/active-month";
import { getCardById, getCardExpenses } from "@/lib/card-expenses";
import { getInvoiceForCardMonth } from "@/lib/cards";
import { formatMonthLabel } from "@/lib/formatters/date";

export default async function CardDetailPage({
  params,
}: {
  params: Promise<{ cardId: string }>;
}) {
  const { cardId } = await params;
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!appUser) {
    notFound();
  }

  const card = await getCardById(appUser.id, cardId);

  if (!card) {
    notFound();
  }

  const month = await getActiveMonthForUser(appUser.id);
  const viewingCurrent = await isViewingCurrentMonth();
  const monthLabel = month
    ? formatMonthLabel(new Date(month.year, month.month - 1, 1))
    : formatMonthLabel();

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Cartão" title={card.name}>
        <Link
          className="focus-ring inline-flex min-h-11 items-center gap-2 rounded-md border border-border-subtle px-4 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-default hover:bg-background-hover hover:text-text-primary"
          href="/app/cards"
        >
          <ArrowLeft size={16} aria-hidden="true" />
          Cartões
        </Link>
      </PageHeading>

      {!month ? (
        viewingCurrent ? <CreateCurrentMonthCard /> : null
      ) : (
        <CardDetail
          card={card}
          expenses={await getCardExpenses(appUser.id, cardId, month.id)}
          invoice={await getInvoiceForCardMonth(appUser.id, cardId, month.id)}
          monthLabel={monthLabel}
        />
      )}
    </div>
  );
}
