import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Card, EmptyState, PageHeader, Stat } from "./Surface";
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
            <Button variant="ghost" size="sm" onClick={() => window.print()}>Imprimir</Button>
            <Button variant="ghost" size="sm" disabled={busy} onClick={() => void exportCsv()}>Exportar CSV</Button>
            <Button variant="ghost" size="sm" disabled={busy} onClick={() => void exportPdf()}>Exportar PDF</Button>
            <Button variant="primary" size="sm" disabled={busy || !clientId} onClick={() => void exportInvoice()}>
              Gerar fatura
            </Button>
          </>
        }
      />

      <Card>
        <div className="tempo-filters">
          <div className="tempo-field">
            <label htmlFor="rep-from">De</label>
            <input id="rep-from" type="date" value={from} onChange={(event) => setFrom(event.currentTarget.value)} />
          </div>
          <div className="tempo-field">
            <label htmlFor="rep-to">Até</label>
            <input id="rep-to" type="date" value={to} onChange={(event) => setTo(event.currentTarget.value)} />
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

        <div className="tempo-presets">
          {PRESETS.map((item) => (
            <button key={item.label} type="button" onClick={() => item.apply(setFrom, setTo)}>
              {item.label}
            </button>
          ))}
        </div>

        {!clientId ? (
          <p className="support-copy">A fatura sai por cliente — escolha um acima para habilitá-la.</p>
        ) : null}
      </Card>

      {filtered.length ? (
        <Card count={period}>
          {/* A linha de números é a resposta inteira do relatório: quem abre
              esta tela quer saber quanto vale o período, e o resto é detalhe de
              conferência. Por isso ela vem antes das tabelas, e não depois. */}
          <div className="tempo-stat-row">
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
          </div>
        </Card>
      ) : (
        <Card count={period}>
          <EmptyState>Nenhuma sessão neste recorte.</EmptyState>
        </Card>
      )}

      {filtered.length ? (
        <>
          <div className="tempo-cols" data-cols="2">
            <div className="tempo-stack">
              <Card label="POR PROJECT" className="flush">
                <table className="tempo-table tempo-table-compact">
                  <tbody>
                    {byProject.map((group) => (
                      <tr key={group.key}>
                        <th scope="row">{named(group.key)}</th>
                        <td>
                          <strong>{moneyOf(group.totals.amountCents)}</strong>
                          <small>{hoursOf(group.totals.billableSeconds)}</small>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Card>

              <Card label="POR TIPO DE ATIVIDADE" className="flush">
                <table className="tempo-table tempo-table-compact">
                  <tbody>
                    {byActivity.map((group) => (
                      <tr key={group.key}>
                        <th scope="row">{ACTIVITY_LABEL[group.key] ?? group.key}</th>
                        <td>
                          <strong>{moneyOf(group.totals.amountCents)}</strong>
                          <small>{hoursOf(group.totals.billableSeconds)}</small>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Card>
            </div>

            <Card label="SESSÕES DETALHADAS" count={String(filtered.length)} className="flush">
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
            </Card>
          </div>

        </>
      ) : null}
    </>
  );
}
