import { useEffect, useMemo, useState } from "react";
import { Clock, Pencil, Plus, RotateCcw, Trash2 } from "lucide-react";
import type { TimeEntry } from "@/types/domain";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { listTimeEntries } from "@/services/timeEntries";
import {
  formatCurrency,
  formatDate,
  formatDuration,
  formatTime,
} from "@/lib/format";
import { amountForDuration } from "@/lib/money";
import { ACTIVITY_TYPE_LABELS } from "@/lib/labels";
import { PageHeader } from "@/components/ui/PageHeader";
import { Panel } from "@/components/ui/Panel";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { EmptyState } from "@/components/ui/EmptyState";
import { Field, Input, Select } from "@/components/ui/Field";
import { EntryForm } from "./EntryForm";
import { QuickTimeModal } from "./QuickTimeModal";
import { DeleteEntryModal } from "./DeleteEntryModal";

export function HistoryPage() {
  const entries = useEntriesStore((s) => s.entries);
  const remove = useEntriesStore((s) => s.remove);
  const restore = useEntriesStore((s) => s.restore);
  const { clients, projects } = useCatalogStore();

  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [projectId, setProjectId] = useState("");
  const [clientId, setClientId] = useState("");
  const [showDeleted, setShowDeleted] = useState(false);
  const [deleted, setDeleted] = useState<TimeEntry[]>([]);

  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<TimeEntry | null>(null);
  const [quickOpen, setQuickOpen] = useState(false);
  const [quickAnchor, setQuickAnchor] = useState<TimeEntry | null>(null);
  const [deleting, setDeleting] = useState<TimeEntry | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);

  useEffect(() => {
    if (!showDeleted) return;
    let active = true;
    void listTimeEntries(undefined, true).then((all) => {
      if (active) setDeleted(all.filter((e) => e.deletedAt !== null));
    });
    return () => {
      active = false;
    };
  }, [showDeleted, entries]);

  const projectOf = (id: string) => projects.find((p) => p.id === id);
  const projectName = (id: string) => projectOf(id)?.name ?? "—";

  const filtered = useMemo(() => {
    const fromTime = from ? new Date(`${from}T00:00:00`).getTime() : -Infinity;
    const toTime = to ? new Date(`${to}T23:59:59.999`).getTime() : Infinity;
    return entries.filter((e) => {
      const t = new Date(e.startedAt).getTime();
      if (t < fromTime || t > toTime) return false;
      if (projectId && e.projectId !== projectId) return false;
      if (clientId && projectOf(e.projectId)?.clientId !== clientId)
        return false;
      return true;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entries, from, to, projectId, clientId, projects]);

  const totalSeconds = filtered.reduce((s, e) => s + e.durationSeconds, 0);

  async function confirmDelete() {
    if (!deleting) return;
    setDeleteBusy(true);
    try {
      await remove(deleting.id);
      setDeleting(null);
    } finally {
      setDeleteBusy(false);
    }
  }

  return (
    <div>
      <PageHeader
        title="Historico"
        description="Sessoes registradas: filtre, edite, adicione ou remova."
        action={
          <div className="flex gap-2">
            <Button
              variant="secondary"
              onClick={() => {
                setQuickAnchor(null);
                setQuickOpen(true);
              }}
              icon={<Clock size={16} strokeWidth={2} />}
            >
              Tempo esquecido
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                setEditing(null);
                setFormOpen(true);
              }}
              icon={<Plus size={16} strokeWidth={2} />}
            >
              Nova sessao
            </Button>
          </div>
        }
      />

      <Panel className="mb-4 p-4">
        <div className="grid gap-3 md:grid-cols-4">
          <Field label="De" htmlFor="f-from">
            <Input
              id="f-from"
              type="date"
              value={from}
              onChange={(e) => setFrom(e.target.value)}
            />
          </Field>
          <Field label="Ate" htmlFor="f-to">
            <Input
              id="f-to"
              type="date"
              value={to}
              onChange={(e) => setTo(e.target.value)}
            />
          </Field>
          <Field label="Cliente" htmlFor="f-client">
            <Select
              id="f-client"
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
            >
              <option value="">Todos</option>
              {clients.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="Projeto" htmlFor="f-project">
            <Select
              id="f-project"
              value={projectId}
              onChange={(e) => setProjectId(e.target.value)}
            >
              <option value="">Todos</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </Field>
        </div>
        <div className="mt-3 flex items-center justify-between">
          <Checkbox
            label="Mostrar excluidas"
            checked={showDeleted}
            onChange={setShowDeleted}
          />
          {!showDeleted && (
            <span className="text-xs text-text-muted">
              {filtered.length} sessao(oes) · {formatDuration(totalSeconds)}
            </span>
          )}
        </div>
      </Panel>

      {showDeleted ? (
        <Panel>
          {deleted.length === 0 ? (
            <p className="px-4 py-6 text-sm text-text-muted">
              Nenhuma sessao excluida.
            </p>
          ) : (
            <ul className="divide-y divide-border">
              {deleted.map((entry) => (
                <li
                  key={entry.id}
                  className="flex items-center justify-between px-4 py-3"
                >
                  <div>
                    <p className="text-sm text-text">
                      {projectName(entry.projectId)}
                    </p>
                    <p className="text-xs text-text-muted">
                      {formatDate(entry.startedAt)} ·{" "}
                      {formatDuration(entry.durationSeconds)}
                    </p>
                  </div>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => void restore(entry.id)}
                    icon={<RotateCcw size={15} strokeWidth={1.75} />}
                  >
                    Restaurar
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      ) : filtered.length === 0 ? (
        <EmptyState
          title="Nenhuma sessao"
          description="Ajuste os filtros ou registre uma sessao (pelo cronometro ou manual)."
        />
      ) : (
        <Panel>
          {/*
            Sao 7 colunas. Na janela minima (960px) sobram 680px para a tabela,
            e sem esta rolagem a coluna de acoes ficava FORA da area visivel —
            os botoes de editar e excluir simplesmente nao eram alcancaveis.
          */}
          <div className="overflow-x-auto">
            <table className="w-full min-w-[980px] text-sm">
            <thead>
              <tr className="border-b border-border text-left text-2xs uppercase tracking-wide text-text-subtle">
                <th className="px-4 py-2 font-medium">Data</th>
                <th className="px-4 py-2 font-medium">Projeto</th>
                <th className="px-4 py-2 font-medium">Atividade</th>
                <th className="px-4 py-2 font-medium">Periodo</th>
                <th className="px-4 py-2 text-right font-medium">Duracao</th>
                <th className="px-4 py-2 text-right font-medium">Valor</th>
                <th className="px-4 py-2 text-right font-medium">Acoes</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {filtered.map((entry) => (
                <tr key={entry.id} className="hover:bg-surface-hover">
                  <td className="tabular whitespace-nowrap px-4 py-3 text-text-muted">
                    {formatDate(entry.startedAt)}
                  </td>
                  <td className="px-4 py-3 text-text">
                    {projectName(entry.projectId)}
                    {!entry.billable && (
                      <span className="ml-2 text-2xs text-text-subtle">
                        (nao faturavel)
                      </span>
                    )}
                  </td>
                  <td className="px-4 py-3 text-text-muted">
                    {ACTIVITY_TYPE_LABELS[entry.activityType]}
                  </td>
                  <td className="tabular whitespace-nowrap px-4 py-3 text-text-muted">
                    {formatTime(entry.startedAt)}
                    {entry.endedAt ? `–${formatTime(entry.endedAt)}` : ""}
                  </td>
                  <td className="tabular whitespace-nowrap px-4 py-3 text-right text-text">
                    {formatDuration(entry.durationSeconds)}
                  </td>
                  <td className="tabular whitespace-nowrap px-4 py-3 text-right text-text">
                    {formatCurrency(
                      amountForDuration(
                        entry.durationSeconds - entry.idleSeconds,
                        entry.hourlyRateSnapshotCents,
                      ),
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <div className="flex justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setQuickAnchor(entry);
                          setQuickOpen(true);
                        }}
                        aria-label="Adicionar tempo a esta sessao"
                        icon={<Clock size={15} strokeWidth={1.75} />}
                      />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setEditing(entry);
                          setFormOpen(true);
                        }}
                        aria-label="Editar sessao"
                        icon={<Pencil size={15} strokeWidth={1.75} />}
                      />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setDeleting(entry)}
                        aria-label="Excluir sessao"
                        icon={<Trash2 size={15} strokeWidth={1.75} />}
                      />
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
            </table>
          </div>
        </Panel>
      )}

      <EntryForm
        open={formOpen}
        entry={editing}
        onClose={() => setFormOpen(false)}
      />

      <DeleteEntryModal
        open={deleting !== null}
        entry={deleting}
        projectName={deleting ? projectName(deleting.projectId) : ""}
        busy={deleteBusy}
        onCancel={() => setDeleting(null)}
        onConfirm={() => void confirmDelete()}
      />

      <QuickTimeModal
        open={quickOpen}
        anchor={quickAnchor}
        onClose={() => setQuickOpen(false)}
      />
    </div>
  );
}
