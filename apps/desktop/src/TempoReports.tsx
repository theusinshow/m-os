import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { EmptyState, Panel } from "./Surface";
import {
  ACTIVITY_LABEL,
  dateInputOf,
  dayOf,
  endOfDay,
  hoursOf,
  moneyOf,
  SOURCE_LABEL,
  startOfDay,
} from "./TempoShared";
import type { Client, Project, ProjectTracking, ReportLine, Totals } from "./types";

const EMPTY: Totals = { grossSeconds: 0, idleSeconds: 0, billableSeconds: 0, amountCents: 0 };

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
      <Panel label="RECORTE" rule>
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
      </Panel>

      <Panel label="RESUMO" count={period}>
        {filtered.length ? (
          <>
            <div className="tempo-stats">
              <div>
                <span className="micro-label">TRABALHADO</span>
                <strong>{hoursOf(totals.grossSeconds)}</strong>
              </div>
              <div>
                <span className="micro-label">COBRÁVEL</span>
                <strong>{hoursOf(totals.billableSeconds)}</strong>
              </div>
              <div>
                <span className="micro-label">{percent ? "VALOR BRUTO" : "VALOR"}</span>
                <strong>{moneyOf(totals.amountCents)}</strong>
              </div>
              {/* O ajustado aparece ao LADO do bruto, nunca no lugar dele: quem
                  vai cobrar precisa ver os dois para conferir o desconto. */}
              {percent ? (
                <div>
                  <span className="micro-label">{percent > 0 ? `COM +${percent}%` : `COM ${percent}%`}</span>
                  <strong>{moneyOf(finalAmount)}</strong>
                </div>
              ) : (
                <div>
                  <span className="micro-label">SESSÕES</span>
                  <strong>{filtered.length}</strong>
                </div>
              )}
            </div>
            <div className="form-actions tempo-exports">
              <Button variant="ghost" size="sm" disabled={busy} onClick={() => void exportCsv()}>Exportar CSV</Button>
              <Button variant="ghost" size="sm" disabled={busy} onClick={() => void exportPdf()}>Exportar PDF</Button>
              {/* Impressão do sistema: no Windows ela abre a caixa com
                  "Microsoft Print to PDF", então também é uma segunda saída em
                  papel para quem prefere ver antes de salvar. */}
              <Button variant="ghost" size="sm" onClick={() => window.print()}>Imprimir</Button>
              <Button variant="primary" size="sm" disabled={busy || !clientId} onClick={() => void exportInvoice()}>
                Fatura do cliente
              </Button>
            </div>
            {!clientId ? (
              <p className="support-copy">A fatura sai por cliente — escolha um acima para habilitá-la.</p>
            ) : null}
          </>
        ) : (
          <EmptyState>Nenhuma sessão neste recorte.</EmptyState>
        )}
      </Panel>

      {filtered.length ? (
        <>
          <Panel label="POR PROJECT">
            <table className="tempo-table">
              <thead>
                <tr>
                  <th scope="col">Project</th>
                  <th scope="col">Sessões</th>
                  <th scope="col">Cobrável</th>
                  <th scope="col">Valor</th>
                </tr>
              </thead>
              <tbody>
                {byProject.map((group) => (
                  <tr key={group.key}>
                    <th scope="row">{named(group.key)}</th>
                    <td>{group.lines.length}</td>
                    <td>{hoursOf(group.totals.billableSeconds)}</td>
                    <td>{moneyOf(group.totals.amountCents)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>

          <Panel label="POR ATIVIDADE">
            <table className="tempo-table">
              <thead>
                <tr>
                  <th scope="col">Atividade</th>
                  <th scope="col">Sessões</th>
                  <th scope="col">Cobrável</th>
                  <th scope="col">Valor</th>
                </tr>
              </thead>
              <tbody>
                {byActivity.map((group) => (
                  <tr key={group.key}>
                    <th scope="row">{ACTIVITY_LABEL[group.key] ?? group.key}</th>
                    <td>{group.lines.length}</td>
                    <td>{hoursOf(group.totals.billableSeconds)}</td>
                    <td>{moneyOf(group.totals.amountCents)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>

          <Panel label="LINHAS" count={String(filtered.length)}>
            <table className="tempo-table tempo-table-lines">
              <thead>
                <tr>
                  <th scope="col">Data</th>
                  <th scope="col">Project</th>
                  <th scope="col">Atividade</th>
                  <th scope="col">Cobrável</th>
                  <th scope="col">Valor</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((line) => (
                  <tr key={line.entryId}>
                    <th scope="row">{dayOf(line.startedAt)}</th>
                    <td>{named(line.projectId)}</td>
                    <td>{ACTIVITY_LABEL[line.activityType] ?? line.activityType}</td>
                    <td>{hoursOf(line.totals.billableSeconds)}</td>
                    <td>{moneyOf(line.totals.amountCents)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>
        </>
      ) : null}

      {note ? <p className="settings-message" aria-live="polite">{note}</p> : null}
    </>
  );
}
