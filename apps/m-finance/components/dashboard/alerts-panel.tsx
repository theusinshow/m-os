import { AlertTriangle } from "lucide-react";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { InlineEmpty } from "@/components/ui/inline-empty";
import type { InternalAlert } from "@/lib/calculations/alerts";
import { cn } from "@/lib/utils";

const severityClass = {
  info: "border-border-subtle bg-background-elevated",
  warning: "border-status-fair/30 bg-status-fair/10",
  danger: "border-accent-border bg-accent-soft",
};

const severityIconClass = {
  info: "text-text-muted",
  warning: "text-status-fair",
  danger: "text-accent",
};

export function AlertsPanel({ alerts }: { alerts: InternalAlert[] }) {
  return (
    <DashboardCard title="Alertas">
      <div className="space-y-3">
        {alerts.length === 0 ? (
          <InlineEmpty>Nenhum alerta ativo. Não há contas vencendo nos próximos dias.</InlineEmpty>
        ) : (
          alerts.map((alert) => (
            <div
              className={cn("rounded-md border p-4", severityClass[alert.severity])}
              key={alert.id}
            >
              <div className="flex items-start gap-3">
                <AlertTriangle
                  className={cn("mt-0.5 shrink-0", severityIconClass[alert.severity])}
                  size={16}
                />
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
