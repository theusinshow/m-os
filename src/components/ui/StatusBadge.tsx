import { cn } from "@/lib/cn";
import type { TimerStatus } from "@/types/domain";
import { TIMER_STATUS_LABELS } from "@/lib/labels";

interface StatusBadgeProps {
  status: TimerStatus | "stopped";
}

const STYLES: Record<StatusBadgeProps["status"], string> = {
  running: "text-running",
  paused: "text-paused",
  stopped: "text-stopped",
};

const LABELS: Record<StatusBadgeProps["status"], string> = {
  running: TIMER_STATUS_LABELS.running,
  paused: TIMER_STATUS_LABELS.paused,
  stopped: "Parado",
};

/** Indicador de estado do cronometro com ponto colorido. */
export function StatusBadge({ status }: StatusBadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 text-xs font-medium",
        STYLES[status],
      )}
    >
      <span
        className={cn(
          "h-2 w-2 rounded-full bg-current",
          status === "running" && "animate-pulse",
        )}
        aria-hidden
      />
      {LABELS[status]}
    </span>
  );
}
