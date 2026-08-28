import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { EmptyState, FilterBand, PageHeader, Region } from "./Surface";
import {
  ACTIVITY,
  dateInputOf,
  dayOf,
  durationOf,
  endOfDay,
  hoursOf,
  startOfDay,
} from "./TempoShared";
import { TempoSessions } from "./TempoSessions";
import type { ActivityType, Client, Project, ProjectTracking, TimeEntry } from "./types";

/** Os recortes que se pede na prática, em vez de digitar duas datas. */
type Preset = "tudo" | "hoje" | "semana" | "mes" | "mes-passado" | "personalizado";

const PRESETS: { value: Preset; label: string }[] = [
  { value: "hoje", label: "Hoje" },
  { value: "semana", label: "Semana" },
  { value: "mes", label: "Este mês" },
  { value: "mes-passado", label: "Mês passado" },
  { value: "tudo", label: "Tudo" },
];


/**
 * As bordas de cada recorte, em dia local.
 *
 * A semana começa na SEGUNDA, e não no domingo: quem cobra por hora trabalhada
 * conta a semana de trabalho, e um domingo no começo do intervalo empurra o
 * total para o mês anterior uma vez a cada sete.
 */
function rangeOf(preset: Preset, today = new Date()): { from: string; to: string } | null {
  const iso = (date: Date) => dateInputOf(date.toISOString());
  if (preset === "hoje") return { from: iso(today), to: iso(today) };
  if (preset === "semana") {
    const monday = new Date(today);
    const weekday = (today.getDay() + 6) % 7;
    monday.setDate(today.getDate() - weekday);
    return { from: iso(monday), to: iso(today) };
  }
  if (preset === "mes") {
    const first = new Date(today.getFullYear(), today.getMonth(), 1);
    return { from: iso(first), to: iso(today) };
  }
  if (preset === "mes-passado") {
    const first = new Date(today.getFullYear(), today.getMonth() - 1, 1);
    const last = new Date(today.getFullYear(), today.getMonth(), 0);
    return { from: iso(first), to: iso(last) };
  }
  return null;
}

/**
 * O histórico completo: filtre, corrija, remova, restaure.
 *
 * O Painel mostra as últimas sessões porque a pergunta lá é "o que aconteceu
 * agora". Aqui a pergunta é outra — "o que aconteceu naquele período, naquele
 * Project" — e ela exige filtro.
 */
