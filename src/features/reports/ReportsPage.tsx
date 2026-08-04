import { useMemo, useState } from "react";
import { Download, FileText, Printer, Receipt } from "lucide-react";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { saveTextFile } from "@/services/timeEntries";
import { exportInvoicePdf } from "@/services/app";
import { isoToDateInput } from "@/lib/datetime";
import {
  formatCurrency,
  formatDate,
  formatDuration,
  formatTime,
} from "@/lib/format";
import { amountForDuration } from "@/lib/money";
import { netDuration } from "@/lib/duration";
import { roundDuration, type RoundingConfig } from "@/lib/rounding";
import { ACTIVITY_TYPE_LABELS } from "@/lib/labels";
import { buildCsv } from "@/lib/csv";
import { groupBy } from "@/lib/reportTotals";
import { exportReportPdf } from "@/services/app";
import { PageHeader } from "@/components/ui/PageHeader";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Button } from "@/components/ui/Button";
import { Stat } from "@/components/ui/Stat";
import { Field, Input, Select } from "@/components/ui/Field";

export function ReportsPage() {
  const entries = useEntriesStore((s) => s.entries);
  const { clients, projects } = useCatalogStore();
  const settings = useSettingsStore((s) => s.settings);

  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [projectId, setProjectId] = useState("");
  const [clientId, setClientId] = useState("");
  const [adjustPct, setAdjustPct] = useState("0");

  function fmt(d: Date) {
    return isoToDateInput(d.toISOString());
  }
  function setToday() {
    const t = fmt(new Date());
    setFrom(t);
    setTo(t);
  }
  function setMonth(offset: number) {
    const now = new Date();
    const first = new Date(now.getFullYear(), now.getMonth() + offset, 1);
    const last = new Date(now.getFullYear(), now.getMonth() + offset + 1, 0);
    setFrom(fmt(first));
    setTo(fmt(last));
  }

  const rounding: RoundingConfig = {
    enabled: settings?.roundingEnabled ?? false,
    intervalMinutes: settings?.roundingIntervalMinutes ?? 15,
    mode: settings?.roundingMode ?? "nearest",
  };

  const projectOf = (id: string) => projects.find((p) => p.id === id);
  const projectName = (id: string) => projectOf(id)?.name ?? "—";
  const clientNameOf = (id: string) =>
    clients.find((c) => c.id === projectOf(id)?.clientId)?.name ?? "—";

  const filtered = useMemo(() => {
    const fromTime = from ? new Date(`${from}T00:00:00`).getTime() : -Infinity;
    const toTime = to ? new Date(`${to}T23:59:59.999`).getTime() : Infinity;
    return entries.filter((e) => {
      const t = new Date(e.startedAt).getTime();
      if (t < fromTime || t > toTime) return false;
      if (projectId && e.projectId !== projectId) return false;
      if (clientId && projectOf(e.projectId)?.clientId !== clientId) return false;
      return true;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entries, from, to, projectId, clientId, projects]);

  // Metricas por sessao (arredondamento aplicado apenas na visualizacao).
  const rows = filtered.map((e) => {
    const net = netDuration(e.durationSeconds, e.idleSeconds);
    const billableNet = e.billable ? net : 0;
    const roundedBillable = roundDuration(billableNet, rounding);
    return {
      entry: e,
      net,
      roundedBillable,
      amount: amountForDuration(roundedBillable, e.hourlyRateSnapshotCents),
    };
  });

  const totals = rows.reduce(
    (acc, r) => {
      acc.gross += r.entry.durationSeconds;
      acc.idle += r.entry.idleSeconds;
      acc.billable += r.roundedBillable;
      acc.amount += r.amount;
      return acc;
    },
    { gross: 0, idle: 0, billable: 0, amount: 0 },
  );

  // Ajuste percentual (desconto negativo / acrescimo positivo) sobre o total.
  const pct = Number(adjustPct.replace(",", ".")) || 0;
  const finalAmount = Math.round(totals.amount * (1 + pct / 100));
  const selectedClientName =
    clients.find((c) => c.id === clientId)?.name ?? null;

  // Quebras do periodo. Ambas somam as mesmas linhas que produzem o "Valor
  // total" acima — nao ha um segundo caminho de calculo que possa divergir.
  const byActivity = useMemo(() => groupBy(rows, (r) => r.entry.activityType), [rows]);
  const byProject = useMemo(() => groupBy(rows, (r) => r.entry.projectId), [rows]);

  async function exportCsv() {
    const headers = [
      "Data",
      "Cliente",
      "Projeto",
      "Atividade",
      "Inicio",
      "Fim",
      "Duracao (h)",
      "Inativo (h)",
      "Faturavel (h)",
      "Valor/hora",
      "Valor",
    ];
    const hours = (s: number) => (s / 3600).toFixed(2).replace(".", ",");
    const money = (c: number) => (c / 100).toFixed(2).replace(".", ",");
    const csvRows = rows.map((r) => {
      const e = r.entry;
      return [
        formatDate(e.startedAt),
        clientNameOf(e.projectId),
        projectName(e.projectId),
        ACTIVITY_TYPE_LABELS[e.activityType],
        formatTime(e.startedAt),
        e.endedAt ? formatTime(e.endedAt) : "",
        hours(e.durationSeconds),
        hours(e.idleSeconds),
        hours(r.roundedBillable),
        money(e.hourlyRateSnapshotCents),
        money(r.amount),
      ];
    });
    const csv = buildCsv(headers, csvRows);
    await saveTextFile(csv, "relatorio-cronocad.csv");
  }

  function periodLabel(): string {
    if (from && to) return `Periodo: ${from} a ${to}`;
    if (from) return `A partir de ${from}`;
    if (to) return `Ate ${to}`;
    return "Todos os periodos";
  }

  function totalsPairs(): [string, string][] {
    const base: [string, string][] = [
      ["Horas reais", formatDuration(totals.gross)],
      ["Horas inativas", formatDuration(totals.idle)],
      ["Horas faturaveis", formatDuration(totals.billable)],
    ];
    // Quebra por projeto: sem isso, uma fatura com varios projetos nao mostra
    // quanto cabe a cada um. Omitida quando ha um projeto so (seria repetir o
    // total) e quando ha muitos (viraria uma parede de linhas no cabecalho).
    if (byProject.length > 1 && byProject.length <= 12) {
      for (const g of byProject) {
        base.push([
          `  ${projectName(g.key)}`,
          `${formatDuration(g.seconds)} · ${formatCurrency(g.amount)}`,
        ]);
      }
    }
    if (pct !== 0) {
      base.push(["Subtotal", formatCurrency(totals.amount)]);
      base.push([`Ajuste (${pct}%)`, formatCurrency(finalAmount - totals.amount)]);
    }
    base.push(["Valor total", formatCurrency(finalAmount)]);
    return base;
  }

  const pdfRows = () =>
    rows.map(
      (r): [string, string, string, string] => [
        formatDate(r.entry.startedAt),
        projectName(r.entry.projectId),
        formatDuration(r.roundedBillable),
        formatCurrency(r.amount),
      ],
    );

  async function exportPdf() {
    await exportReportPdf(
      {
        title: "Relatorio CronoCAD",
        period: periodLabel(),
        totals: totalsPairs(),
        columns: ["Data", "Projeto", "Faturavel", "Valor"],
        rows: pdfRows(),
      },
      "relatorio-cronocad.pdf",
    );
  }

  async function exportInvoice() {
    if (!selectedClientName) return;
    await exportInvoicePdf(
      {
        issuerName: settings?.issuerName ?? "",
        issuerDocument: settings?.issuerDocument ?? "",
        issuerContact: settings?.issuerContact ?? "",
        clientName: selectedClientName,
        period: periodLabel(),
        columns: ["Data", "Projeto", "Faturavel", "Valor"],
        rows: pdfRows(),
        totalLabel: "Total a pagar",
        totalValue: formatCurrency(finalAmount),
      },
      `fatura-${selectedClientName.replace(/\s+/g, "-").toLowerCase()}.pdf`,
    );
  }

  return (
    <div>
      <PageHeader
        title="Relatorios"
        description="Horas reais, inativas, faturaveis e valores por periodo."
        action={
          <div className="flex gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={() => window.print()}
              icon={<Printer size={15} strokeWidth={1.75} />}
            >
              Imprimir
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void exportCsv()}
              disabled={rows.length === 0}
              icon={<Download size={15} strokeWidth={1.75} />}
            >
              Exportar CSV
            </Button>
            <Button
              variant="secondary"
              size="sm"
              onClick={() => void exportPdf()}
              disabled={rows.length === 0}
              icon={<FileText size={15} strokeWidth={1.75} />}
            >
              Exportar PDF
            </Button>
            <Button
              variant="primary"
              size="sm"
              onClick={() => void exportInvoice()}
              disabled={rows.length === 0 || !selectedClientName}
              title={
                selectedClientName
                  ? undefined
                  : "Selecione um cliente para gerar a fatura"
              }
              icon={<Receipt size={15} strokeWidth={1.75} />}
            >
              Gerar fatura
            </Button>
          </div>
        }
      />

      <Panel className="mb-4 p-4">
        <div className="grid gap-3 md:grid-cols-4">
          <Field label="De" htmlFor="r-from">
            <Input id="r-from" type="date" value={from} onChange={(e) => setFrom(e.target.value)} />
          </Field>
          <Field label="Ate" htmlFor="r-to">
            <Input id="r-to" type="date" value={to} onChange={(e) => setTo(e.target.value)} />
          </Field>
          <Field label="Cliente" htmlFor="r-client">
            <Select id="r-client" value={clientId} onChange={(e) => setClientId(e.target.value)}>
              <option value="">Todos</option>
              {clients.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </Select>
          </Field>
          <Field label="Projeto" htmlFor="r-project">
            <Select id="r-project" value={projectId} onChange={(e) => setProjectId(e.target.value)}>
              <option value="">Todos</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </Select>
          </Field>
        </div>
        <div className="mt-3 flex flex-wrap items-end justify-between gap-3">
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={setToday}>
              Hoje
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setMonth(0)}>
              Este mes
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setMonth(-1)}>
              Mes passado
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setFrom("");
                setTo("");
              }}
            >
              Tudo
            </Button>
          </div>
          <div className="w-40">
            <Field label="Ajuste (%)" htmlFor="r-adjust" hint="Desconto (-) ou acrescimo (+)">
              <Input
                id="r-adjust"
                inputMode="decimal"
                value={adjustPct}
                onChange={(e) => setAdjustPct(e.target.value)}
              />
            </Field>
          </div>
        </div>
        {rounding.enabled && (
          <p className="mt-3 text-2xs text-text-subtle">
            Arredondamento ativo ({rounding.intervalMinutes} min, {rounding.mode})
            aplicado apenas na visualizacao/cobranca — o tempo real e preservado.
          </p>
        )}
      </Panel>

      {/*
        4 colunas so a partir de `xl`: em `md` (768px) cada coluna ficaria com
        ~140px, e "R$ 12.480,83" (indivisivel — o Intl usa espaco nao-quebravel)
        vazava por cima da borda do card.
      */}
      <Panel className="mb-4 grid grid-cols-2 gap-6 p-6 xl:grid-cols-4">
        <Stat label="Horas reais" value={formatDuration(totals.gross)} />
        <Stat label="Horas inativas" value={formatDuration(totals.idle)} />
        <Stat label="Horas faturaveis" value={formatDuration(totals.billable)} />
        <Stat
          label="Valor total"
          value={formatCurrency(finalAmount)}
          hint={pct !== 0 ? `Ajuste de ${pct}%` : undefined}
        />
      </Panel>

      {/*
        `min-w-0` nos filhos: item de grid tem `min-width: auto`, entao a tabela
        de sessoes impedia a coluna de encolher e o painel vazava para fora da
        janela em larguras intermediarias (~1120px). O `overflow-x-auto` interno
        so funciona depois que o painel pode, de fato, ficar mais estreito.
      */}
      <div className="grid gap-4 lg:grid-cols-[1fr_1.6fr]">
        <div className="grid min-w-0 content-start gap-4">
          {/* `min-w-0` tambem aqui: sem isso o painel nao encolhe e o nome
              longo do projeto vaza por baixo do painel vizinho em vez de
              truncar. A cadeia inteira precisa poder encolher. */}
          <Panel className="min-w-0">
            <PanelHeader title="Por projeto" />
            {byProject.length === 0 ? (
              <p className="px-4 py-6 text-sm text-text-muted">Sem dados no periodo.</p>
            ) : (
              <ul className="divide-y divide-border">
                {byProject.map((g) => (
                  <li
                    key={g.key}
                    className="flex items-center justify-between gap-3 px-4 py-2.5"
                  >
                    <span className="min-w-0 truncate text-sm text-text">
                      {projectName(g.key)}
                    </span>
                    <span className="shrink-0 text-right">
                      <span className="tabular block text-sm text-text">
                        {formatCurrency(g.amount)}
                      </span>
                      <span className="tabular block text-2xs text-text-muted">
                        {formatDuration(g.seconds)}
                      </span>
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>

          <Panel className="min-w-0">
            <PanelHeader title="Por tipo de atividade" />
            {byActivity.length === 0 ? (
              <p className="px-4 py-6 text-sm text-text-muted">Sem dados no periodo.</p>
            ) : (
              <ul className="divide-y divide-border">
                {byActivity.map((g) => (
                  <li
                    key={g.key}
                    className="flex items-center justify-between gap-3 px-4 py-2.5"
                  >
                    <span className="min-w-0 truncate text-sm text-text">
                      {ACTIVITY_TYPE_LABELS[g.key as keyof typeof ACTIVITY_TYPE_LABELS]}
                    </span>
                    <span className="shrink-0 text-right">
                      <span className="tabular block text-sm text-text">
                        {formatCurrency(g.amount)}
                      </span>
                      <span className="tabular block text-2xs text-text-muted">
                        {formatDuration(g.seconds)}
                      </span>
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>
        </div>

        <Panel className="min-w-0">
          <PanelHeader title="Sessoes detalhadas" />
          {rows.length === 0 ? (
            <p className="px-4 py-6 text-sm text-text-muted">
              Nenhuma sessao no periodo/filtro selecionado.
            </p>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border text-left text-2xs uppercase tracking-wide text-text-subtle">
                    <th className="px-4 py-2 font-medium">Data</th>
                    <th className="px-4 py-2 font-medium">Projeto</th>
                    <th className="px-4 py-2 text-right font-medium">Faturavel</th>
                    <th className="px-4 py-2 text-right font-medium">Valor</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {rows.map((r) => (
                    <tr key={r.entry.id}>
                      <td className="tabular whitespace-nowrap px-4 py-2.5 text-text-muted">
                        {formatDate(r.entry.startedAt)}
                      </td>
                      <td className="px-4 py-2.5 text-text">
                        {projectName(r.entry.projectId)}
                      </td>
                      <td className="tabular whitespace-nowrap px-4 py-2.5 text-right text-text">
                        {formatDuration(r.roundedBillable)}
                      </td>
                      <td className="tabular whitespace-nowrap px-4 py-2.5 text-right text-text">
                        {formatCurrency(r.amount)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>
      </div>
    </div>
  );
}
