import { TriangleMark } from "@/components/brand/triangle-mark";
import { Badge } from "@/components/ui/badge";
import type { RiskLevel } from "@/db/schema";

const riskMap: Record<RiskLevel, { label: string; className: string }> = {
  safe: {
    label: "Seguro",
    className: "border-status-positive/30 bg-status-positive/10 text-status-positive",
  },
  controlled: {
    label: "Controlado",
    className: "border-status-fair/30 bg-status-fair/10 text-status-fair",
  },
  tight: {
    label: "Apertado",
    className: "border-status-tight/30 bg-status-tight/10 text-status-tight",
  },
  critical: {
    label: "Crítico",
    className: "border-accent-border bg-accent-soft text-accent",
  },
};

export function RiskBadge({ risk }: { risk: RiskLevel }) {
  const config = riskMap[risk];

  return (
    <Badge
      label={config.label}
      className={config.className}
      icon={<TriangleMark className="text-current" size={9} variant="solid" />}
    />
  );
}
