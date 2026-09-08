import { CheckCircle2, ChevronDown } from "lucide-react";
import { AttentionStrip } from "@/components/dashboard/attention-strip";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { AlertsPanel } from "@/components/dashboard/alerts-panel";
import { IncomeFormCard } from "@/components/dashboard/income-form-card";
import { InvoiceSummaryCard } from "@/components/dashboard/invoice-summary-card";
import { MonthGenerationReviewCard } from "@/components/dashboard/month-generation-review-card";
import { CommitmentsCard } from "@/components/dashboard/commitments-card";
import { ProjectionCard } from "@/components/dashboard/projection-card";
import { PersonalBusinessCard } from "@/components/dashboard/personal-business-card";
import { QuickActionButton } from "@/components/quick-action-button";
import { StatusBadge } from "@/components/status-badge";
import { Badge } from "@/components/ui/badge";
import { UpcomingBillsList } from "@/components/dashboard/upcoming-bills-list";
import { BalanceDisplay } from "@/components/dashboard/balance-display";
import { CategoryBreakdownChart } from "@/components/charts/category-breakdown-chart";
import { DueDateHeatmap } from "@/components/charts/due-date-heatmap";
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
import {
  getBillCategories,
  getBillsByMonth,
  getInstallmentBillsForUser,
  getRecurringBillsByMonth,
} from "@/lib/bills";
import { getInvoicesByMonth } from "@/lib/cards";
import { getMonthTotalsForUser, getNextMonthForUser } from "@/lib/months";
import { getMonthPartsAtOffset } from "@/lib/months";
import { monthValue } from "@/lib/active-month";
import { summarizeInstallments } from "@/lib/calculations/commitments";
import { buildMonthProjection } from "@/lib/calculations/projection";
import { toMonthCategoryData } from "@/lib/calculations/charts/month-categories";
import { pendingRecurrences } from "@/lib/recurrence";
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
  const nextMonthBills = nextMonth ? await getBillsByMonth(nextMonth.id) : [];
  // O card de revisão aparece pelo que falta no mês seguinte, não pela
  // existência da linha do mês: uma série parcelada cria meses até 2028 e
  // fazia o card sumir para sempre.
  const recurrencesToReview = pendingRecurrences(recurringBills, nextMonthBills);
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
  const categoryData = toMonthCategoryData(realBills, realInvoices);
  const installmentSeries = summarizeInstallments(
    appUser ? await getInstallmentBillsForUser(appUser.id) : [],
  );
  // O seletor de mês da receita cobre o mês da tela e o ano seguinte: a nota
  // emitida hoje pode cair em qualquer um deles.
  const incomeMonthOptions = currentMonth
    ? Array.from({ length: 13 }, (_, offset) => {
        const parts = getMonthPartsAtOffset(currentMonth.month, currentMonth.year, offset);
        return {
          value: monthValue(parts.month, parts.year),
          label: formatMonthLabel(new Date(parts.year, parts.month - 1, 1)),
        };
      })
    : [];
  const projection = currentMonth
    ? buildMonthProjection(
        appUser ? await getMonthTotalsForUser(appUser.id) : [],
        { month: currentMonth.month, year: currentMonth.year },
        6,
      )
    : [];
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
      note: summary.totalIncomeCents === 0 ? "Falta lançar a receita" : "Depois de pagar tudo",
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
                {/* Sem receita, a saúde do mês é sempre "Negativo" — o veredito
                    é do dado que falta, não do mês. */}
                {summary.totalIncomeCents === 0 ? (
                  <Badge
                    className="border-border-default bg-background-elevated text-text-secondary"
                    icon={<TriangleMark className="text-current" size={9} variant="solid" />}
                    label="Sem receita"
                  />
                ) : (
                  <StatusBadge status={summary.monthHealth} />
                )}
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

              {summary.totalIncomeCents === 0 ? (
                // Sem receita, "sobra estimada" é só o comprometido com sinal
                // trocado — um número que parece resposta e não é. Melhor pedir
                // o dado que falta do que exibir a conta pela metade.
                <p className="mt-4 max-w-xl text-sm leading-6 text-text-muted">
                  Nenhuma receita lançada neste mês, então não dá para dizer quanto sobra. Você tem{" "}
                  <span className="font-medium text-text-secondary">
                    {formatCurrency(totalCommittedCents)}
                  </span>{" "}
                  comprometidos —{" "}
                  <a className="focus-ring rounded font-medium text-accent underline underline-offset-4" href="#receitas">
                    lance sua receita
                  </a>{" "}
                  para fechar a conta.
                </p>
              ) : (
                <p className="mt-4 max-w-xl text-sm leading-6 text-text-muted">
                  De {formatCurrency(summary.totalIncomeCents)} previstos este mês, sobra estimada de{" "}
                  <span className="font-medium text-text-secondary">
                    {formatCurrency(summary.estimatedRemainingCents)}
                  </span>{" "}
                  depois de tudo pago.
                </p>
              )}
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

      {currentMonth ? (
        <details
          className="group scroll-mt-24 rounded-xl border border-border-subtle bg-background-card/95 p-5 shadow-xl shadow-black/15"
          id="receitas"
          open={realIncomes.length === 0}
        >
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
            <IncomeFormCard
              activeMonthValue={
                currentMonth ? monthValue(currentMonth.month, currentMonth.year) : ""
              }
              incomes={realIncomes}
              monthOptions={incomeMonthOptions}
            />
          </div>
        </details>
      ) : null}
      <section className="grid gap-4 xl:grid-cols-[1fr_0.8fr]">
        <UpcomingBillsList bills={realBills} invoices={realInvoices} />
        <div className="space-y-4">
          <InvoiceSummaryCard invoices={realInvoices} />
          <PersonalBusinessCard invoices={realInvoices} />
        </div>
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

      {currentMonth ? (
        <DashboardCard description="Onde os vencimentos se concentram." title="Pressão do mês">
          <DueDateHeatmap
            items={[...realBills, ...realInvoices].map((item) => ({
              dueDate: item.dueDate,
              amountCents: item.amountCents,
            }))}
            month={currentMonth.month}
            year={currentMonth.year}
          />
        </DashboardCard>
      ) : null}

      {categoryData.length > 0 ? (
        <DashboardCard
          description="Contas e faturas — para onde o dinheiro do mês está indo."
          title="Por categoria"
        >
          <CategoryBreakdownChart data={categoryData} />
        </DashboardCard>
      ) : null}

      {projection.length > 1 ? <ProjectionCard rows={projection} /> : null}

      {installmentSeries.length > 0 ? <CommitmentsCard series={installmentSeries} /> : null}

      {currentMonth && viewingCurrent && recurrencesToReview.length > 0 ? (
        <MonthGenerationReviewCard categories={categories} recurringBills={recurrencesToReview} />
      ) : null}

    </div>
  );
}
