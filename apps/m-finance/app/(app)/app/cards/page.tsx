import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { CardManager } from "@/components/cards/card-manager";
import { CardFormDrawer } from "@/components/cards/card-form-drawer";
import { CardForecastGrid } from "@/components/cards/card-forecast-grid";
import { CardMonthSummary } from "@/components/cards/card-month-summary";
import { PageHeading } from "@/components/page-heading";
import { InvoiceFormDrawer } from "@/components/cards/invoice-form-drawer";
import { requireUser } from "@/lib/auth/guard";
import { getCreditCards } from "@/lib/cards";
import { getAppUserBySupabaseId, getMonthPartsAtOffset } from "@/lib/months";
import { getActiveMonthForUser, getActiveMonthParts, isViewingCurrentMonth } from "@/lib/active-month";
import { getCardLinesForMonth, getForecastInputs } from "@/lib/forecast";
import { formatMonthLabel } from "@/lib/formatters/date";

const shortMonth = new Intl.DateTimeFormat("pt-BR", { month: "short", year: "2-digit" });

export default async function CardsPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const currentMonth = appUser ? await getActiveMonthForUser(appUser.id) : null;
  const viewingCurrent = await isViewingCurrentMonth();
  // O mês da tela, exista ou não a linha dele: a fatura de um mês à frente
  // existe antes de alguém abrir esse mês.
  const active = await getActiveMonthParts();
  const cards = appUser ? await getCreditCards(appUser.id) : [];
  const inputs = appUser
    ? await getForecastInputs(appUser.id)
    : { cards: [], invoices: [], installments: [], incomes: [] };
  const lines = getCardLinesForMonth(inputs, active);
  const monthLabel = formatMonthLabel(new Date(active.year, active.month - 1, 1));
  const columns = Array.from({ length: 4 }, (_, index) => {
    const parts = getMonthPartsAtOffset(active.month, active.year, index + 1);
    const label = shortMonth.format(new Date(parts.year, parts.month - 1, 1)).replace(".", "");
    return {
      key: `${parts.year}-${parts.month}`,
      label: label.charAt(0).toUpperCase() + label.slice(1),
      lines: getCardLinesForMonth(inputs, parts),
    };
  });
  const activeCards = inputs.cards.filter((card) => card.isActive);
  const inactiveCards = inputs.cards.filter(
    (card) => !card.isActive && !lines.some((line) => line.card.id === card.id),
  );

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Cartões" title={`Faturas de ${monthLabel}`}>
        <div className="flex flex-wrap items-center gap-2">
          <CardFormDrawer />
          {currentMonth ? <InvoiceFormDrawer cards={cards} /> : null}
        </div>
      </PageHeading>

      {!currentMonth && viewingCurrent ? <CreateCurrentMonthCard /> : null}

      {lines.length > 0 ? <CardMonthSummary lines={lines} monthLabel={monthLabel} /> : null}

      <CardManager inactiveCards={inactiveCards} lines={lines} />

      <CardForecastGrid cards={activeCards} columns={columns} />
    </div>
  );
}
