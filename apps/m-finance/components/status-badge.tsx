import { TriangleMark } from "@/components/brand/triangle-mark";
import { Badge } from "@/components/ui/badge";

type Status = "positive" | "fair" | "tight" | "negative" | "pending" | "paid" | "overdue";

const statusMap: Record<Status, { label: string; className: string }> = {
  positive: {
    label: "Positivo",
    className: "border-status-positive/30 bg-status-positive/10 text-status-positive",
  },
  fair: {
    label: "Justo",
    className: "border-status-fair/30 bg-status-fair/10 text-status-fair",
  },
  tight: {
    label: "Apertado",
    className: "border-status-tight/30 bg-status-tight/10 text-status-tight",
  },
  negative: {
    label: "Negativo",
    className: "border-accent-border bg-accent-soft text-accent",
  },
  pending: {
    label: "Pendente",
    className: "border-border-default bg-background-elevated text-text-secondary",
  },
  paid: {
    label: "Pago",
    className: "border-status-positive/30 bg-status-positive/10 text-status-positive",
  },
  overdue: {
    label: "Vencido",
    className: "border-accent-border bg-accent-soft text-accent",
  },
};

export function StatusBadge({ status }: { status: Status }) {
  const config = statusMap[status];

  return (
    <Badge
      label={config.label}
      className={config.className}
      icon={<TriangleMark className="text-current" size={9} variant="solid" />}
    />
  );
}