export function TempoHistory({ projects, entries, onChanged, receipt }: {
  projects: Project[];
  entries: TimeEntry[];
  onChanged: () => void;
  receipt?: (action: { message: string; run: () => Promise<unknown> }) => void;
}) {
  const [preset, setPreset] = useState<Preset>("mes");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [projectId, setProjectId] = useState("");
  const [clientId, setClientId] = useState("");
  const [activity, setActivity] = useState<ActivityType | "">("");
  const [source, setSource] = useState("");
  const [clients, setClients] = useState<Client[]>([]);
  const [tracking, setTracking] = useState<ProjectTracking[]>([]);
  const [trashOpen, setTrashOpen] = useState(false);
  const [trashed, setTrashed] = useState<TimeEntry[]>([]);
  const [amounts, setAmounts] = useState<Record<string, number>>({});
  const [note, setNote] = useState("");

  useEffect(() => {
    void (async () => {
      const [people, rows, report] = await Promise.all([
        api.clients(true).catch(() => [] as Client[]),
        api.projectTracking().catch(() => [] as ProjectTracking[]),
        // O valor por sessão vem do Rust, calculado com a taxa que valia no dia.
        // Multiplicar aqui criaria um segundo caminho para o mesmo número.
        api.trackingReport(null, null).catch(() => []),
      ]);
      setClients(people);
      setTracking(rows);
      setAmounts(Object.fromEntries(report.map((line) => [line.entryId, line.rawAmountCents])));
    })();
  }, []);

  const loadTrash = useCallback(async () => {
    setTrashed(await api.trackingTrashed().catch(() => [] as TimeEntry[]));
  }, []);

  useEffect(() => { if (trashOpen) void loadTrash(); }, [trashOpen, loadTrash]);

  // Trocar de recorte preenche as datas em vez de escondê-las: o usuário vê
  // exatamente qual intervalo está somando, e pode ajustar a partir dele.
  function choose(next: Preset) {
    setPreset(next);
    const range = rangeOf(next);
    if (range) {
      setFrom(range.from);
      setTo(range.to);
    } else if (next === "tudo") {
      setFrom("");
      setTo("");
    }
  }

  useEffect(() => { choose("mes"); }, []);

  const projectsOfClient = useMemo(() => {
    if (!clientId) return null;
    return new Set(tracking.filter((row) => row.clientId === clientId).map((row) => row.projectId));
  }, [clientId, tracking]);

  const filtered = useMemo(() => {
    const start = from ? new Date(startOfDay(from)).getTime() : -Infinity;
    const end = to ? new Date(endOfDay(to)).getTime() : Infinity;
    return entries.filter((entry) => {
      const at = new Date(entry.startedAt).getTime();
      if (at < start || at > end) return false;
      if (projectId && entry.projectId !== projectId) return false;
      if (projectsOfClient && !projectsOfClient.has(entry.projectId)) return false;
      if (activity && entry.activityType !== activity) return false;
      if (source && entry.source !== source) return false;
      return true;
    });
  }, [entries, from, to, projectId, projectsOfClient, activity, source]);

  const totalSeconds = filtered.reduce((sum, entry) => sum + entry.durationSeconds, 0);

  async function restore(entry: TimeEntry) {
    setNote("");
    try {
      await api.trackingRestore(entry.id);
      await loadTrash();
      onChanged();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }

  const named = (id: string) => projects.find((project) => project.id === id)?.name ?? "Project removido";

  return (
    <>
      {/* Acima da lista, e não depois dela: o erro de uma correção precisa ser
          visto sem rolar por trinta sessões. */}
      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}

      <PageHeader
        title="Histórico"
        subtitle="Sessões registradas: filtre, corrija, remova ou restaure."
        actions={
          <Button variant="secondary" size="sm" onClick={() => setTrashOpen((open) => !open)}>
            {trashOpen ? "Ver sessões" : "Ver lixeira"}
          </Button>
        }
      />

      <FilterBand>
        <div className="tempo-filters">
          <div className="tempo-field">
            <label htmlFor="hist-from">De</label>
            <input
              id="hist-from"
              type="date"
              value={from}
              onChange={(event) => { setFrom(event.currentTarget.value); setPreset("personalizado"); }}
            />
          </div>
          <div className="tempo-field">
            <label htmlFor="hist-to">Até</label>
            <input
              id="hist-to"
              type="date"
              value={to}
              onChange={(event) => { setTo(event.currentTarget.value); setPreset("personalizado"); }}
            />
          </div>
          <div className="tempo-field">
            <label htmlFor="hist-client">Cliente</label>
            <select id="hist-client" value={clientId} onChange={(event) => setClientId(event.currentTarget.value)}>
              <option value="">Todos</option>
              {clients.map((client) => <option key={client.id} value={client.id}>{client.name}</option>)}
            </select>
          </div>
          <div className="tempo-field">
            <label htmlFor="hist-project">Project</label>
            <select id="hist-project" value={projectId} onChange={(event) => setProjectId(event.currentTarget.value)}>
              <option value="">Todos</option>
              {projects.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
            </select>
          </div>
          <div className="tempo-field">
            <label htmlFor="hist-activity">Atividade</label>
            <select
              id="hist-activity"
              value={activity}
              onChange={(event) => setActivity(event.currentTarget.value as ActivityType | "")}
            >
              <option value="">Todas</option>
              {ACTIVITY.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
            </select>
          </div>
          <div className="tempo-field">
            <label htmlFor="hist-source">Origem</label>
            {/* Filtrar por origem responde uma pergunta de confiança: quanto do
                que estou cobrando foi MEDIDO, e quanto foi estimado depois. */}
            <select id="hist-source" value={source} onChange={(event) => setSource(event.currentTarget.value)}>
              <option value="">Todas</option>
              <option value="timer">cronômetro</option>
              <option value="manual">manual</option>
              <option value="reconstructed">reconstruída</option>
            </select>
          </div>
        </div>

        {/* Atalhos de período como chips, e não mais um `select`: são cinco
            recortes que se pedem o tempo todo, e um menu suspenso cobra dois
            cliques e uma leitura para cada um deles. */}
        <div className="tempo-presets">
          {PRESETS.map((item) => (
            <button
              key={item.value}
              type="button"
              aria-pressed={preset === item.value}
              onClick={() => choose(item.value)}
            >
              {item.label}
            </button>
          ))}
        </div>
      </FilterBand>

      <Region
        label={trashOpen ? "LIXEIRA" : "SESSÕES"}
        count={
          trashOpen
            ? (trashed.length ? String(trashed.length) : undefined)
            : (filtered.length ? `${filtered.length} sessões · ${hoursOf(totalSeconds)}` : undefined)
        }
        className="flush"
      >
        {trashOpen ? (
          trashed.length ? (
            <div className="tempo-sessions">
              {trashed.map((entry) => (
                <div className="tempo-session" key={entry.id}>
                  <span>
                    <strong>{named(entry.projectId)}</strong>
                    <small>{dayOf(entry.startedAt)} · removida</small>
                  </span>
                  <span className="tempo-session-duration">{durationOf(entry.durationSeconds)}</span>
                  <span className="tempo-session-actions" data-always>
                    <Button variant="ghost" size="sm" onClick={() => void restore(entry)}>Restaurar</Button>
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <EmptyState>A lixeira está vazia. Sessões removidas ficam aqui até você restaurá-las.</EmptyState>
          )
        ) : (
          <TempoSessions
            variant="table"
            amounts={amounts}
            entries={filtered}
            projects={projects}
            onChanged={onChanged}
            receipt={receipt}
            onError={setNote}
          />
        )}
      </Region>
    </>
  );
}
