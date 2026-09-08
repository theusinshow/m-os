import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatMonthLabel } from "@/lib/formatters/date";
import type { InstallmentSeries } from "@/lib/calculations/commitments";

function endLabel(isoDate: string) {
  const date = new Date(`${isoDate}T12:00:00`);
  return formatMonthLabel(date);
}

/**
 * O que já está comprometido depois deste mês.
 *
 * Um parcelamento aparecia só como a parcela do mês corrente; a dívida inteira
 * não tinha lugar em tela nenhuma.
 */
export function CommitmentsCard({ series }: { series: InstallmentSeries[] }) {
  const totalRemainingCents = series.reduce((total, item) => total + item.remainingCents, 0);
  const monthlyCents = series.reduce((total, item) => total + item.installmentCents, 0);

  return (
    <DashboardCard
      description="Parcelamentos em aberto — o que este mês já promete aos próximos."
      title="Compromisso futuro"
    >
      {series.length === 0 ? (
        <InlineEmpty>Nenhum parcelamento em aberto.</InlineEmpty>
      ) : (
        <div className="space-y-4">
          <div className="flex flex-wrap items-end gap-x-8 gap-y-2">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                Falta pagar ao todo
              </p>
              <p className="num mt-1 text-2xl font-semibold text-text-primary">
                {formatCurrency(totalRemainingCents)}
              </p>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                Peso por mês
              </p>
              <p className="num mt-1 text-2xl font-semibold text-text-primary">
                {formatCurrency(monthlyCents)}
              </p>
            </div>
          </div>

          <ul className="space-y-2">
            {series.map((item) => (
              <li
                className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 rounded-lg border border-border-subtle bg-background-elevated px-4 py-3"
                key={item.seriesId}
              >
                <div>
                  <p className="text-sm font-semibold text-text-primary">{item.name}</p>
                  <p className="mt-0.5 text-xs text-text-muted">
                    {item.paidCount} de {item.seriesTotal} pagas · última em{" "}
                    {endLabel(item.lastDueDate)}
                  </p>
                </div>
                <div className="text-right">
                  <p className="num text-sm font-semibold text-text-primary">
                    {formatCurrency(item.remainingCents)}
                  </p>
                  <p className="num mt-0.5 text-xs text-text-muted">
                    {item.remainingCount} × {formatCurrency(item.installmentCents)}
                  </p>
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
    </DashboardCard>
  );
}
