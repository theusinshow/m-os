import type { ActiveTimer, Project } from "@/types/domain";
import { elapsedSeconds } from "@/lib/duration";
import { amountForDuration } from "@/lib/money";
import { formatClock, formatCurrency } from "@/lib/format";
import { ACTIVITY_TYPE_LABELS } from "@/lib/labels";
import { useNow } from "@/hooks/useNow";
import { StatusBadge } from "@/components/ui/StatusBadge";

interface TimerCardProps {
  timer: ActiveTimer | null;
  project: Project | null;
}

/**
 * Exibicao do cronometro (apresentacional): projeto, status, tempo decorrido e
 * valor estimado. Os controles ficam no TimerPanel. O tempo e recalculado a
 * cada segundo a partir dos timestamps persistidos (a fonte da verdade e o
 * backend), sem acumular erro de contador.
 */
export function TimerCard({ timer, project }: TimerCardProps) {
  const now = useNow(1000);

  const seconds = timer ? elapsedSeconds(timer, now) : 0;
  const rate = project?.hourlyRateCents ?? 0;
  const estimate = amountForDuration(seconds, rate);

  return (
    <div>
      <div className="flex items-center justify-between">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-text">
            {project ? project.name : "Nenhum projeto selecionado"}
          </p>
          <p className="mt-0.5 text-xs text-text-muted">
            {project?.code ? `${project.code} · ` : ""}
            {timer
              ? ACTIVITY_TYPE_LABELS[timer.activityType]
              : "Sem cronometro ativo"}
          </p>
        </div>
        <StatusBadge status={timer ? timer.status : "stopped"} />
      </div>

      <div className="my-8 text-center">
        <div className="tabular text-6xl font-semibold tracking-tight text-text">
          {formatClock(seconds)}
        </div>
        {timer?.description && (
          <p className="mt-2 text-sm text-text-muted">{timer.description}</p>
        )}
      </div>

      <div className="flex items-center justify-end">
        <div className="text-right">
          <p className="text-2xs uppercase tracking-wide text-text-subtle">
            Valor estimado
          </p>
          <p className="tabular text-lg font-semibold text-text">
            {formatCurrency(estimate)}
          </p>
        </div>
      </div>
    </div>
  );
}
