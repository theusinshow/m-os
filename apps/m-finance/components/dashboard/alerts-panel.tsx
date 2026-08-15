import { AlertTriangle } from "lucide-react";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import type { InternalAlert } from "@/lib/calculations/alerts";
import { cn } from "@/lib/utils";

const severityClass = {
  info: "border-border-subtle bg-background-elevated",
  warning: "border-status-fair/30 bg-status-fair/10",
  danger: "border-accent-border bg-accent-soft",
};

export function AlertsPanel({ alerts }: { alerts: InternalAlert[] }) {
  return (
    <DashboardCard title="Alertas">
      <div className="space-y-3">
        {alerts.length === 0 ? (
          <div className="rounded-md border border-border-subtle bg-background-elevated px-4 py-3 text-sm text-text-muted">
            Nenhum alerta ativo para este mês.
          </div>
        ) : (
          alerts.map((alert) => (
            <div
              className={cn("rounded-md border p-4", severityClass[alert.severity])}
              key={alert.id}
            >
              <div className="flex items-start gap-3">
                <AlertTriangle className="mt-0.5 shrink-0 text-accent" size={16} />
                <div>
                  <p className="text-sm font-semibold text-text-primary">{alert.title}</p>
                  <p className="mt-1 text-sm text-text-muted">{alert.message}</p>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </DashboardCard>
  );
}
