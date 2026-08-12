import type { TimeEntry } from "@/types/domain";
import {
  formatCurrency,
  formatDate,
  formatDuration,
  formatTime,
} from "@/lib/format";
import { amountForDuration } from "@/lib/money";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";

interface DeleteEntryModalProps {
  open: boolean;
  entry: TimeEntry | null;
  projectName: string;
  busy?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-4 py-1.5">
      <span className="text-xs text-text-muted">{label}</span>
      <span className="tabular text-sm text-text">{value}</span>
    </div>
  );
}

/**
 * Confirmacao de exclusao de sessao. Mostra o que sai da conta antes de
 * remover: nunca descartar tempo silenciosamente (regra critica 8).
 */
export function DeleteEntryModal({
  open,
  entry,
  projectName,
  busy = false,
  onCancel,
  onConfirm,
}: DeleteEntryModalProps) {
  if (!entry) return null;

  const amount = amountForDuration(
    entry.durationSeconds - entry.idleSeconds,
    entry.hourlyRateSnapshotCents,
  );
  const periodo = entry.endedAt
    ? `${formatTime(entry.startedAt)}–${formatTime(entry.endedAt)}`
    : formatTime(entry.startedAt);

  return (
    <Modal
      open={open}
      title="Excluir sessao"
      onClose={onCancel}
      footer={
        <>
          <Button variant="ghost" onClick={onCancel} type="button">
            Cancelar
          </Button>
          <Button
            variant="danger"
            onClick={onConfirm}
            type="button"
            disabled={busy}
          >
            {busy ? "Excluindo…" : "Excluir"}
          </Button>
        </>
      }
    >
      <div className="divide-y divide-border">
        <Row label="Projeto" value={projectName} />
        <Row label="Data" value={formatDate(entry.startedAt)} />
        <Row label="Periodo" value={periodo} />
        <Row label="Duracao" value={formatDuration(entry.durationSeconds)} />
        <Row label="Valor" value={formatCurrency(amount)} />
      </div>
      <p className="mt-4 text-xs text-text-muted">
        A sessao sai das telas e dos relatorios, mas continua guardada: da para
        restaurar depois em Historico, marcando “Mostrar excluidas”.
      </p>
    </Modal>
  );
}
