import { AlertTriangle } from "lucide-react";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { formatCurrency } from "@/lib/formatters/currency";
import type { InvoiceBreakdown } from "@/lib/calculations/invoice-breakdown";

/**
 * A fatura repartida por origem, com o não classificado como uma fatia
 * explícita. É a resposta para "de onde vieram esses R$ 1.027" sem obrigar
 * ninguém a digitar quarenta compras.
 */
export function InvoiceBreakdownCard({ breakdown }: { breakdown: InvoiceBreakdown }) {
  const { totalCents, classifiedCents, unclassifiedCents, isOverclassified, slices } = breakdown;
  const classifiedPercent =
    totalCents === 0 ? 0 : Math.min(Math.round((classifiedCents / totalCents) * 100), 100);

  return (
    <DashboardCard
      description="O total é o que o cartão cobra. As compras lançadas explicam parte dele — o resto fica em Outros."
      title="De onde veio a fatura"
    >
      {slices.length === 0 ? (
        <p className="text-sm text-text-muted">
          Sem fatura neste mês. Lance o total na tela de Cartões ou uma compra aqui embaixo.
        </p>
      ) : (
        <div className="space-y-4">
          <div className="flex flex-wrap items-end gap-x-8 gap-y-2">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                Fatura
              </p>
              <p className="num mt-1 text-2xl font-semibold text-text-primary">
                {formatCurrency(totalCents)}
              </p>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                Classificado · {classifiedPercent}%
              </p>
              <p className="num mt-1 text-2xl font-semibold text-text-primary">
                {formatCurrency(classifiedCents)}
              </p>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                Em Outros
              </p>
              <p className="num mt-1 text-2xl font-semibold text-text-secondary">
                {formatCurrency(unclassifiedCents)}
              </p>
            </div>
          </div>

          {isOverclassified ? (
            <p className="flex items-start gap-2 rounded-lg border border-accent-border bg-accent-soft px-4 py-3 text-sm leading-6 text-accent">
              <AlertTriangle aria-hidden="true" className="mt-0.5 shrink-0" size={16} />
              As compras lançadas somam mais do que o total da fatura. Confira o total na tela de
              Cartões ou algum lançamento repetido.
            </p>
          ) : null}

          <ul className="space-y-2">
            {slices.map((slice) => (
              <li key={slice.name}>
                <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-0.5">
                  <span
                    className={
                      slice.isRemainder
                        ? "text-sm text-text-muted"
                        : "text-sm font-medium text-text-primary"
                    }
                  >
                    {slice.name}
                  </span>
                  <span className="num text-sm text-text-secondary">
                    {formatCurrency(slice.valueCents)} · {slice.percent}%
                  </span>
                </div>
                <div
                  aria-hidden="true"
                  className="mt-1.5 h-2 overflow-hidden rounded-full bg-background-elevated"
                >
                  <span
                    className={`block h-full rounded-full ${
                      slice.isRemainder ? "bg-text-muted/40" : "bg-accent/70"
                    }`}
                    style={{ width: `${Math.max(slice.percent, 1)}%` }}
                  />
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
    </DashboardCard>
  );
}
