import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Card, ContextPath, EmptyState, PageHeader, Region, StatBand, Stat } from "./Surface";
import { TempoClients } from "./TempoClients";
import { TempoHistory } from "./TempoHistory";
import { TempoProjects } from "./TempoProjects";
import { TempoReports } from "./TempoReports";
import { TempoSettings } from "./TempoSettings";
import { TempoSessions } from "./TempoSessions";
import { TempoTimeline } from "./TempoTimeline";
import {
  DraftFields,
  durationOf,
  emptyDraft,
  hoursOf,
  moneyOf,
  momentOf,
  secondsOf,
  type Draft,
} from "./TempoShared";
import { Timer } from "./Timer";
import type { Project, ProjectTracking, TimeEntry, Totals } from "./types";

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
  const [tracking, setTracking] = useState<Record<string, ProjectTracking>>({});
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
  // O recado da importacao vive AQUI e nao no `note` da pagina.
  //
  // Isto foi um bug real: o botao "Importar agora" fica no topo, e o `note`
  // renderiza no rodape — depois do cronometro, da tabela, das sessoes, da
  // linha do tempo e das configuracoes. O backend recusava com "ja foi
  // importado" e a explicacao aparecia milhares de pixels abaixo da dobra. Da
  // cadeira do usuario, o botao estava morto.
  const [importNote, setImportNote] = useState("");

  const load = useCallback(async () => {
    /* O tracking entra aqui por causa do `paidAt`: sem ele o Painel nao sabe
       separar o que ja entrou do que ainda esta na rua, e o numero grande volta
       a somar as duas coisas. */
    const [nextTotals, nextEntries, nextTracking] = await Promise.all([
      api.trackingTotals().catch(() => ({}) as Record<string, Totals>),
      api.trackingEntries().catch(() => [] as TimeEntry[]),
      api.projectTracking().catch(() => [] as ProjectTracking[]),
    ]);
    setTotals(nextTotals);
    setEntries(nextEntries);
    setTracking(Object.fromEntries(nextTracking.map((item) => [item.projectId, item])));
  }, []);

  useEffect(() => { void load(); }, [load]);

  /**
   * Decide se o convite de importação faz sentido agora.
   *
   * A pergunta "já importei?" NÃO pode ser respondida com `catch(() => null)`.
   * Fazia isso antes, e "a checagem falhou" virava "nunca importou" — o convite
   * aparecia oferecendo uma ação que o backend ia recusar. Não saber e saber
   * que não são coisas diferentes, e na dúvida o certo é não convidar.
   */
  const checkImport = useCallback(async () => {
    let importedAt: string | null;
    try {
      importedAt = await api.cronocadImportedAt();
    } catch {
      setPendingImport(null);
      return;
    }
    if (importedAt) {
      setPendingImport(null);
      return;
    }
    // Ausente em dois casos: nunca importou, ou não há CronoCAD nesta máquina.
    // Só o primeiro merece convite, e o caminho é o que distingue os dois.
    setPendingImport(await api.defaultCronocadPath().catch(() => null));
  }, []);

  useEffect(() => { void checkImport(); }, [checkImport]);

  async function runImport() {
    if (!pendingImport) return;
    setImporting(true);
    setImportNote("");
    try {
      const report = await api.importCronocad(pendingImport);
      setPendingImport(null);
      setImportNote(
        `Importado: ${report.projects} projects · ${report.entries} sessões · ` +
        `${(report.trackedSeconds / 3600).toFixed(1)} h · ${report.activityEvents} eventos.`,
      );
      await load();
    } catch (error) {
      setImportNote(error instanceof Error ? error.message : String(error));
      // Recusou porque já estava importado? Então o convite está desatualizado,
      // e insistir em mostrá-lo convida o usuário a clicar de novo no mesmo
      // botão para receber o mesmo "não".
      await checkImport();
      await load();
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

  /* Duas colunas de dinheiro, e nao uma soma.
   *
   * "Acumulado R$ 780,00" somava o que ja foi pago com o que ainda nao — um
   * numero que nao responde nem "quanto rendeu" nem "quanto me devem". O
   * segundo e o que se olha num painel. */
  const trackedTotal = ranked.reduce((sum, [, total]) => sum + total.grossSeconds, 0);
  const aReceber = ranked.reduce(
    (sum, [id, total]) => (tracking[id]?.paidAt ? sum : sum + total.amountCents),
    0,
  );
  const jaPago = ranked.reduce(
    (sum, [id, total]) => (tracking[id]?.paidAt ? sum + total.amountCents : sum),
    0,
  );

  // Horas de hoje, em dia LOCAL. `toDateString` compara ano, mês e dia sem
  // passar por fuso — que é o que estraga a comparação depois das 21h.
  const todayLabel = new Date().toDateString();
  const todaySeconds = entries
    .filter((entry) => new Date(entry.startedAt).toDateString() === todayLabel)
    .reduce((sum, entry) => sum + Math.max(0, entry.durationSeconds), 0);

  const label = VIEWS.find((item) => item.value === view)?.label ?? "";

  return (
    <div className="page tempo-page">
      <ContextPath segments={["M", "CRONOCAD", label.toUpperCase()]} />

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

      {/* Resposta a UM clique de distância do botão que a provocou. Um recado no
          rodapé de uma página longa é um recado que não existe. */}
      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}

      {/* Faixa, e nao painel.
          O convite abria TODA sessao de trabalho com um bloco de tres paragrafos
          sobre migracao de banco — o assunto mais secundario que esta pagina
          tem, ocupando o lugar mais nobre dela. Como faixa de uma linha ele
          continua sendo impossivel de nao ver, e para de ser a primeira coisa
          que se le. O detalhe inteiro segue em Settings, onde quem quer ler
          sobre importacao vai procurar. */}
      {pendingImport ? (
        <div className="tempo-invite">
          <span className="micro-label">CRONOCAD ENCONTRADO</span>
          <span>
            Existe um banco do CronoCAD nesta máquina com horas que ainda não estão aqui. Ele é aberto{" "}
            <strong>somente para leitura</strong>, e a importação roda uma vez.
          </span>
          <Button variant="secondary" size="sm" disabled={importing} onClick={() => void runImport()}>
            {importing ? "Importando" : "Importar"}
          </Button>
        </div>
      ) : null}

      {/* O recibo sobrevive ao convite que o gerou: sem isto, o resultado
          desapareceria junto com o botao e o clique pareceria nao ter feito
          nada. */}
      {importNote ? (
        <p className="tempo-invite-note" aria-live="polite">{importNote}</p>
      ) : null}

      {view === "painel" ? (
        <>
          {/* O título visível saiu, e o `h1` ficou.
              O caminho ("M / CRONOCAD / PAINEL") e a aba acesa já diziam onde
              você está; um `<h1>Painel</h1>` logo abaixo era a terceira camada a
              dizer a mesma palavra em 130px de altura, e o subtítulo descrevia o
              LAYOUT ("cronômetro, resumo do dia e sessões recentes") em vez de
              dizer alguma coisa. O leitor de tela continua recebendo o título. */}
          <h1 className="visually-hidden">Painel</h1>

          {/* Os três números que mudam uma decisão, e só eles.
              "Sessões registradas" e "Projects ativos" saíram: nenhum dos dois
              muda o que se faz a seguir, e eram eles que faziam a coluna de
              números ficar tão alta quanto o cronômetro ao lado. Em faixa, e não
              em coluna, o olho compara os três sem descer. */}
          {/* Faixa, e nao card. A moldura em volta de tres numeros era uma
              caixa cujo unico conteudo era uma regua de leitura — e cardizar a
              regua foi o que a auditoria chamou pelo nome. As reguas verticais
              entre eles separam melhor do que a borda em volta dos tres. */}
          <StatBand>
            <Stat label="TRABALHADO HOJE" value={durationOf(todaySeconds)} />
            <Stat label="A RECEBER" value={moneyOf(aReceber)} hint={`${hoursOf(trackedTotal)} rastreadas`} />
            {/* So aparece quando ha: um "R$ 0,00 pago" fixo ocuparia a
                faixa todo dia para dizer nada. */}
            {jaPago ? <Stat label="JÁ PAGO" value={moneyOf(jaPago)} settled /> : null}
          </StatBand>

          {/* Duas colunas de peso diferente: o cronômetro é o que se usa, o
              acumulado por Project é o que se confere. Empilham numa janela
              estreita. */}
          <div className="tempo-cols" data-cols="main">
            {/* A UNICA superficie elevada do Painel, e e por isso que ela
                significa alguma coisa. Quando toda peca da tela tinha moldura,
                estar dentro de uma nao dizia nada; agora dizer "isto e o que
                voce veio fazer aqui" custa exatamente uma borda. */}
            {/* Sem rotulo no Card: quem rotula e o proprio cronometro, porque a
                palavra muda com o estado. "INICIAR TRABALHO" sobre uma sessao
                em curso seria a moldura contradizendo o conteudo. */}
            <Card>
              <Timer projects={projects} entries={entries} onChanged={() => void load()} detailed />

              {active.length ? (
                <details className="tempo-forgot">
                  {/* Dobrado, e não escondido em outra tela: lançar tempo esquecido
                      é a segunda coisa mais feita aqui, mas não a primeira — e
                      aberto o tempo todo ele competia com o cronômetro. */}
                  <summary>Esqueceu de registrar? Adicionar tempo</summary>
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
                </details>
              ) : null}
            </Card>

            {/* "Projects" e o vocabulario do M/OS, e ele fica: renomear para
                "Projetos" so aqui criaria dois nomes para a mesma entidade
                dentro da mesma tela — o rail, a pagina vizinha e o resto desta
                pagina dizem Project. */}
            {/* Sem `flush`: as celulas do card `flush` levam 24px de cada lado,
                e numa coluna de duas colunas isso custa 96px dos ~260 que a
                janela estreita da — o nome do Project truncava cedo e "16.1 h"
                caia para uma segunda linha. O respiro vem do corpo do card, uma
                vez so, e as celulas ficam com a largura toda. */}
            <Region label="POR PROJECT" count={trackedTotal ? hoursOf(trackedTotal) : undefined}>
              {ranked.length ? (
                <table className="tempo-table tempo-table-compact">
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
                          <td>
                            <strong>{moneyOf(total.amountCents)}</strong>
                            <small>{hoursOf(total.grossSeconds)}</small>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              ) : (
                <EmptyState>Nenhuma hora registrada ainda.</EmptyState>
              )}
            </Region>
          </div>

          {/* A lista ocupa a largura toda, e nao um terco dela.
              Espremida numa coluna de ~300px, cada sessao gastava duas alturas
              (nome em cima, apoio embaixo) e a lista virava a peca mais alta da
              tela ao lado de duas colunas que acabavam muito antes — era isso
              que fazia o rodape da pagina parecer desmontado. Larga, cada sessao
              cabe numa linha e a mesma quantidade de informacao ocupa metade da
              altura. */}
          <Region
            className="tempo-recent"
            label="SESSÕES RECENTES"
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
              <EmptyState>As sessões encerradas aparecem aqui.</EmptyState>
            )}
          </Region>

          {/* Era um card de um terco de largura com uma frase e um botao dentro:
              uma moldura em volta de um link. Como linha de rodape ele devolve a
              coluna a pagina e continua encontrando quem chegou ao fim da lista
              — que e exatamente onde a pergunta "faltou registrar alguma coisa?"
              aparece. */}
          <p className="tempo-timeline-cta">
            <span>Programas monitorados abertos hoje viram sessões: períodos sem registro podem ser lançados na linha do tempo.</span>
            <Button variant="ghost" size="sm" onClick={() => setView("linha")}>Abrir linha do tempo</Button>
          </p>
        </>
      ) : null}

      {view === "projetos" ? (
        <TempoProjects projects={projects} totals={totals} openProject={openProject} openClients={() => setView("config")} />
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
          <PageHeader
            title="Configurações"
            subtitle="Arredondamento, observação, clientes e emissor da fatura."
          />
          {/* Duas colunas: clientes e emissor de um lado, o que o sistema faz
              sozinho do outro. Sete cards numa coluna só era rolagem sem fim. */}
          <div className="tempo-cols" data-cols="2">
            <div className="tempo-stack">
              <TempoClients />
            </div>
            <div className="tempo-stack">
              <TempoSettings onChanged={() => void load()} />
            </div>
          </div>
        </>
      ) : null}
    </div>
  );
}
