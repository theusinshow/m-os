import { useState } from "react";
import type { TimeEntry } from "@/types/domain";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import {
  clampQuickSeconds,
  QUICK_INCREMENTS,
  resolveQuickEntryWindow,
} from "@/lib/quickTime";
import { recentProjectIds } from "@/lib/projects";
import { isoToDateInput } from "@/lib/datetime";
import { formatDuration } from "@/lib/format";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Field, Input, Select } from "@/components/ui/Field";

export interface QuickTimeModalProps {
  open: boolean;
  onClose: () => void;
  /** Sessao ancora: trava projeto e dia, e cola o ajuste logo antes dela. */
  anchor?: TimeEntry | null;
  /** Projeto pre-selecionado quando nao ha ancora. */
  defaultProjectId?: string;
}

/**
 * Registra tempo esquecido por duracao, nao por horario.
 *
 * O `EntryForm` ja cria sessoes manuais, mas exige inicio e fim em
 * `datetime-local`. Quem esqueceu de ligar o cronometro nao lembra do horario —
 * lembra da duracao. Este modal e o caminho curto: projeto, total, dia, nota.
 *
 * Com `anchor`, o tempo vira um registro **separado** colado logo antes da
 * sessao indicada; a sessao original nunca e alterada (regra critica 5), para
 * o historico continuar distinguindo o cronometrado do estimado.
 */
export function QuickTimeModal({
  open,
  onClose,
  anchor,
  defaultProjectId,
}: QuickTimeModalProps) {
  const projects = useCatalogStore((s) => s.projects);
  const entries = useEntriesStore((s) => s.entries);
  const create = useEntriesStore((s) => s.create);

  const [projectId, setProjectId] = useState("");
  const [seconds, setSeconds] = useState(0);
  const [day, setDay] = useState("");
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Mesmo padrao de reinicializacao do EntryForm.tsx:51-76.
  const [initializedFor, setInitializedFor] = useState<string | null>(null);
  const key = anchor?.id ?? `new-${defaultProjectId ?? ""}`;
  if (open && initializedFor !== key) {
    const mostRecentProjectId = recentProjectIds(entries, projects, 1)[0] ?? null;
    setProjectId(
      anchor?.projectId ?? defaultProjectId ?? mostRecentProjectId ?? "",
    );
    setDay(
      isoToDateInput(anchor?.startedAt ?? new Date().toISOString()),
    );
    setSeconds(0);
    setNote("");
    setError(null);
    setInitializedFor(key);
  }
  if (!open && initializedFor !== null) setInitializedFor(null);

  const locked = Boolean(anchor);
  const anchorProjectName =
    projects.find((p) => p.id === anchor?.projectId)?.name ?? "Projeto";

  function add(delta: number) {
    setSeconds((s) => clampQuickSeconds(s + delta));
  }

  async function handleSave() {
    setSaving(true);
    setError(null);
    try {
      if (!projectId) throw "Selecione um projeto.";
      if (!day) throw "Escolha o dia.";
      const { startedAt, endedAt } = resolveQuickEntryWindow({
        durationSeconds: seconds,
        day,
        anchorAtIso: anchor?.startedAt ?? null,
        dayEntries: entries,
        now: new Date(),
      });
      await create({
        projectId,
        startedAt,
        endedAt,
        description: note || null,
        activityType: "drawing",
        billable: true,
        idleSeconds: 0,
        source: "manual",
      });
      onClose();
    } catch (err) {
      setError(
        typeof err === "string" ? err : "Falha ao adicionar o tempo.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      open={open}
      title="Adicionar tempo esquecido"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} type="button">
            Cancelar
          </Button>
          <Button
            variant="primary"
            type="button"
            onClick={() => void handleSave()}
            disabled={saving || seconds === 0}
          >
            {saving ? "Salvando…" : "Salvar"}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        {locked ? (
          <Field label="Projeto" htmlFor="q-project">
            <Input id="q-project" value={anchorProjectName} disabled />
          </Field>
        ) : (
          <Field label="Projeto" htmlFor="q-project" required>
            <Select
              id="q-project"
              value={projectId}
              onChange={(e) => setProjectId(e.target.value)}
              required
            >
              <option value="">Selecione…</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.code ? `${p.code} · ${p.name}` : p.name}
                </option>
              ))}
            </Select>
          </Field>
        )}

        <div className="rounded border border-border bg-surface-raised py-5 text-center">
          <p
            data-testid="quick-total"
            className="tabular text-4xl font-semibold tracking-tight text-text"
          >
            {formatDuration(seconds)}
          </p>
          <p className="mt-1 text-2xs uppercase tracking-wide text-text-subtle">
            total a adicionar
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
          {QUICK_INCREMENTS.map((inc) => (
            <Button
              key={inc.label}
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => add(inc.seconds)}
            >
              {inc.label}
            </Button>
          ))}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => add(-15 * 60)}
          >
            -15min
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => setSeconds(0)}
          >
            Limpar
          </Button>
        </div>

        {locked ? (
          <p className="text-xs text-text-muted">
            O tempo entra como um registro separado, logo antes desta sessao.
            A sessao original nao e alterada.
          </p>
        ) : (
          <Field
            label="Dia"
            htmlFor="q-day"
            hint="O horario e aproximado — o que importa e a duracao."
          >
            <Input
              id="q-day"
              type="date"
              value={day}
              onChange={(e) => setDay(e.target.value)}
            />
          </Field>
        )}

        <Field label="Nota (opcional)" htmlFor="q-note">
          <Input
            id="q-note"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="No que voce estava trabalhando?"
          />
        </Field>

        {error && <p className="text-sm text-danger">{error}</p>}
      </div>
    </Modal>
  );
}
