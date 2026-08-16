import { useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { EmptyState } from "./Surface";
import { inspectEntry, suspicionText } from "./suspiciousEntry";
import {
  ACTIVITY_LABEL,
  dateInputOf,
  dayOf,
  DraftFields,
  durationOf,
  emptyDraft,
  momentOf,
  secondsOf,
  type Draft,
} from "./TempoShared";
import type { Project, TimeEntry } from "./types";

/**
 * A lista de sessões, com correção e remoção.
 *
 * Uma só, usada pelo Painel (últimas) e pelo Histórico (todas, filtradas). Duas
 * listas parecidas divergiriam: o dia em que "Corrigir" ganhasse um campo, uma
 * das duas ficaria para trás — e seria a que o usuário estivesse usando.
 */
export function TempoSessions({ entries, projects, onChanged, receipt, onError }: {
  entries: TimeEntry[];
  projects: Project[];
  onChanged: () => void;
  receipt?: (action: { message: string; run: () => Promise<unknown> }) => void;
  onError: (message: string) => void;
}) {
  const [editing, setEditing] = useState<TimeEntry | null>(null);
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const dialog = useRef<HTMLDialogElement>(null);

  const named = (id: string) => projects.find((project) => project.id === id);

  async function guard(run: () => Promise<unknown>) {
    onError("");
    try {
      await run();
      onChanged();
    } catch (error) {
      onError(error instanceof Error ? error.message : String(error));
    }
  }

  function openEdit(entry: TimeEntry) {
    setEditing(entry);
    setDraft({
      day: dateInputOf(entry.startedAt),
      hours: Math.floor(entry.durationSeconds / 3600),
      minutes: Math.floor((entry.durationSeconds % 3600) / 60),
      description: entry.description,
      activityType: entry.activityType,
      billable: entry.billable,
    });
    dialog.current?.showModal();
  }

  async function saveEdit() {
    if (!editing) return;
    await guard(async () => {
      await api.trackingEdit(editing.id, {
        startedAt: momentOf(draft.day, editing.startedAt),
        durationSeconds: secondsOf(draft),
        idleSeconds: editing.idleSeconds,
        description: draft.description,
        activityType: draft.activityType,
        billable: draft.billable,
      });
      dialog.current?.close();
      setEditing(null);
    });
  }

  async function trash(entry: TimeEntry) {
    await guard(async () => {
      await api.trackingTrash(entry.id);
      // Soft delete, então o inverso existe de verdade (ADR-035).
      receipt?.({
        message: `Sessão de ${durationOf(entry.durationSeconds)} removida.`,
        run: () => api.trackingRestore(entry.id).then(() => onChanged()),
      });
    });
  }

  if (!entries.length) {
    return <EmptyState>Nenhuma sessão neste recorte.</EmptyState>;
  }

  return (
    <>
      <div className="tempo-sessions">
        {entries.map((entry) => {
          const suspicion = inspectEntry(entry);
          return (
            <div className="tempo-session" key={entry.id}>
              <span>
                <strong>{named(entry.projectId)?.name ?? "Project removido"}</strong>
                {/* A origem só aparece quando NÃO é o cronômetro. */}
                <small>
                  {dayOf(entry.startedAt)} · {ACTIVITY_LABEL[entry.activityType] ?? entry.activityType}
                  {entry.source === "reconstructed" ? " · reconstruída" : ""}
                  {entry.source === "manual" ? " · manual" : ""}
                  {entry.billable ? "" : " · não cobrável"}
                  {entry.description ? ` · ${entry.description}` : ""}
                </small>
              </span>
              {/* O selo não bloqueia nada e não altera o registro: uma sessão de
                  dez horas pode ser real. Ele só põe o olho em cima antes de a
                  hora virar fatura. */}
              {suspicion.length ? (
                <span className="tempo-flag" title={suspicionText(suspicion)}>Conferir?</span>
              ) : null}
              <span className="tempo-session-duration">{durationOf(entry.durationSeconds)}</span>
              <span className="tempo-session-actions">
                <Button variant="ghost" size="sm" onClick={() => openEdit(entry)}>Corrigir</Button>
                <Button variant="ghost" size="sm" onClick={() => void trash(entry)}>Remover</Button>
              </span>
            </div>
          );
        })}
      </div>

      <dialog ref={dialog} className="restore-dialog" onCancel={() => { dialog.current?.close(); setEditing(null); }}>
        <span className="micro-label">CORRIGIR SESSÃO</span>
        <h2>{editing ? named(editing.projectId)?.name ?? "Sessão" : "Sessão"}</h2>
        {/* O Project não é editável aqui, e a razão é de dinheiro: mover uma hora
            entre Projects mexeria no valor/hora guardado, e reprecificar em
            silêncio pode alterar algo já faturado. */}
        <p className="support-copy">O Project não muda por aqui. A taxa gravada é a do momento em que o trabalho aconteceu.</p>
        <form className="tempo-form" onSubmit={(event) => { event.preventDefault(); void saveEdit(); }}>
          <DraftFields draft={draft} onChange={setDraft} idPrefix="edit" />
          <div className="form-actions">
            <Button variant="ghost" onClick={() => { dialog.current?.close(); setEditing(null); }}>Cancelar</Button>
            <Button variant="primary" type="submit" disabled={secondsOf(draft) <= 0}>Salvar</Button>
          </div>
        </form>
      </dialog>
    </>
  );
}
