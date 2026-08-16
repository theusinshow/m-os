import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { ContextPath, EmptyState, Panel } from "./Surface";
import { TempoClients } from "./TempoClients";
import { TempoSettings } from "./TempoSettings";
import { Timer } from "./Timer";
import type { ActivityType, Project, TimeEntry, Totals } from "./types";

/** `16,1 h` — a unidade do trabalho cobrado é a hora, e uma casa basta. */
function hoursOf(seconds: number) {
  return `${(seconds / 3600).toFixed(1)} h`;
}

function moneyOf(cents: number) {
  return (cents / 100).toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
}

/** `2h07` na linha da sessão: minutos importam quando se olha uma sessão só. */
function durationOf(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}h${String(minutes).padStart(2, "0")}` : `${minutes}min`;
}

function dayOf(iso: string) {
  return new Date(iso).toLocaleDateString("pt-BR", { day: "2-digit", month: "short" });
}

/** `2026-08-16` no fuso LOCAL: `toISOString` devolve UTC e vira o dia errado à noite. */
function dateInputOf(iso: string) {
  const moment = new Date(iso);
  const local = new Date(moment.getTime() - moment.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 10);
}

/** Monta o instante a partir do dia escolhido, preservando a hora original. */
function momentOf(day: string, keepTimeFrom?: string) {
  const base = keepTimeFrom ? new Date(keepTimeFrom) : new Date();
  const [year, month, date] = day.split("-").map(Number);
  const moment = new Date(base);
  moment.setFullYear(year, month - 1, date);
  return moment.toISOString();
}

const ACTIVITY: { value: ActivityType; label: string }[] = [
  { value: "drawing", label: "desenho" },
  { value: "detailing", label: "detalhamento" },
  { value: "revision", label: "revisão" },
  { value: "meeting", label: "reunião" },
  { value: "study", label: "estudo" },
  { value: "other", label: "outro" },
];

const ACTIVITY_LABEL = Object.fromEntries(ACTIVITY.map((item) => [item.value, item.label]));

/** Os campos que o lançamento e a correção compartilham. */
type Draft = {
  day: string;
  hours: number;
  minutes: number;
  description: string;
  activityType: ActivityType;
  billable: boolean;
};

function emptyDraft(): Draft {
  return {
    day: dateInputOf(new Date().toISOString()),
    hours: 1,
    minutes: 0,
    description: "",
    activityType: "other",
    billable: true,
  };
}

/**
 * Horas e minutos em campos separados, e não um texto livre.
 *
 * "1h30", "1:30" e "1,5" são a mesma duração escrita de três jeitos, e qualquer
 * parser que aceite os três vai errar um quarto. Dois números não têm ambiguidade
 * — e o teclado numérico do sistema já valida.
 */
function DurationFields({ draft, onChange, idPrefix }: {
  draft: Draft;
  onChange: (next: Draft) => void;
  idPrefix: string;
}) {
  return (
    <div className="tempo-duration">
      <label htmlFor={`${idPrefix}-hours`}>Horas</label>
      <input
        id={`${idPrefix}-hours`}
        type="number"
        min={0}
        max={23}
        value={draft.hours}
        onChange={(event) => onChange({ ...draft, hours: Math.max(0, Number(event.currentTarget.value) || 0) })}
      />
      <label htmlFor={`${idPrefix}-minutes`}>Minutos</label>
      <input
        id={`${idPrefix}-minutes`}
        type="number"
        min={0}
        max={59}
        step={5}
        value={draft.minutes}
        onChange={(event) => onChange({ ...draft, minutes: Math.min(59, Math.max(0, Number(event.currentTarget.value) || 0)) })}
      />
    </div>
  );
}

function DraftFields({ draft, onChange, idPrefix }: {
  draft: Draft;
  onChange: (next: Draft) => void;
  idPrefix: string;
}) {
  return (
    <>
      <div className="tempo-field">
        <label htmlFor={`${idPrefix}-day`}>Dia</label>
        <input
          id={`${idPrefix}-day`}
          type="date"
          value={draft.day}
          onChange={(event) => onChange({ ...draft, day: event.currentTarget.value })}
        />
      </div>
      <DurationFields draft={draft} onChange={onChange} idPrefix={idPrefix} />
      <div className="tempo-field">
        <label htmlFor={`${idPrefix}-activity`}>Atividade</label>
        <select
          id={`${idPrefix}-activity`}
          value={draft.activityType}
          onChange={(event) => onChange({ ...draft, activityType: event.currentTarget.value as ActivityType })}
        >
          {ACTIVITY.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
        </select>
      </div>
      <div className="tempo-field">
        <label htmlFor={`${idPrefix}-description`}>Descrição</label>
        <input
          id={`${idPrefix}-description`}
          value={draft.description}
          placeholder="opcional"
          onChange={(event) => onChange({ ...draft, description: event.currentTarget.value })}
        />
      </div>
      <label className="tempo-check">
        <input
          type="checkbox"
          checked={draft.billable}
          onChange={(event) => onChange({ ...draft, billable: event.currentTarget.checked })}
        />
        Cobrável
      </label>
    </>
  );
}

/**
 * A página de Tempo.
 *
 * Quatro perguntas, nesta ordem: o que estou contando agora, o que esqueci de
 * contar, quanto cada Project acumulou, e o que exatamente aconteceu.
 *
 * O lançamento manual vem logo depois do cronômetro de propósito. A pergunta que
 * guia o CronoCAD é "isso reduz a chance de esquecer de registrar o trabalho?",
 * e esquecer de INICIAR é tão comum quanto esquecer de encerrar.
 */
export function TempoPage({ projects, openProject, receipt }: {
  projects: Project[];
  openProject: (project: Project) => void;
  receipt?: (action: { message: string; run: () => Promise<unknown> }) => void;
}) {
  const [totals, setTotals] = useState<Record<string, Totals>>({});
  const [entries, setEntries] = useState<TimeEntry[]>([]);
  const [note, setNote] = useState("");
  const [choice, setChoice] = useState("");
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [editing, setEditing] = useState<TimeEntry | null>(null);
  const [editDraft, setEditDraft] = useState<Draft>(emptyDraft);
  const dialog = useRef<HTMLDialogElement>(null);

  const load = useCallback(async () => {
    const [nextTotals, nextEntries] = await Promise.all([
      api.trackingTotals().catch(() => ({}) as Record<string, Totals>),
      api.trackingEntries().catch(() => [] as TimeEntry[]),
    ]);
    setTotals(nextTotals);
    setEntries(nextEntries);
  }, []);

  useEffect(() => { void load(); }, [load]);

  async function guard(run: () => Promise<unknown>) {
    setNote("");
    try {
      await run();
      await load();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }

  const secondsOf = (value: Draft) => value.hours * 3600 + value.minutes * 60;

  async function record() {
    const seconds = secondsOf(draft);
    if (!choice || seconds <= 0) return;
    await guard(async () => {
      await api.trackingRecord({
        projectId: choice,
        startedAt: momentOf(draft.day),
        durationSeconds: seconds,
        description: draft.description,
        activityType: draft.activityType,
        billable: draft.billable,
      });
      setDraft(emptyDraft());
    });
  }

  function openEdit(entry: TimeEntry) {
    setEditing(entry);
    setEditDraft({
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
        startedAt: momentOf(editDraft.day, editing.startedAt),
        durationSeconds: secondsOf(editDraft),
        idleSeconds: editing.idleSeconds,
        description: editDraft.description,
        activityType: editDraft.activityType,
        billable: editDraft.billable,
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
        run: () => api.trackingRestore(entry.id).then(() => load()),
      });
    });
  }

  const named = (id: string) => projects.find((project) => project.id === id);
  const active = projects.filter((project) => project.lifecycleState === "active");

  // Só Projects com tempo. Listar os que têm zero encheria a página com linhas
  // que não respondem nada — quem quer ver todos os Projects tem a página deles.
  const ranked = Object.entries(totals)
    .filter(([, total]) => total.grossSeconds > 0)
    .sort(([, left], [, right]) => right.grossSeconds - left.grossSeconds);

  const trackedTotal = ranked.reduce((sum, [, total]) => sum + total.grossSeconds, 0);

  return (
    <div className="page">
      <ContextPath segments={["M", "TEMPO"]} />

      <Panel label="CRONÔMETRO" rule>
        <Timer projects={projects} onChanged={() => void load()} />
      </Panel>

      <Panel label="LANÇAR TEMPO ESQUECIDO">
        {active.length ? (
          <form className="tempo-form" onSubmit={(event) => { event.preventDefault(); void record(); }}>
            <div className="tempo-field">
              <label htmlFor="record-project">Project</label>
              <select id="record-project" value={choice} onChange={(event) => setChoice(event.currentTarget.value)}>
                <option value="">Escolha</option>
                {active.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
              </select>
            </div>
            <DraftFields draft={draft} onChange={setDraft} idPrefix="record" />
            <Button variant="primary" size="sm" type="submit" disabled={!choice || secondsOf(draft) <= 0}>Lançar</Button>
            {/* Dito na tela, e não escondido no banco: hora lançada a mão é
                estimativa, e quem fatura precisa saber a diferença. */}
            <p className="support-copy">Entra marcada como <strong>manual</strong> — hora estimada depois não é hora medida.</p>
          </form>
        ) : (
          <EmptyState>Crie um Project para lançar tempo nele.</EmptyState>
        )}
      </Panel>

      <Panel label="POR PROJECT" count={trackedTotal ? hoursOf(trackedTotal) : undefined}>
        {ranked.length ? (
          <table className="tempo-table">
            <thead>
              <tr>
                <th scope="col">Project</th>
                <th scope="col">Trabalhado</th>
                <th scope="col">Cobrável</th>
                <th scope="col">Valor</th>
              </tr>
            </thead>
            <tbody>
              {ranked.map(([id, total]) => {
                const project = named(id);
                return (
                  <tr key={id}>
                    <th scope="row">
                      {project ? (
                        <button type="button" onClick={() => openProject(project)}>{project.name}</button>
                      ) : (
                        "Project removido"
                      )}
                    </th>
                    {/* Bruto e cobrável lado a lado de propósito: são perguntas
                        diferentes — quanto eu trabalhei, e quanto eu cobro. */}
                    <td>{hoursOf(total.grossSeconds)}</td>
                    <td>{hoursOf(total.billableSeconds)}</td>
                    <td>{moneyOf(total.amountCents)}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        ) : (
          <EmptyState>Nenhuma hora registrada ainda. Inicie o cronômetro acima, ou importe o CronoCAD em Settings.</EmptyState>
        )}
      </Panel>

      <Panel label="SESSÕES" count={entries.length ? String(entries.length) : undefined}>
        {entries.length ? (
          <div className="tempo-sessions">
            {entries.map((entry) => (
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
                <span className="tempo-session-duration">{durationOf(entry.durationSeconds)}</span>
                <span className="tempo-session-actions">
                  <Button variant="ghost" size="sm" onClick={() => openEdit(entry)}>Corrigir</Button>
                  <Button variant="ghost" size="sm" onClick={() => void trash(entry)}>Remover</Button>
                </span>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState>As sessões encerradas aparecem aqui, da mais recente para a mais antiga.</EmptyState>
        )}
      </Panel>

      <TempoClients />
      <TempoSettings onChanged={() => void load()} />

      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}

      <dialog ref={dialog} className="restore-dialog" onCancel={() => { dialog.current?.close(); setEditing(null); }}>
        <span className="micro-label">CORRIGIR SESSÃO</span>
        <h2>{editing ? named(editing.projectId)?.name ?? "Sessão" : "Sessão"}</h2>
        {/* O Project não é editável aqui, e a razão é de dinheiro: mover uma hora
            entre Projects mexeria no valor/hora guardado, e reprecificar em
            silêncio pode alterar algo já faturado. */}
        <p className="support-copy">O Project não muda por aqui. A taxa gravada é a do momento em que o trabalho aconteceu.</p>
        <form className="tempo-form" onSubmit={(event) => { event.preventDefault(); void saveEdit(); }}>
          <DraftFields draft={editDraft} onChange={setEditDraft} idPrefix="edit" />
          <div className="form-actions">
            <Button variant="ghost" onClick={() => { dialog.current?.close(); setEditing(null); }}>Cancelar</Button>
            <Button variant="primary" type="submit" disabled={secondsOf(editDraft) <= 0}>Salvar</Button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
