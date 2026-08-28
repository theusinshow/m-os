import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { EmptyState, FilterBand, PageHeader, Region, Share, Stat, StatBand } from "./Surface";
import {
  ACTIVITY_LABEL,
  dateInputOf,
  endOfDay,
  hoursOf,
  moneyOf,
  SOURCE_LABEL,
  startOfDay,
} from "./TempoShared";
import type { Client, Project, ProjectTracking, ReportLine, Totals } from "./types";

const EMPTY: Totals = { grossSeconds: 0, idleSeconds: 0, billableSeconds: 0, amountCents: 0 };

/** `2026-08-16` no fuso LOCAL. */
const iso = (date: Date) => dateInputOf(date.toISOString());

/**
 * Os recortes que se pede na prática.
 *
 * Cada um preenche AS DUAS datas, em vez de esconder o intervalo: quem vai
 * faturar precisa ver de quando até quando está somando, e um atalho que some
 * com o campo obriga a confiar sem conferir.
 */
const PRESETS: { label: string; apply: (setFrom: (v: string) => void, setTo: (v: string) => void) => void }[] = [
  {
    label: "Hoje",
    apply: (setFrom, setTo) => {
      const today = iso(new Date());
      setFrom(today);
      setTo(today);
    },
  },
  {
    label: "Este mês",
    apply: (setFrom, setTo) => {
      const now = new Date();
      setFrom(iso(new Date(now.getFullYear(), now.getMonth(), 1)));
      setTo(iso(now));
    },
  },
  {
    label: "Mês passado",
    apply: (setFrom, setTo) => {
      const now = new Date();
      setFrom(iso(new Date(now.getFullYear(), now.getMonth() - 1, 1)));
      setTo(iso(new Date(now.getFullYear(), now.getMonth(), 0)));
    },
  },
  {
    label: "Tudo",
    apply: (setFrom, setTo) => {
      setFrom("");
      setTo("");
    },
  },
];

function sum(lines: ReportLine[]): Totals {
  return lines.reduce(
    (acc, line) => ({
      grossSeconds: acc.grossSeconds + line.totals.grossSeconds,
      idleSeconds: acc.idleSeconds + line.totals.idleSeconds,
      billableSeconds: acc.billableSeconds + line.totals.billableSeconds,
      amountCents: acc.amountCents + line.totals.amountCents,
    }),
    EMPTY,
  );
}

/**
 * Agrupa as linhas por uma chave, do maior valor para o menor.
 *
 * Opera sobre as MESMAS linhas que produzem o total geral, então a soma dos
 * grupos fecha com o total por construção — não existe um segundo caminho de
 * cálculo que possa divergir.
 */
function groupBy(lines: ReportLine[], keyOf: (line: ReportLine) => string) {
  const groups = new Map<string, { key: string; lines: ReportLine[]; totals: Totals }>();
  for (const line of lines) {
    const key = keyOf(line);
    const current = groups.get(key) ?? { key, lines: [], totals: EMPTY };
    current.lines.push(line);
    groups.set(key, current);
  }
  return [...groups.values()]
    .map((group) => ({ ...group, totals: sum(group.lines) }))
    .sort((left, right) => right.totals.amountCents - left.totals.amountCents);
}

/** Escapa uma célula de CSV: aspas dobradas, e o campo inteiro entre aspas. */
function cell(value: string) {
  return `"${value.replace(/"/g, '""')}"`;
}

function buildCsv(headers: string[], rows: string[][]) {
  return [headers, ...rows].map((row) => row.map(cell).join(";")).join("\r\n");
}

/** `01/08/2026` — a fatura vai para outra pessoa, e ali data curta confunde. */
function fullDay(iso: string) {
  return new Date(iso).toLocaleDateString("pt-BR");
}

/**
 * O relatório: quanto vale um período, por Project e por atividade — e o
 * arquivo que sai daqui para quem paga.
 *
 * Recalcular no backend a cada mudança de recorte, em vez de filtrar em memória
 * o que o Painel já carregou, é deliberado: o arredondamento acontece POR
 * SESSÃO, e somar depois de filtrar dá um número diferente de filtrar depois de
 * somar. O certo é o primeiro, e ele exige a conta refeita.
 */
