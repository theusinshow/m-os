import { CheckCircle2, ChevronDown } from "lucide-react";
import { AttentionStrip } from "@/components/dashboard/attention-strip";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { AlertsPanel } from "@/components/dashboard/alerts-panel";
import { IncomeFormCard } from "@/components/dashboard/income-form-card";
import { InvoiceSummaryCard } from "@/components/dashboard/invoice-summary-card";
import { MonthGenerationReviewCard } from "@/components/dashboard/month-generation-review-card";
import { QuickActionButton } from "@/components/quick-action-button";
import { StatusBadge } from "@/components/status-badge";
import { UpcomingBillsList } from "@/components/dashboard/upcoming-bills-list";
import { BalanceDisplay } from "@/components/dashboard/balance-display";
import { CategoryBreakdownChart } from "@/components/charts/category-breakdown-chart";
import { MetricSparkline } from "@/components/charts/metric-sparkline";
import { MonthWaterfallChart } from "@/components/charts/month-waterfall-chart";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { calculateInternalAlerts } from "@/lib/calculations/alerts";
import { getDashboardSummary } from "@/lib/calculations/dashboard";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatMonthLabel } from "@/lib/formatters/date";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser, isViewingCurrentMonth } from "@/lib/active-month";
import { getMonthlySnapshots } from "@/lib/history";
import { getIncomesByMonth } from "@/lib/incomes";
import { getBillCategories, getBillsByMonth, getRecurringBillsByMonth } from "@/lib/bills";
import { getInvoicesByMonth } from "@/lib/cards";
import { getNextMonthForUser } from "@/lib/months";
import { getSettingsForUser } from "@/lib/settings";

