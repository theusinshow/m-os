import { AlertTriangle, Check, RefreshCw, ShieldCheck } from "lucide-react";
import { acknowledgeInsight, refreshInsights, resolveInsight } from "@/app/actions/intelligence";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { InlineEmpty } from "@/components/ui/inline-empty";
import type { DashboardIntelligence } from "@/lib/finance-intelligence/dashboard";
import { formatCurrency } from "@/lib/formatters/currency";
import { cn } from "@/lib/utils";

const severityClass = {
  info: "border-border-subtle bg-background-elevated",
  warning: "border-status-fair/30 bg-status-fair/10",
  critical: "border-accent-border bg-accent-soft",
} as const;

const severityIconClass = {
  info: "text-text-muted",
  warning: "text-status-fair",
  critical: "text-accent",
} as const;

/**
 * Safe-to-Spend e insights, lado a lado.
 *
 * O Safe-to-Spend nunca aparece sozinho: a sobra contábil fica ao lado e cada
 * dedução diz de onde veio. Um número que a pessoa não consegue reconstruir
 * não serve para decidir — e ela precisa poder discordar da premissa.
 */
export function IntelligencePanel({ data }: { data: DashboardIntelligence }) {
  const { safeToSpend: safe, insights } = data;
  const negative = safe.safeToSpendCents < 0;

  return (
    <section className="grid scroll-mt-24 gap-4 xl:grid-cols-[0.9fr_1.1fr]" id="insights">
      <DashboardCard
        title="Pode gastar com segurança"
        description="Depois das margens e do que os próximos meses já pedem."
      >
        {safe.status === "unknown" ? (
          <InlineEmpty>Sem receita lançada ou estimada neste mês — não dá para calcular.</InlineEmpty>
        ) : (
          <>
            <p className={cn("num text-3xl font-semibold", negative ? "text-accent" : "text-text-primary")}>
              {formatCurrency(safe.safeToSpendCents)}
            </p>
            <dl className="mt-4 space-y-2 text-sm">
              <div className="flex items-center justify-between gap-4">
                <dt className="text-text-muted">Sobra contábil</dt>
                <dd className="num text-text-secondary">{formatCurrency(safe.accountingRemainingCents)}</dd>
              </div>
              {safe.deductions.map((deduction) => (
                <div className="flex items-start justify-between gap-4" key={deduction.reason}>
                  <dt className="text-text-muted">
                    − {deduction.label}
                    <span className="mt-0.5 block text-xs leading-5 text-text-muted/80">{deduction.detail}</span>
                  </dt>
                  <dd className="num shrink-0 text-text-secondary">{formatCurrency(deduction.amountCents)}</dd>
                </div>
              ))}
            </dl>
            {safe.assumptions.length > 0 ? (
              <ul className="mt-4 space-y-1 border-t border-border-subtle pt-3 text-xs leading-5 text-text-muted">
                {safe.assumptions.map((assumption) => (
                  <li key={assumption}>· {assumption}</li>
                ))}
              </ul>
            ) : null}
            <a
              className="focus-ring mt-4 inline-flex rounded text-xs font-medium text-accent underline underline-offset-4"
              href="/app/settings#politicas"
            >
              Ajustar margens e regras
            </a>
          </>
        )}
      </DashboardCard>

      <DashboardCard
        title="O que mudou"
        description="Achado por regra, não por palpite. Some sozinho quando deixa de valer."
        action={
          <ToastForm action={refreshInsights} successMessage="Insights reavaliados.">
            <FormSubmitButton pendingLabel="Reavaliando..." variant="secondary">
              <RefreshCw aria-hidden="true" size={14} />
              Reavaliar
            </FormSubmitButton>
          </ToastForm>
        }
      >
        <div className="space-y-3">
          {insights.length === 0 ? (
            <InlineEmpty>
              <span className="inline-flex items-center gap-2">
                <ShieldCheck aria-hidden="true" size={15} />
                Nada material agora.
              </span>
            </InlineEmpty>
          ) : (
            insights.map((insight) => (
              <div
                className={cn(
                  "rounded-md border p-4",
                  severityClass[insight.severity],
                  insight.status === "acknowledged" && "opacity-70",
                )}
                key={insight.id}
              >
                <div className="flex items-start gap-3">
                  <AlertTriangle
                    aria-hidden="true"
                    className={cn("mt-0.5 shrink-0", severityIconClass[insight.severity])}
                    size={16}
                  />
                  <div className="min-w-0 flex-1">
                    <p className="text-sm font-semibold text-text-primary">{insight.title}</p>
                    <p className="mt-1 text-sm text-text-muted">{insight.narrative ?? insight.summary}</p>
                    <div className="mt-3 flex flex-wrap gap-2">
                      {insight.status === "open" ? (
                        <ToastForm action={acknowledgeInsight} successMessage="Anotado. Só volta se piorar.">
                          <input name="insightId" type="hidden" value={insight.id} />
                          <FormSubmitButton pendingLabel="..." variant="secondary">
                            Vi
                          </FormSubmitButton>
                        </ToastForm>
                      ) : null}
                      <ToastForm action={resolveInsight} successMessage="Resolvido.">
                        <input name="insightId" type="hidden" value={insight.id} />
                        <FormSubmitButton pendingLabel="..." variant="secondary">
                          <Check aria-hidden="true" size={14} />
                          Resolvido
                        </FormSubmitButton>
                      </ToastForm>
                    </div>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </DashboardCard>
    </section>
  );
}
