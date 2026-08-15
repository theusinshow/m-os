import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { cn } from "@/lib/utils";

export function FinancialSummaryCard({
  description,
  label,
  tone = "default",
  value,
}: {
  description: string;
  label: string;
  tone?: "default" | "danger";
  value: string;
}) {
  return (
    <DashboardCard className="group relative h-full overflow-hidden">
      <div
        className={cn(
          "absolute inset-x-0 top-0 h-0.5",
          tone === "danger" ? "bg-accent" : "bg-status-positive/70",
        )}
      />
      <div className="flex items-center gap-2">
        <TriangleMark
          className={cn(
            "transition-transform duration-300 group-hover:-translate-y-0.5",
            tone === "danger" ? "text-accent" : "text-status-positive/80",
          )}
          size={10}
          variant="solid"
        />
        <p className="text-sm font-medium text-text-muted">{label}</p>
      </div>
      <p
        className={cn(
          "num mt-3 text-3xl font-semibold text-text-primary",
          tone === "danger" && "text-accent",
        )}
      >
        {value}
      </p>
      <p className="mt-2 text-sm text-text-muted">{description}</p>
    </DashboardCard>
  );
}
