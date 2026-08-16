import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { ContextPath, EmptyState, Panel } from "./Surface";
import { Timer } from "./Timer";
import type { Project, TimeEntry, Totals } from "./types";

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

const ACTIVITY: Record<string, string> = {
  drawing: "desenho",
  detailing: "detalhamento",
  revision: "revisão",
  meeting: "reunião",
  study: "estudo",
  other: "outro",
};

/**
 * A página de Tempo.
 *
 * Três perguntas, nesta ordem: o que estou contando agora, quanto cada Project
 * acumulou, e o que exatamente aconteceu. Os totais vêm depois do cronômetro
 * porque quem abre esta página costuma estar começando ou conferindo, não
 * auditando.
 */
export function TempoPage({ projects, openProject }: {
  projects: Project[];
  openProject: (project: Project) => void;
}) {
  const [totals, setTotals] = useState<Record<string, Totals>>({});
  const [entries, setEntries] = useState<TimeEntry[]>([]);

  const load = useCallback(async () => {
    const [nextTotals, nextEntries] = await Promise.all([
      api.trackingTotals().catch(() => ({}) as Record<string, Totals>),
      api.trackingEntries().catch(() => [] as TimeEntry[]),
    ]);
    setTotals(nextTotals);
    setEntries(nextEntries);
  }, []);

  useEffect(() => { void load(); }, [load]);

  const named = (id: string) => projects.find((project) => project.id === id);

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

      <Panel
        label="POR PROJECT"
        count={trackedTotal ? hoursOf(trackedTotal) : undefined}
      >
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
                        diferentes — quanto eu trabalhei, e quanto eu cobro. O
                        banco guarda o primeiro; o segundo é o primeiro depois
                        da inatividade e do arredondamento. */}
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
                  {/* A origem só aparece quando NÃO é o cronômetro. "Reconstruída"
                      e "manual" são hora que o usuário estimou depois, e faturar
                      isso sem distinguir seria cobrar estimativa como medição —
                      a distinção veio do CronoCAD e sobreviveu à absorção. */}
                  <small>
                    {dayOf(entry.startedAt)} · {ACTIVITY[entry.activityType] ?? entry.activityType}
                    {entry.source === "reconstructed" ? " · reconstruída" : ""}
                    {entry.source === "manual" ? " · manual" : ""}
                    {entry.billable ? "" : " · não cobrável"}
                  </small>
                </span>
                <span className="tempo-session-duration">{durationOf(entry.durationSeconds)}</span>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState>As sessões encerradas aparecem aqui, da mais recente para a mais antiga.</EmptyState>
        )}
      </Panel>
    </div>
  );
}
