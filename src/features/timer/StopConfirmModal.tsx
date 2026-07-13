import type { ActiveTimer, Project } from "@/types/domain";
import { elapsedSeconds } from "@/lib/duration";
import { amountForDuration } from "@/lib/money";
import { formatClock, formatCurrency } from "@/lib/format";
import { ACTIVITY_TYPE_LABELS } from "@/lib/labels";
import { useNow } from "@/hooks/useNow";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";

interface StopConfirmModalProps {
  open: boolean;
  timer: ActiveTimer;
  project: Project | null;
  busy: boolean;
  onCancel: () => void;
  onPause: () => void;
  onStop: () => void;
}

/**
 * Confirmacao antes de encerrar o cronometro (regra critica 8: nunca encerrar
 * tempo sem decisao consciente). Encerrar grava a sessao em `time_entries` e e
 * irreversivel — nao existe comando que reabra uma sessao encerrada.
 *
 * O erro mais provavel e querer pausar e encerrar por engano, entao o botao
 * primario e o Pausar. O tempo continua correndo enquanto o modal esta aberto:
 * o cronometro nao para, so a decisao e adiada.
 *
 * Apresentacional: nao chama o store. As acoes vem do TimerPanel.
 */
export function StopConfirmModal({
  open,
  timer,
  project,
  busy,
  onCancel,
  onPause,
  onStop,
}: StopConfirmModalProps) {
  const now = useNow(1000);
  const seconds = elapsedSeconds(timer, now);
  const amount = amountForDuration(seconds, project?.hourlyRateCents ?? 0);
  const running = timer.status === "running";

  return (
    <Modal
      open={open}
      title="Encerrar sessao?"
      onClose={onCancel}
      footer={
        <>
          <Button variant="ghost" onClick={onCancel} disabled={busy}>
            Cancelar
          </Button>
          <Button variant="danger" onClick={onStop} disabled={busy}>
            Encerrar mesmo assim
          </Button>
          {running && (
            <Button variant="primary" onClick={onPause} disabled={busy}>
              Pausar em vez disso
            </Button>
          )}
        </>
      }
    >
      <p className="text-sm text-text">
        {project ? project.name : "Projeto"}
        {project?.code ? ` · ${project.code}` : ""} ·{" "}
        {ACTIVITY_TYPE_LABELS[timer.activityType]}
      </p>

      <div className="my-5 flex items-baseline justify-center gap-3">
        <span className="tabular text-4xl font-semibold tracking-tight text-text">
          {formatClock(seconds)}
        </span>
        <span className="tabular text-sm text-text-muted">
          {formatCurrency(amount)}
        </span>
      </div>

      <p className="text-sm text-text-muted">
        Isso vira um registro definitivo no historico. Se voce so vai dar uma
        pausa, use Pausar — o cronometro continua de onde parou.
      </p>
    </Modal>
  );
}