export default async function DashboardPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const currentMonth = appUser ? await getActiveMonthForUser(appUser.id) : null;
  const viewingCurrent = await isViewingCurrentMonth();
  const nextMonth = appUser ? await getNextMonthForUser(appUser.id) : null;
  const realIncomes = currentMonth ? await getIncomesByMonth(currentMonth.id) : [];
  const realBills = currentMonth ? await getBillsByMonth(currentMonth.id) : [];
  const recurringBills = currentMonth ? await getRecurringBillsByMonth(currentMonth.id) : [];
  const realInvoices = currentMonth ? await getInvoicesByMonth(currentMonth.id) : [];
  const categories = appUser ? await getBillCategories(appUser.id) : [];
  const settings = appUser ? await getSettingsForUser(appUser.id) : null;
  const snapshots = appUser ? await getMonthlySnapshots(appUser.id) : [];
  // Os snapshots vêm do mais novo para o mais antigo; a linha lê da esquerda
  // para a direita, então a série vai ao contrário.
  const history = [...snapshots].reverse();
  const summary = getDashboardSummary({
    incomes: realIncomes,
    bills: realBills,
    invoices: realInvoices,
  });
  const categoryData = Object.values(
    realBills.reduce<Record<string, { name: string; value: number }>>((acc, bill) => {
      const name = bill.categoryName ?? "Sem categoria";
      acc[name] ??= { name, value: 0 };
      acc[name].value += bill.amountCents;
      return acc;
    }, {}),
  );
  const totalOutstandingCents = summary.totalPendingCents + summary.totalOverdueCents;
  const totalCommittedCents = summary.totalBillsCents + summary.totalInvoicesCents;
  const allSettled = totalCommittedCents > 0 && totalOutstandingCents === 0;
  const alerts = calculateInternalAlerts([
    ...realBills.map((bill) => ({
      id: bill.id,
      type: "bill" as const,
      title: bill.name,
      amountCents: bill.amountCents,
      dueDate: bill.dueDate,
      status: bill.status,
    })),
    ...realInvoices.map((invoice) => ({
      id: invoice.id,
      type: "invoice" as const,
      title: invoice.name,
      amountCents: invoice.amountCents,
      dueDate: invoice.dueDate,
      status: invoice.status,
    })),
  ], { daysBefore: settings?.alertDaysBefore ?? 3 });
  const monthMetrics = [
    {
      label: "Receita prevista",
      value: summary.totalIncomeCents,
      note: `${realIncomes.length} entrada${realIncomes.length === 1 ? "" : "s"}`,
      points: history.map((snapshot) => snapshot.totalIncomeCents),
      tone: "neutral" as const,
    },
    {
      label: "Comprometido",
      value: totalCommittedCents,
      note: "Contas e faturas",
      points: history.map(
        (snapshot) => snapshot.totalBillsCents + snapshot.totalInvoicesCents,
      ),
      tone: "neutral" as const,
    },
    {
      label: "Pago",
      value: summary.totalPaidCents,
      note: allSettled ? "Mês liquidado" : "Já resolvido",
      points: history.map((snapshot) => snapshot.totalPaidCents),
      tone: "neutral" as const,
    },
    {
      label: "Sobra estimada",
      value: summary.estimatedRemainingCents,
      note: "Depois de pagar tudo",
      points: history.map((snapshot) => snapshot.estimatedRemainingCents),
      // A sobra é a métrica que a pessoa acompanha; ela ganha o acento.
      tone: "accent" as const,
    },
  ];
  const attentionItems = [
    ...realBills.map((bill) => ({
      id: bill.id,
      type: "bill" as const,
      title: bill.name,
      amountCents: bill.amountCents,
      dueDate: bill.dueDate,
      status: bill.status,
    })),
    ...realInvoices.map((invoice) => ({
      id: invoice.id,
      type: "invoice" as const,
      title: invoice.name,
      amountCents: invoice.amountCents,
      dueDate: invoice.dueDate,
      status: invoice.status,
    })),
  ];

  return (
    <div className="space-y-6">
      {!currentMonth && viewingCurrent ? <CreateCurrentMonthCard /> : null}

      {currentMonth ? <AttentionStrip items={attentionItems} /> : null}

      <section className="grid gap-4 xl:grid-cols-[1.25fr_0.75fr]">
        <DashboardCard accent className="min-h-64">
          <div className="flex flex-col gap-8 lg:flex-row lg:items-end lg:justify-between">
            <div>
              <div className="mb-4 flex items-center gap-3">
                <StatusBadge status={summary.monthHealth} />
                <span className="text-sm text-text-muted">
                  {currentMonth
                    ? formatMonthLabel(new Date(currentMonth.year, currentMonth.month - 1, 1))
                    : formatMonthLabel()}
                </span>
              </div>
              <BalanceDisplay cents={totalOutstandingCents} label="Falta pagar neste mês" />

              {allSettled ? (
                <p className="mt-4 inline-flex items-center gap-2 text-sm font-medium text-status-positive">
                  <CheckCircle2 size={16} aria-hidden="true" />
                  Tudo pago neste mês. Nada vencendo por aqui.
                </p>
              ) : (
                <div className="mt-4 flex flex-wrap items-center gap-x-5 gap-y-1.5 text-sm">
                  {summary.totalOverdueCents > 0 ? (
                    <span className="num font-semibold text-accent">
                      {formatCurrency(summary.totalOverdueCents)} vencido
                    </span>
                  ) : null}
                  <span className="num text-text-secondary">
                    {formatCurrency(summary.totalPendingCents)} a vencer
                  </span>
                  <span className="num text-text-muted">
                    {formatCurrency(summary.totalPaidCents)} pago de{" "}
                    {formatCurrency(totalCommittedCents)}
                  </span>
                </div>
              )}

              <p className="mt-4 max-w-xl text-sm leading-6 text-text-muted">
                De {formatCurrency(summary.totalIncomeCents)} previstos este mês, sobra estimada de{" "}
                <span className="font-medium text-text-secondary">
                  {formatCurrency(summary.estimatedRemainingCents)}
                </span>{" "}
                depois de tudo pago.
              </p>
            </div>
            <div className="flex flex-col gap-3 lg:w-72">
              <QuickActionButton href="/app/bills" label="Adicionar conta" />
              <QuickActionButton
                href="/app/cards"
                label="Adicionar fatura do cartão"
                variant="secondary"
              />
              <QuickActionButton href="/app/calendar" label="Ver calendário" variant="secondary" />
            </div>
          </div>
        </DashboardCard>

        <AlertsPanel alerts={alerts} />
      </section>

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {monthMetrics.map((metric) => (
          <DashboardCard className="p-4" key={metric.label}>
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
              {metric.label}
            </p>
            <p className="num mt-2 text-2xl font-semibold text-text-primary">
              {formatCurrency(metric.value)}
            </p>
            <p className="mt-1 text-xs text-text-muted">{metric.note}</p>
            <MetricSparkline points={metric.points} tone={metric.tone} />
          </DashboardCard>
        ))}
      </section>

      <section className="grid gap-4 xl:grid-cols-[1fr_0.8fr]">
        <UpcomingBillsList bills={realBills} invoices={realInvoices} />
        <InvoiceSummaryCard invoices={realInvoices} />
      </section>

      {currentMonth ? (
        <DashboardCard
          description="De onde veio, para onde foi, e o que sobra."
          title="O mês em cascata"
        >
          <MonthWaterfallChart
            billsCents={summary.totalBillsCents}
            incomeCents={summary.totalIncomeCents}
            invoicesCents={summary.totalInvoicesCents}
          />
        </DashboardCard>
      ) : null}

      {categoryData.length > 0 ? (
        <DashboardCard description="Para onde as contas do mês estão indo." title="Por categoria">
          <CategoryBreakdownChart data={categoryData} />
        </DashboardCard>
      ) : null}

      {currentMonth && viewingCurrent && !nextMonth ? (
        <MonthGenerationReviewCard categories={categories} recurringBills={recurringBills} />
      ) : null}

      {currentMonth ? (
        <details className="group rounded-xl border border-border-subtle bg-background-card/95 p-5 shadow-xl shadow-black/15">
          <summary className="focus-ring flex cursor-pointer items-center justify-between gap-2 rounded-md text-sm font-semibold uppercase tracking-[0.16em] text-text-muted [&::-webkit-details-marker]:hidden">
            <span className="flex items-center gap-2">
              <TriangleMark className="shrink-0 text-accent/70" size={10} variant="solid" />
              Receitas do mês
            </span>
            <span className="flex items-center gap-2 text-xs font-medium normal-case tracking-normal text-text-muted">
              <span className="num group-open:hidden">
                {formatCurrency(summary.totalIncomeCents)} previstos
              </span>
              <ChevronDown
                aria-hidden="true"
                className="transition-transform duration-200 group-open:rotate-180"
                size={14}
              />
            </span>
          </summary>
          <div className="mt-4">
            <IncomeFormCard incomes={realIncomes} />
          </div>
        </details>
      ) : null}
    </div>
  );
}
