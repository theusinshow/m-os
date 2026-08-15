import { Badge } from "@/components/ui/badge";
import type { GoalPriority } from "@/db/schema";

const priorityMap: Record<GoalPriority, { label: string; className: string }> = {
  low: {
    label: "Baixa",
    className: "border-border-default bg-background-elevated text-text-muted",
  },
  medium: {
    label: "Média",
    className: "border-status-fair/30 bg-status-fair/10 text-status-fair",
  },
  high: {
    label: "Alta",
    className: "border-accent-border bg-accent-soft text-accent",
  },
};

export function PriorityBadge({ priority }: { priority: GoalPriority }) {
  const config = priorityMap[priority];

  return <Badge label={config.label} className={config.className} />;
}
