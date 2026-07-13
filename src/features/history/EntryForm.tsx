import { useState, type FormEvent } from "react";
import type { ActivityType, TimeEntry } from "@/types/domain";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { isoToLocalInput, localInputToIso } from "@/lib/datetime";
import { ACTIVITY_TYPE_OPTIONS } from "@/lib/labels";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { Field, Input, Select, Textarea } from "@/components/ui/Field";

export interface EntryPrefill {
  startedAt: string;
  endedAt: string;
  description?: string;
  source?: "manual" | "reconstructed";
}

interface EntryFormProps {
  open: boolean;
  entry: TimeEntry | null;
  onClose: () => void;
  /** Valores iniciais ao criar (ex.: transformar lacuna em registro — secao 14). */
  prefill?: EntryPrefill;
}

function defaultStart(): string {
  const d = new Date(Date.now() - 60 * 60 * 1000);
  return isoToLocalInput(d.toISOString());
}
function defaultEnd(): string {
  return isoToLocalInput(new Date().toISOString());
}

/** Formulario de criacao (sessao manual) e edicao de sessao. */
export function EntryForm({ open, entry, onClose, prefill }: EntryFormProps) {
  const projects = useCatalogStore((s) => s.projects);
  const create = useEntriesStore((s) => s.create);
  const update = useEntriesStore((s) => s.update);

  const [projectId, setProjectId] = useState("");
  const [startedAt, setStartedAt] = useState(defaultStart());
  const [endedAt, setEndedAt] = useState(defaultEnd());
  const [activityType, setActivityType] = useState<ActivityType>("drawing");
  const [billable, setBillable] = useState(true);
  const [idleMinutes, setIdleMinutes] = useState("0");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [initializedFor, setInitializedFor] = useState<string | null>(null);
  const key = entry?.id ?? (prefill ? `gap-${prefill.startedAt}` : "new");
  if (open && initializedFor !== key) {
    setProjectId(entry?.projectId ?? "");
    setStartedAt(
      entry
        ? isoToLocalInput(entry.startedAt)
        : prefill
          ? isoToLocalInput(prefill.startedAt)
          : defaultStart(),
    );
    setEndedAt(
      entry?.endedAt
        ? isoToLocalInput(entry.endedAt)
        : prefill
          ? isoToLocalInput(prefill.endedAt)
          : defaultEnd(),
    );
    setActivityType(entry?.activityType ?? "drawing");
    setBillable(entry?.billable ?? true);
    setIdleMinutes(entry ? String(Math.round(entry.idleSeconds / 60)) : "0");
    setDescription(entry?.description ?? prefill?.description ?? "");
    setError(null);
    setInitializedFor(key);
  }
  if (!open && initializedFor !== null) setInitializedFor(null);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      const idleSeconds = Math.max(0, Number(idleMinutes) || 0) * 60;
      const common = {
        startedAt: localInputToIso(startedAt),
        endedAt: localInputToIso(endedAt),
        description: description || null,
        activityType,
        billable,
        idleSeconds,
      };
      if (entry) {
        await update(entry.id, common);
      } else {
        if (!projectId) throw "Selecione um projeto.";
        await create({ ...common, projectId, source: prefill?.source });
      }
      onClose();
    } catch (err) {
      setError(typeof err === "string" ? err : "Falha ao salvar a sessao.");
    } finally {
      setSaving(false);
    }
  }

  const projectName =
    projects.find((p) => p.id === entry?.projectId)?.name ?? "Projeto";

  return (
    <Modal
      open={open}
      title={entry ? "Editar sessao" : "Nova sessao manual"}
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} type="button">
            Cancelar
          </Button>
          <Button
            variant="primary"
            type="submit"
            form="entry-form"
            disabled={saving}
          >
            {saving ? "Salvando…" : "Salvar"}
          </Button>
        </>
      }
    >
      <form id="entry-form" onSubmit={handleSubmit} className="space-y-4">
        {entry ? (
          <Field label="Projeto" htmlFor="e-project">
            <Input id="e-project" value={projectName} disabled />
          </Field>
        ) : (
          <Field label="Projeto" htmlFor="e-project" required>
            <Select
              id="e-project"
              value={projectId}
              onChange={(ev) => setProjectId(ev.target.value)}
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

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="Inicio" htmlFor="e-start" required>
            <Input
              id="e-start"
              type="datetime-local"
              value={startedAt}
              onChange={(ev) => setStartedAt(ev.target.value)}
              required
            />
          </Field>
          <Field label="Fim" htmlFor="e-end" required>
            <Input
              id="e-end"
              type="datetime-local"
              value={endedAt}
              onChange={(ev) => setEndedAt(ev.target.value)}
              required
            />
          </Field>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="Atividade" htmlFor="e-activity">
            <Select
              id="e-activity"
              value={activityType}
              onChange={(ev) => setActivityType(ev.target.value as ActivityType)}
            >
              {ACTIVITY_TYPE_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="Minutos inativos" htmlFor="e-idle">
            <Input
              id="e-idle"
              inputMode="numeric"
              value={idleMinutes}
              onChange={(ev) => setIdleMinutes(ev.target.value)}
            />
          </Field>
        </div>

        <Checkbox label="Faturavel" checked={billable} onChange={setBillable} />

        <Field label="Descricao" htmlFor="e-desc">
          <Textarea
            id="e-desc"
            rows={2}
            value={description}
            onChange={(ev) => setDescription(ev.target.value)}
          />
        </Field>

        {error && <p className="text-sm text-danger">{error}</p>}
      </form>
    </Modal>
  );
}
