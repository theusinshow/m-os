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
  moneyOf,
  secondsOf,
  SOURCE_LABEL,
  type Draft,
} from "./TempoShared";
import type { Project, TimeEntry } from "./types";

/** `22:33` — a hora de parede, que é como se lembra de uma sessão. */
function clockOf(iso: string) {
  return new Date(iso).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
}

function fullDay(iso: string) {
  return new Date(iso).toLocaleDateString("pt-BR");
}

/**
 * A lista de sessões, com correção e remoção.
 *
 * Uma só, usada pelo Painel (últimas) e pelo Histórico (todas, filtradas). Duas
 * listas parecidas divergiriam: o dia em que "Corrigir" ganhasse um campo, uma
 * das duas ficaria para trás — e seria a que o usuário estivesse usando.
 */
export function TempoSessions({ entries, projects, onChanged, receipt, onError, variant = "list", amounts }: {
  entries: TimeEntry[];
  projects: Project[];
  onChanged: () => void;
  receipt?: (action: { message: string; run: () => Promise<unknown> }) => void;
  onError: (message: string) => void;
  /**
   * `list` na coluna estreita do Painel; `table` no Histórico.
   *
   * Duas apresentações e UMA lógica. Separar em dois componentes faria a
   * correção de sessão existir duas vezes, e no dia em que ela ganhasse um
   * campo, uma das duas ficaria para trás — justamente a que o usuário
   * estivesse usando.
   */
  variant?: "list" | "table";
  /** Valor REAL por sessão, calculado no Rust. Só a tabela usa. */
  amounts?: Record<string, number>;
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
      {variant === "table" ? (
        <table className="tempo-table tempo-table-sessions">
          <thead>
            <tr>
              <th scope="col">Data</th>
              <th scope="col">Project</th>
              <th scope="col">Atividade</th>
              {/* O PERÍODO é o que faz a sessão ser reconhecível: "22:33—22:53"
                  devolve a lembrança de quando aquilo aconteceu, coisa que
                  "20min" sozinho não faz. */}
              <th scope="col">Período</th>
              <th scope="col">Duração</th>
              <th scope="col">Valor</th>
              <th scope="col"><span className="visually-hidden">Ações</span></th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => {
              const suspicion = inspectEntry(entry);
              const amount = amounts?.[entry.id];
              return (
                <tr key={entry.id}>
                  <th scope="row">{fullDay(entry.startedAt)}</th>
                  <td>
                    {named(entry.projectId)?.name ?? "Project removido"}
                    {entry.description ? <small>{entry.description}</small> : null}
                  </td>
                  <td>
                    {ACTIVITY_LABEL[entry.activityType] ?? entry.activityType}
                    {entry.source === "timer" ? null : <small>{SOURCE_LABEL[entry.source]}</small>}
                  </td>
                  <td>
                    {entry.endedAt ? `${clockOf(entry.startedAt)}—${clockOf(entry.endedAt)}` : clockOf(entry.startedAt)}
                    {suspicion.length ? (
                      <small className="tempo-flag" title={suspicionText(suspicion)}>Conferir?</small>
                    ) : null}
                  </td>
                  <td>{durationOf(entry.durationSeconds)}</td>
                  <td>
                    {amount === undefined ? "—" : moneyOf(amount)}
                    {entry.billable ? null : <small>não cobrável</small>}
                  </td>
                  <td>
                    <span className="tempo-row-actions">
                      <Button variant="ghost" size="sm" onClick={() => openEdit(entry)}>Corrigir</Button>
                      <Button variant="ghost" size="sm" onClick={() => void trash(entry)}>Remover</Button>
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      ) : (
      <div className="tempo-sessions">
        {entries.map((entry) => {
          const suspicion = inspectEntry(entry);
          return (
            <div className="tempo-session" key={entry.id}>
              <span>
                {/* O selo mora DENTRO do bloco do nome, e nao ao lado dele.
                    Como irmao dos outros filhos do flex ele era um quarto item
                    na linha, e numa sessao marcada a soma passava da largura do
                    card: o "Corrigir" caia sozinho para uma segunda altura e
                    aquela linha ficava com o dobro do tamanho das vizinhas. A
                    lista inteira perdia a varredura por causa de um selo. */}
                <span className="tempo-session-title">
                  <strong>{named(entry.projectId)?.name ?? "Project removido"}</strong>
                  {/* O selo não bloqueia nada e não altera o registro: uma sessão
                      de dez horas pode ser real. Ele só põe o olho em cima antes
                      de a hora virar fatura. */}
                  {suspicion.length ? (
                    <span className="tempo-flag" title={suspicionText(suspicion)}>Conferir?</span>
                  ) : null}
                </span>
                {/* A origem só aparece quando NÃO é o cronômetro. */}
                <small>
                  {dayOf(entry.startedAt)} · {ACTIVITY_LABEL[entry.activityType] ?? entry.activityType}
                  {entry.source === "reconstructed" ? " · reconstruída" : ""}
                  {entry.source === "manual" ? " · manual" : ""}
                  {entry.billable ? "" : " · não cobrável"}
                  {entry.description ? ` · ${entry.description}` : ""}
                </small>
              </span>
              <span className="tempo-session-duration">{durationOf(entry.durationSeconds)}</span>
              {/* UMA acao na coluna estreita, e nao duas.
                  Cabem duas no Historico, que e largo; aqui elas empurravam a
                  linha para uma terceira altura e a lista deixava de ser
                  varrivel de relance. Remover continua a um clique, na tela que
                  existe para mexer em sessao — e e a mesma divisao que o
                  CronoCAD faz entre o Painel e o Historico dele. */}
              <span className="tempo-session-actions">
                <Button variant="ghost" size="sm" onClick={() => openEdit(entry)}>Corrigir</Button>
              </span>
            </div>
          );
        })}
      </div>
      )}

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
