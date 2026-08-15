import { deleteSimulation } from "@/app/actions/simulations";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { RiskBadge } from "@/components/simulator/risk-badge";
import { StatusBadge } from "@/components/status-badge";
import { ToastForm } from "@/components/toast-form";
import { EmptyState } from "@/components/empty-state";
import { monthName } from "@/lib/calculations/simulator";
import { formatCurrency } from "@/lib/formatters/currency";
import type { StoredSimulation } from "@/lib/simulations";

function capitalize(value: string) {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

export function SimulationList({ simulations }: { simulations: StoredSimulation[] }) {
  if (simulations.length === 0) {
    return (
      <EmptyState
        title="Nenhuma simulação ainda"
        description="Simule uma compra acima para ver o impacto projetado nos próximos meses e guardar o resultado aqui."
      />
    );
  }

  return (
    <div className="space-y-4">
      {simulations.map((simulation) => (
        <DashboardCard key={simulation.id}>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <h3 className="text-lg font-semibold text-text-primary">{simulation.name}</h3>
                <RiskBadge risk={simulation.riskLevel} />
              </div>
              <p className="mt-1 text-sm text-text-muted">
                {formatCurrency(simulation.totalAmountCents)} ·{" "}
                {simulation.paymentType === "installment"
                  ? `${simulation.installments}x de ${formatCurrency(simulation.monthlyImpactCents)}`
                  : "à vista"}{" "}
                · início em {capitalize(monthName(simulation.startMonth))}/{simulation.startYear}
              </p>
            </div>
          </div>

          <p className="mt-4 rounded-lg border border-border-subtle bg-background-elevated p-4 text-sm leading-6 text-text-secondary">
            {simulation.recommendation}
          </p>

          <div className="mt-4 space-y-2">
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
              Impacto por mês
            </p>
            <div className="grid gap-2 sm:grid-cols-2">
              {simulation.result.months.map((month) => (
                <div
                  className="flex items-center justify-between gap-3 rounded-md border border-border-subtle bg-background-elevated px-3 py-2.5"
                  key={`${month.year}-${month.month}`}
                >
                  <div>
                    <p className="text-sm font-medium text-text-primary">
                      {capitalize(monthName(month.month))}/{month.year}
                    </p>
                    <p className="num mt-0.5 text-xs text-text-muted">
                      sobra {formatCurrency(month.remainingWithCents)} · −
                      {formatCurrency(month.impactCents)}
                    </p>
                  </div>
                  <StatusBadge status={month.health} />
                </div>
              ))}
            </div>
          </div>

          <div className="mt-4 flex justify-end">
            <ToastForm action={deleteSimulation} successMessage="Simulação excluída.">
              <input name="simulationId" type="hidden" value={simulation.id} />
              <ConfirmDeleteButton confirmMessage="Excluir esta simulação?">
                Excluir simulação
              </ConfirmDeleteButton>
            </ToastForm>
          </div>
        </DashboardCard>
      ))}
    </div>
  );
}
