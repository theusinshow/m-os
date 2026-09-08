import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatMonthLabel } from "@/lib/formatters/date";
import type { CardInstallmentSeries } from "@/lib/calculations/card-installments";

/**
 * Os parcelamentos vivos do cartão. Cada linha responde "em que parcela estou"
 * e "quanto ainda falta" — o que a lista de compras solta nunca disse.
 */
export function CardInstallmentsCard({ series }: { series: CardInstallmentSeries[] }) {
  const remainingCents = series.reduce((total, item) => total + item.remainingCents, 0);
  const monthlyCents = series.reduce((total, item) => total + item.installmentCents, 0);

  return (
    <DashboardCard
      description="O que este cartão já promete aos próximos meses."
      title="Parcelamentos em aberto"
    >
      {series.length === 0 ? (
        <InlineEmpty>Nenhum parcelamento em aberto neste cartão.</InlineEmpty>
      ) : (
        <div className="space-y-4">
          <div className="flex flex-wrap items-end gap-x-8 gap-y-2">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                Falta pagar
              </p>
              <p className="num mt-1 text-2xl font-semibold text-text-primary">
                {formatCurrency(remainingCents)}
              </p>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                Peso neste mês
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
                key={item.installmentId}
              >
                <div className="min-w-0">
                  <p className="truncate font-medium text-text-primary">{item.description}</p>
                  <p className="mt-0.5 text-xs text-text-muted">
                    Parcela {item.currentNumber}/{item.installmentTotal}
                    {item.remainingCount > 0
                      ? ` · última em ${formatMonthLabel(
                          new Date(item.lastMonth.year, item.lastMonth.month - 1, 1),
                        )}`
                      : " · última parcela"}
                  </p>
                </div>
                <div className="text-right">
                  <p className="num text-sm font-semibold text-text-primary">
                    {formatCurrency(item.installmentCents)}
                  </p>
                  <p className="num mt-0.5 text-xs text-text-muted">
                    {item.remainingCount > 0
                      ? `faltam ${item.remainingCount} × ${formatCurrency(item.installmentCents)}`
                      : "quitando"}
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
