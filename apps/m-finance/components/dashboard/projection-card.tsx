import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { NfQuickForm } from "@/components/dashboard/nf-quick-form";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatMonthLabel } from "@/lib/formatters/date";
import type { ProjectionRow } from "@/lib/calculations/projection";

function money(cents: number, estimated: boolean) {
  return `${estimated ? "≈ " : ""}${formatCurrency(cents)}`;
}

/**
 * Quanto entra e quanto sai, mês a mês.
 *
 * A pergunta "quanto vou receber em novembro" antes tinha resposta só se a NF
 * já estivesse lançada; e a de "quanto vou pagar", só se a fatura estivesse.
 * Agora cada mês mostra a NF e os cartões — lançados ou estimados, e a linha
 * diz qual — e a NF de um mês à frente se lança dali mesmo.
 */
export function ProjectionCard({ rows }: { rows: ProjectionRow[] }) {
  return (
    <DashboardCard
      description="NF e faturas lançadas aparecem cheias; o que ainda é estimado aparece com ≈."
      title="Quanto entra e quanto sai"
    >
      <ul className="space-y-2">
        {rows.map((row) => {
          const label = formatMonthLabel(new Date(row.year, row.month - 1, 1));
          const negative = row.remainingCents < 0;
          const monthValue = `${row.year}-${String(row.month).padStart(2, "0")}`;
          const nfEstimated = row.incomeEstimatedCents > 0;
          const nfMissing = !row.hasIncome || nfEstimated;

          return (
            <li
              className="rounded-lg border border-border-subtle bg-background-elevated px-4 py-3"
              key={monthValue}
            >
              <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
                <div className="min-w-0">
                  <p className="font-medium text-text-primary">
                    {label}
                    {row.isCurrent ? (
                      <span className="ml-2 rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                        Atual
                      </span>
                    ) : null}
                  </p>
                  <dl className="num mt-1 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-xs text-text-muted">
                    <dt>Entra</dt>
                    <dd className={nfEstimated ? "" : "text-text-secondary"}>
                      {row.hasIncome ? money(row.incomeCents, nfEstimated) : "NF não lançada"}
                      {nfEstimated ? " (NF estimada)" : ""}
                    </dd>
                    <dt>Cartões</dt>
                    <dd className={row.invoicesEstimatedCents > 0 ? "" : "text-text-secondary"}>
                      {money(row.invoicesCents, row.invoicesEstimatedCents > 0)}
                      {row.invoicesEstimatedCents > 0 &&
                      row.invoicesEstimatedCents < row.invoicesCents
                        ? ` (${formatCurrency(row.invoicesEstimatedCents)} estimado)`
                        : ""}
                    </dd>
                    <dt>Contas</dt>
                    <dd className="text-text-secondary">{formatCurrency(row.billsCents)}</dd>
                  </dl>
                </div>
                <div className="text-right">
                  {row.hasIncome ? (
                    <>
                      <p
                        className={`num text-lg font-semibold ${
                          negative ? "text-accent" : "text-status-positive"
                        }`}
                      >
                        {money(row.remainingCents, row.isEstimated)}
                      </p>
                      <p className="mt-0.5 text-xs text-text-muted">{negative ? "falta" : "sobra"}</p>
                    </>
                  ) : (
                    <p className="text-sm text-text-muted">sem resposta ainda</p>
                  )}
                </div>
              </div>

              {nfMissing ? (
                <details className="group mt-3 border-t border-border-subtle pt-3">
                  <summary className="focus-ring cursor-pointer rounded-md text-xs font-semibold text-text-secondary [&::-webkit-details-marker]:hidden">
                    Lançar NF de {label}
                  </summary>
                  <div className="mt-3">
                    <NfQuickForm
                      monthLabel={label}
                      monthValue={monthValue}
                      suggestedCents={row.incomeEstimatedCents}
                    />
                  </div>
                </details>
              ) : null}
            </li>
          );
        })}
      </ul>
    </DashboardCard>
  );
}
