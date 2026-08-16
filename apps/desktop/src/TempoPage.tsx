import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { ContextPath, EmptyState, Panel } from "./Surface";
import { TempoClients } from "./TempoClients";
import { TempoHistory } from "./TempoHistory";
import { TempoProjects } from "./TempoProjects";
import { TempoReports } from "./TempoReports";
import { TempoSettings } from "./TempoSettings";
import { TempoSessions } from "./TempoSessions";
import { TempoTimeline } from "./TempoTimeline";
import {
  DraftFields,
  emptyDraft,
  hoursOf,
  moneyOf,
  momentOf,
  secondsOf,
  type Draft,
} from "./TempoShared";
import { Timer } from "./Timer";
import type { Project, TimeEntry, Totals } from "./types";

/**
 * As telas do Tempo.
 *
 * São as mesmas seis do CronoCAD, com os mesmos nomes. Trocar "Painel" por
 * "Visão geral" na travessia obrigaria quem usou o app por meses a reaprender
 * onde as coisas estão — e o ganho seria zero.
 */
type View = "painel" | "projetos" | "historico" | "linha" | "relatorios" | "config";

const VIEWS: { value: View; label: string }[] = [
  { value: "painel", label: "Painel" },
  { value: "projetos", label: "Projetos" },
  { value: "historico", label: "Histórico" },
  { value: "linha", label: "Linha do tempo" },
  { value: "relatorios", label: "Relatórios" },
  { value: "config", label: "Configurações" },
];

/** Quantas sessões o Painel mostra antes de mandar para o Histórico. */
const RECENT = 8;

