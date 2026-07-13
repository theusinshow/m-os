import { useState } from "react";
import { useTimerStore } from "@/stores/timerStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { elapsedSeconds } from "@/lib/duration";
import { formatDuration } from "@/lib/format";
import { useNow } from "@/hooks/useNow";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";

/**
 * Recuperacao de cronometro (secao 9). Aparece quando o app abre e encontra um
 * cronometro remanescente. Oferece manter, encerrar (registra a sessao) ou
 * descartar — nunca decide silenciosamente.
 */
export function RecoveryModal() {
  const recoveryPending = useTimerStore((s) => s.recoveryPending);
  const activeTimer = useTimerStore((s) => s.activeTimer);
  const dismissRecovery = useTimerStore((s) => s.dismissRecovery);
  const stop = useTimerStore((s) => s.stop);
  const discard = useTimerStore((s) => s.discard);
  const projects = useCatalogStore((s) => s.projects);
  const now = useNow(1000);

  const [busy, setBusy] = useState(false);

  if (!recoveryPending || !activeTimer) return null;

  const project = projects.find((p) => p.id === activeTimer.projectId) ?? null;
  const elapsed = elapsedSeconds(activeTimer, now);

  async function act(action: () => Promise<void> | void) {
    setBusy(true);
    try {
      await action();
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal
      open
      title="Cronometro recuperado"
      onClose={dismissRecovery}
      footer={
        <>
          <Button
            variant="danger"
            onClick={() => void act(discard)}
            disabled={busy}
          >
            Descartar
          </Button>
          <Button
            variant="secondary"
            onClick={() => void act(stop)}
            disabled={busy}
          >
            Encerrar registro
          </Button>
          <Button
            variant="primary"
            onClick={() => act(dismissRecovery)}
            disabled={busy}
          >
            Manter em execucao
          </Button>
        </>
      }
    >
      <p className="text-sm text-text">
        Havia um cronometro em andamento quando o aplicativo foi fechado.
      </p>
      <div className="mt-4 rounded border border-border bg-bg px-4 py-3 text-sm">
        <div className="flex justify-between">
          <span className="text-text-muted">Projeto</span>
          <span className="text-text">{project?.name ?? "—"}</span>
        </div>
        <div className="mt-1.5 flex justify-between">
          <span className="text-text-muted">Tempo decorrido</span>
          <span className="tabular text-text">{formatDuration(elapsed)}</span>
        </div>
      </div>
      <p className="mt-4 text-xs text-text-subtle">
        Manter continua a contagem. Encerrar registra a sessao no historico.
        Descartar remove sem registrar.
      </p>
    </Modal>
  );
}
