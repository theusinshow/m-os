import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

// Generic pill used by StatusBadge, RiskBadge and PriorityBadge. Each of those
// owns its own label/className map and passes an optional leading icon.
export function Badge({
  label,
  className,
  icon,
}: {
  label: string;
  className: string;
  icon?: ReactNode;
}) {
  return (
    <span
      className={cn(
        "inline-flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-xs font-semibold",
        className,
      )}
    >
      {icon}
      {label}
    </span>
  );
}