/**
 * A página de Tempo.
 *
 * O Painel responde "o que está acontecendo agora": o cronômetro, o que esqueci
 * de lançar, quanto cada Project acumulou e as últimas sessões. As outras cinco
 * telas respondem perguntas que exigem recorte, e por isso não cabiam empilhadas
 * aqui — uma página que rola por seis assuntos não é uma página, é um depósito.
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
  const [view, setView] = useState<View>("painel");
  const [totals, setTotals] = useState<Record<string, Totals>>({});
  const [entries, setEntries] = useState<TimeEntry[]>([]);
  const [note, setNote] = useState("");
  const [choice, setChoice] = useState("");
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  // O convite so existe enquanto ha o que importar. Ele vive AQUI, e nao so em
  // Settings, porque quem abre a pagina de Tempo com ela vazia esta exatamente
  // na pergunta que a importacao responde — e um botao que so existe em outra
  // tela e um botao que nao acontece.
  const [pendingImport, setPendingImport] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);

  const load = useCallback(async () => {
    const [nextTotals, nextEntries] = await Promise.all([
      api.trackingTotals().catch(() => ({}) as Record<string, Totals>),
      api.trackingEntries().catch(() => [] as TimeEntry[]),
    ]);
    setTotals(nextTotals);
    setEntries(nextEntries);
  }, []);

  useEffect(() => { void load(); }, [load]);

  useEffect(() => {
    void (async () => {
      const [at, path] = await Promise.all([
        api.cronocadImportedAt().catch(() => null),
        api.defaultCronocadPath().catch(() => null),
      ]);
      // Nulo em dois casos que dao no mesmo para a tela: ja importou, ou nao ha
      // CronoCAD nesta maquina. Nos dois o convite nao faz sentido.
      setPendingImport(at ? null : path);
    })();
  }, []);

  async function runImport() {
    if (!pendingImport) return;
    setImporting(true);
    setNote("");
    try {
      const report = await api.importCronocad(pendingImport);
      setPendingImport(null);
      setNote(
        `Importado: ${report.projects} projects · ${report.entries} sessões · ` +
        `${(report.trackedSeconds / 3600).toFixed(1)} h · ${report.activityEvents} eventos.`,
      );
      await load();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
    setImporting(false);
  }

  async function record() {
    const seconds = secondsOf(draft);
    if (!choice || seconds <= 0) return;
    setNote("");
    try {
      await api.trackingRecord({
        projectId: choice,
        startedAt: momentOf(draft.day),
        durationSeconds: seconds,
        description: draft.description,
        activityType: draft.activityType,
        billable: draft.billable,
      });
      setDraft(emptyDraft());
      await load();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }

  const named = (id: string) => projects.find((project) => project.id === id);
  const active = projects.filter((project) => project.lifecycleState === "active");

  // Só Projects com tempo de verdade. O corte é um MINUTO, e não zero: um
  // cronômetro iniciado e parado por engano deixa uma sessão de segundos, e ela
  // aparecia aqui como "0,0 h · R$ 0,00" — uma linha que não responde a pergunta
  // da tabela e ainda sugere que o Project foi trabalhado.
  const ranked = Object.entries(totals)
    .filter(([, total]) => total.grossSeconds >= 60)
    .sort(([, left], [, right]) => right.grossSeconds - left.grossSeconds);

  const trackedTotal = ranked.reduce((sum, [, total]) => sum + total.grossSeconds, 0);
  const label = VIEWS.find((item) => item.value === view)?.label ?? "";

  return (
    <div className="page tempo-page">
      <ContextPath segments={["M", "TEMPO", label.toUpperCase()]} />

      {/* `nav` com `aria-current`, e não o padrão de abas do ARIA. Abas de
          verdade exigem `tabpanel`, `aria-controls` e navegação por setas; um
          `role="tab"` sem isso anuncia ao leitor de tela um comportamento que a
          tela não tem, o que é pior do que não anunciar nada. */}
      <nav className="tempo-nav" aria-label="Telas do Tempo">
        {VIEWS.map((item) => (
          <button
            key={item.value}
            type="button"
            aria-current={view === item.value ? "page" : undefined}
            className={view === item.value ? "current" : undefined}
            onClick={() => setView(item.value)}
          >
            {item.label}
          </button>
        ))}
      </nav>

      {pendingImport ? (
        <Panel label="CRONOCAD ENCONTRADO" rule>
          <div className="tempo-invite">
            <div>
              <p>Existe um banco do CronoCAD nesta máquina, e as horas dele ainda não estão aqui.</p>
              <p className="support-copy">
                Vêm projetos, sessões, pendências, programas monitorados, o histórico observado pelo sistema e a
                sua configuração de arredondamento. O banco de origem é aberto <strong>somente para leitura</strong> —
                o CronoCAD continua intacto, e você compara o total antes de desinstalar. Roda uma vez.
              </p>
            </div>
            <Button variant="primary" size="sm" disabled={importing} onClick={() => void runImport()}>
              {importing ? "Importando" : "Importar agora"}
            </Button>
          </div>
        </Panel>
      ) : null}

      {view === "painel" ? (
        <>
          <Panel label="CRONÔMETRO" rule>
            <Timer projects={projects} entries={entries} onChanged={() => void load()} />
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
              <EmptyState>Nenhuma hora registrada ainda. Inicie o cronômetro acima, ou importe o CronoCAD em Configurações.</EmptyState>
            )}
          </Panel>

          <Panel
            label="ÚLTIMAS SESSÕES"
            count={entries.length > RECENT ? `${RECENT} de ${entries.length}` : undefined}
            action={
              entries.length > RECENT ? (
                <Button variant="ghost" size="sm" onClick={() => setView("historico")}>Ver todas</Button>
              ) : undefined
            }
          >
            {entries.length ? (
              <TempoSessions
                entries={entries.slice(0, RECENT)}
                projects={projects}
                onChanged={() => void load()}
                receipt={receipt}
                onError={setNote}
              />
            ) : (
              <EmptyState>As sessões encerradas aparecem aqui, da mais recente para a mais antiga.</EmptyState>
            )}
          </Panel>
        </>
      ) : null}

      {view === "projetos" ? (
        <TempoProjects projects={projects} totals={totals} openProject={openProject} />
      ) : null}

      {view === "historico" ? (
        <TempoHistory
          projects={projects}
          entries={entries}
          onChanged={() => void load()}
          receipt={receipt}
        />
      ) : null}

      {view === "linha" ? <TempoTimeline projects={projects} onChanged={() => void load()} /> : null}

      {view === "relatorios" ? <TempoReports projects={projects} /> : null}

      {view === "config" ? (
        <>
          <TempoClients />
          <TempoSettings onChanged={() => void load()} />
        </>
      ) : null}

      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}
    </div>
  );
}
