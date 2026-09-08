import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatMonthLabel } from "@/lib/formatters/date";
import type { ProjectionRow } from "@/lib/calculations/projection";

/**
 * O mês atual e os seguintes lado a lado.
 *
 * O app respondia só sobre o mês da tela; para saber se outubro fechava era
 * preciso navegar até outubro. Com a receita lançada no mês em que ela chega,
 * a resposta dos próximos meses cabe numa lista.
 */
export function ProjectionCard({ rows }: { rows: ProjectionRow[] }) {
  return (
    <DashboardCard
      description="Receita lançada menos o que já está comprometido, mês a mês."
      title="Meses à frente"
    >
      <ul className="space-y-2">
        {rows.map((row) => {
          const label = formatMonthLabel(new Date(row.year, row.month - 1, 1));
          const negative = row.remainingCents < 0;

          return (
            <li
              className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 rounded-lg border border-border-subtle bg-background-elevated px-4 py-3"
              key={`${row.year}-${row.month}`}
            >
              <div className="min-w-0">
                <p className="font-medium text-text-primary">
                  {label}
                  {row.isCurrent ? (
                    <span className="ml-2 rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                      Atual
                    </span>
                  ) : null}
                </p>
                <p className="num mt-0.5 text-xs text-text-muted">
                  {row.hasIncome
                    ? `${formatCurrency(row.incomeCents)} de receita`
                    : "receita não lançada"}
                  {" · "}
                  {formatCurrency(row.committedCents)} comprometidos
                </p>
              </div>
              <div className="text-right">
                {row.hasIncome ? (
                  <>
                    <p
                      className={`num text-lg font-semibold ${
                        negative ? "text-accent" : "text-status-positive"
                      }`}
                    >
                      {formatCurrency(row.remainingCents)}
                    </p>
                    <p className="mt-0.5 text-xs text-text-muted">
                      {negative ? "falta" : "sobra"}
                    </p>
                  </>
                ) : (
                  <p className="text-sm text-text-muted">sem resposta ainda</p>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </DashboardCard>
  );
}