export function TempoReports({ projects }: { projects: Project[] }) {
  const today = dateInputOf(new Date().toISOString());
  const monthStart = dateInputOf(new Date(new Date().getFullYear(), new Date().getMonth(), 1).toISOString());

  const [from, setFrom] = useState(monthStart);
  const [to, setTo] = useState(today);
  const [projectId, setProjectId] = useState("");
  const [clientId, setClientId] = useState("");
  const [adjust, setAdjust] = useState("0");
  /* Qual atalho de periodo esta aceso. Comeca em "Este mes" porque e o recorte
     que `from`/`to` ja carregam na primeira abertura — sem isso a tela abriria
     num periodo que nenhum atalho reivindica. */
  const [preset, setPreset] = useState<string | null>("Este mês");
  const [lines, setLines] = useState<ReportLine[]>([]);
  const [clients, setClients] = useState<Client[]>([]);
  const [tracking, setTracking] = useState<ProjectTracking[]>([]);
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    const rows = await api
      .trackingReport(from ? startOfDay(from) : null, to ? endOfDay(to) : null)
      .catch(() => [] as ReportLine[]);
    setLines(rows);
  }, [from, to]);

  useEffect(() => { void load(); }, [load]);

  useEffect(() => {
    void (async () => {
      const [people, billing] = await Promise.all([
        api.clients(true).catch(() => [] as Client[]),
        api.projectTracking().catch(() => [] as ProjectTracking[]),
      ]);
      setClients(people);
      setTracking(billing);
    })();
  }, []);

  const projectsOfClient = useMemo(() => {
    if (!clientId) return null;
    return new Set(tracking.filter((row) => row.clientId === clientId).map((row) => row.projectId));
  }, [clientId, tracking]);

  const filtered = useMemo(
    () => lines.filter((line) => {
      if (projectId && line.projectId !== projectId) return false;
      if (projectsOfClient && !projectsOfClient.has(line.projectId)) return false;
      return true;
    }),
    [lines, projectId, projectsOfClient],
  );

  const totals = sum(filtered);

  /**
   * Desconto (negativo) ou acréscimo (positivo) sobre o total.
   *
   * Vive só aqui, e nunca no banco: é uma negociação de uma fatura — "tira 10%
   * desse mês" — e não uma correção do que foi trabalhado. Gravá-lo faria a
   * próxima leitura das mesmas horas dar outro número.
   *
   * Vírgula aceita porque o teclado brasileiro escreve assim.
   */
  const percent = Number(adjust.replace(",", ".")) || 0;
  const finalAmount = Math.round(totals.amountCents * (1 + percent / 100));

  const named = (id: string) => projects.find((project) => project.id === id)?.name ?? "Project removido";
  const byProject = groupBy(filtered, (line) => line.projectId);
  const byActivity = groupBy(filtered, (line) => line.activityType);

  /**
   * A fatia de um grupo dentro do periodo, em horas FATURAVEIS.
   *
   * Faturaveis e nao brutas de proposito: as duas quebras vivem ao lado do
   * numero que vai para a fatura, e uma barra medida em horas brutas mostraria
   * uma proporcao que nao e a do dinheiro ali do lado.
   *
   * Denominador zero devolve zero — um periodo sem hora faturavel nao tem
   * proporcao para exibir, e `0/0` viraria `NaN%` na tela.
   */
  const shareOf = (seconds: number) =>
    totals.billableSeconds > 0 ? seconds / totals.billableSeconds : 0;

  const period = from || to
    ? `${from ? fullDay(startOfDay(from)) : "início"} a ${to ? fullDay(endOfDay(to)) : "hoje"}`
    : "Todo o período";

  /** As quatro colunas que o PDF e o resumo compartilham. */
  const pdfRows = (): [string, string, string, string][] =>
    filtered.map((line) => [
      fullDay(line.startedAt),
      named(line.projectId),
      hoursOf(line.totals.billableSeconds),
      moneyOf(line.totals.amountCents),
    ]);

  async function run(label: string, action: () => Promise<boolean>) {
    setBusy(true);
    setNote("");
    try {
      // `false` = o usuário fechou o diálogo. Dizer "exportado" nesse caso
      // mandaria ele procurar um arquivo que não existe.
      setNote(await action() ? `${label} salvo.` : "Exportação cancelada.");
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
    setBusy(false);
  }

  function exportCsv() {
    const headers = ["Data", "Project", "Atividade", "Origem", "Cobrável", "Trabalhado (h)", "Cobrável (h)", "Valor", "Descrição"];
    const rows = filtered.map((line) => [
      fullDay(line.startedAt),
      named(line.projectId),
      ACTIVITY_LABEL[line.activityType] ?? line.activityType,
      SOURCE_LABEL[line.source] ?? line.source,
      line.billable ? "sim" : "não",
      // Vírgula decimal: o Excel em português lê ponto como separador de milhar,
      // e "2.5" vira vinte e cinco.
      (line.totals.grossSeconds / 3600).toFixed(2).replace(".", ","),
      (line.totals.billableSeconds / 3600).toFixed(2).replace(".", ","),
      (line.totals.amountCents / 100).toFixed(2).replace(".", ","),
      line.description,
    ]);
    return run("CSV", () => api.exportCsv(buildCsv(headers, rows), "relatorio-tempo.csv"));
  }

  /** O resumo do PDF. O ajuste só entra quando existe — zero por cento numa
   *  linha do relatório é uma linha que não informa nada. */
  const pdfTotals = (): [string, string][] => {
    const rows: [string, string][] = [
      ["Trabalhado", hoursOf(totals.grossSeconds)],
      ["Cobravel", hoursOf(totals.billableSeconds)],
      ["Valor", moneyOf(totals.amountCents)],
    ];
    if (percent) {
      rows.push([`Ajuste (${percent}%)`, moneyOf(finalAmount)]);
    }
    return rows;
  };

  function exportPdf() {
    return run("PDF", () => api.exportReportPdf({
      title: "Relatório de horas",
      period,
      totals: pdfTotals(),
      columns: ["Data", "Project", "Cobrável", "Valor"],
      rows: pdfRows(),
    }, "relatorio-tempo.pdf"));
  }

  async function exportInvoice() {
    const client = clients.find((item) => item.id === clientId);
    if (!client) {
      setNote("Escolha um cliente para emitir a fatura.");
      return;
    }
    const issuer = await api.trackingIssuer().catch(() => ({ name: "", document: "", contact: "" }));
    const slug = client.name.replace(/\s+/g, "-").toLowerCase();
    await run("Fatura", () => api.exportInvoicePdf({
      issuerName: issuer.name,
      issuerDocument: issuer.document,
      issuerContact: issuer.contact,
      clientName: client.companyName || client.name,
      period,
      columns: ["Data", "Project", "Cobrável", "Valor"],
      rows: pdfRows(),
      // O ajuste é dito no rótulo, e não escondido no número: uma fatura cujo
      // total não fecha com a soma das linhas parece erro de conta.
      totalLabel: percent ? `Total com ${percent}%` : "Total",
      totalValue: moneyOf(percent ? finalAmount : totals.amountCents),
    }, `fatura-${slug}.pdf`));
  }

  return (
    <>
      {/* Acima e não abaixo: "CSV salvo" e "Exportação cancelada" são a resposta
          a um botão que fica no topo, e o rodapé desta página está três tabelas
          adiante. Um recibo que exige rolar é um recibo que ninguém lê. */}
      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}

      <PageHeader
        title="Relatórios"
        subtitle="Horas reais, faturáveis e valores por período."
        actions={
          <>
            {/* `outline`, e nao `ghost`: sem contorno os tres liam como link de
                texto ao lado do botao de fatura, e sair um PDF de um texto que
                parece rotulo e o tipo de acao que ninguem encontra. Contorno
                sem preenchimento os poe como controle secundario, que e o que
                eles sao ao lado de "Gerar fatura". */}
            <Button variant="outline" size="sm" onClick={() => window.print()}>Imprimir</Button>
            <Button variant="outline" size="sm" disabled={busy} onClick={() => void exportCsv()}>Exportar CSV</Button>
            <Button variant="outline" size="sm" disabled={busy} onClick={() => void exportPdf()}>Exportar PDF</Button>
            <Button variant="primary" size="sm" disabled={busy || !clientId} onClick={() => void exportInvoice()}>
              Gerar fatura
            </Button>
          </>
        }
      />

      <FilterBand>
        <div className="tempo-filters">
          <div className="tempo-field">
            <label htmlFor="rep-from">De</label>
            <input id="rep-from" type="date" value={from} onChange={(event) => { setFrom(event.currentTarget.value); setPreset(null); }} />
          </div>
          <div className="tempo-field">
            <label htmlFor="rep-to">Até</label>
            <input id="rep-to" type="date" value={to} onChange={(event) => { setTo(event.currentTarget.value); setPreset(null); }} />
          </div>
          <div className="tempo-field">
            <label htmlFor="rep-client">Cliente</label>
            <select id="rep-client" value={clientId} onChange={(event) => setClientId(event.currentTarget.value)}>
              <option value="">Todos</option>
              {clients.map((client) => <option key={client.id} value={client.id}>{client.name}</option>)}
            </select>
          </div>
          <div className="tempo-field">
            <label htmlFor="rep-project">Project</label>
            <select id="rep-project" value={projectId} onChange={(event) => setProjectId(event.currentTarget.value)}>
              <option value="">Todos</option>
              {projects.map((project) => <option key={project.id} value={project.id}>{project.name}</option>)}
            </select>
          </div>
          <div className="tempo-field">
            <label htmlFor="rep-adjust">Ajuste (%)</label>
            {/* Texto e não `number`: aceita vírgula, que é como se escreve
                decimal em português, e aceita o sinal de menos do desconto. */}
            <input
              id="rep-adjust"
              inputMode="decimal"
              value={adjust}
              onChange={(event) => setAdjust(event.currentTarget.value)}
            />
          </div>
        </div>

        {/* O atalho aceso, e nao quatro rotulos iguais.
            O Historico ja marcava o periodo escolhido com `aria-pressed`, e aqui
            os quatro ficavam apagados o tempo todo — a mesma peca dizia o estado
            numa tela e nao dizia na outra. Quem volta ao relatorio depois de
            mexer nas datas nao tinha como saber de que recorte estava olhando
            sem reler as duas datas.

            Escolher um atalho preenche as datas; editar uma data a mao apaga o
            atalho, porque a partir dali o recorte deixou de ser aquele. */}
        <div className="tempo-presets">
          {PRESETS.map((item) => (
            <button
              key={item.label}
              type="button"
              aria-pressed={preset === item.label}
              onClick={() => { item.apply(setFrom, setTo); setPreset(item.label); }}
            >
              {item.label}
            </button>
          ))}
        </div>

        {!clientId ? (
          <p className="support-copy">A fatura sai por cliente — escolha um acima para habilitá-la.</p>
        ) : null}
      </FilterBand>

      {filtered.length ? (
        <>
          {/* O recorte vira legenda da faixa, e nao contagem de card: ele diz
              de QUE periodo sao os numeros logo abaixo, e essa e a primeira
              pergunta de quem olha um relatorio. */}
          <p className="tempo-period">{period}</p>
          {/* A linha de números é a resposta inteira do relatório: quem abre
              esta tela quer saber quanto vale o período, e o resto é detalhe de
              conferência. Por isso ela vem antes das tabelas, e não depois. */}
          <StatBand>
            <Stat label="HORAS REAIS" value={hoursOf(totals.grossSeconds)} />
            <Stat label="HORAS INATIVAS" value={hoursOf(totals.idleSeconds)} />
            <Stat label="HORAS FATURÁVEIS" value={hoursOf(totals.billableSeconds)} />
            {/* O ajustado aparece ao LADO do bruto, nunca no lugar dele: quem
                vai cobrar precisa ver os dois para conferir o desconto. */}
            <Stat
              label={percent ? "VALOR BRUTO" : "VALOR TOTAL"}
              value={moneyOf(totals.amountCents)}
              hint={`${filtered.length} sessões`}
            />
            {percent ? (
              <Stat
                label={percent > 0 ? `COM +${percent}%` : `COM ${percent}%`}
                value={moneyOf(finalAmount)}
              />
            ) : null}
          </StatBand>
        </>
      ) : (
        <>
          <p className="tempo-period">{period}</p>
          <EmptyState>Nenhuma sessão neste recorte.</EmptyState>
        </>
      )}

      {filtered.length ? (
        <>
          <div className="tempo-cols" data-cols="2">
            <div className="tempo-stack">
              {/* Barra de proporcao, e nao so o numero.
                  "R$ 247,50" e "R$ 52,50" exigem uma subtracao mental para virar
                  "a maior parte do mes foi outro, nao desenho" — que e a unica
                  coisa que esta quebra existe para responder. A barra entrega a
                  comparacao antes da leitura.

                  Ela e NEUTRA de proposito: proporcao e quantidade, e quantidade
                  nao e sinal. Em sodio, ela seria a segunda cor de sinal da tela
                  e disputaria com o botao que emite a fatura. */}
              <Region label="POR PROJECT" count={String(byProject.length)}>
                {byProject.map((group) => (
                  <Share
                    key={group.key}
                    name={named(group.key)}
                    value={moneyOf(group.totals.amountCents)}
                    hours={hoursOf(group.totals.billableSeconds)}
                    share={shareOf(group.totals.billableSeconds)}
                  />
                ))}
              </Region>

              <Region label="POR TIPO DE ATIVIDADE" count={String(byActivity.length)}>
                {byActivity.map((group) => (
                  <Share
                    key={group.key}
                    name={ACTIVITY_LABEL[group.key] ?? group.key}
                    value={moneyOf(group.totals.amountCents)}
                    hours={hoursOf(group.totals.billableSeconds)}
                    share={shareOf(group.totals.billableSeconds)}
                  />
                ))}
              </Region>
            </div>

            <Region label="SESSÕES DETALHADAS" count={String(filtered.length)}>
              <table className="tempo-table tempo-table-lines">
                <thead>
                  <tr>
                    <th scope="col">Data</th>
                    <th scope="col">Project</th>
                    <th scope="col">Faturável</th>
                    <th scope="col">Valor</th>
                  </tr>
                </thead>
                <tbody>
                  {filtered.map((line) => (
                    <tr key={line.entryId}>
                      <th scope="row">{fullDay(line.startedAt)}</th>
                      <td>{named(line.projectId)}</td>
                      <td>{hoursOf(line.totals.billableSeconds)}</td>
                      <td>{moneyOf(line.totals.amountCents)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </Region>
          </div>

        </>
      ) : null}
    </>
  );
}
