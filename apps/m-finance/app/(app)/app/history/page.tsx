import { saveCurrentMonthSnapshot } from "@/app/actions/history";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { HistoryTrendChart } from "@/components/charts/history-trend-chart";
import { EmptyState } from "@/components/empty-state";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { PageHeading } from "@/components/page-heading";
import { StatusBadge } from "@/components/status-badge";
import { requireUser } from "@/lib/auth/guard";
import { formatCurrency } from "@/lib/formatters/currency";
import { getMonthlySnapshots } from "@/lib/history";
import { getAppUserBySupabaseId } from "@/lib/months";

const monthNames = [
  "Janeiro",
  "Fevereiro",
  "Março",
  "Abril",
  "Maio",
  "Junho",
  "Julho",
  "Agosto",
  "Setembro",
  "Outubro",
  "Novembro",
  "Dezembro",
];

export default async function HistoryPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const snapshots = appUser ? await getMonthlySnapshots(appUser.id) : [];
  // Snapshots come newest-first; the trend reads left-to-right oldest-first.
  const trend = [...snapshots].reverse().map((snapshot) => ({
    label: `${monthNames[snapshot.month - 1].slice(0, 3)}/${String(snapshot.year).slice(2)}`,
    receita: snapshot.totalIncomeCents,
    comprometido: snapshot.totalBillsCents + snapshot.totalInvoicesCents,
    sobra: snapshot.estimatedRemainingCents,
  }));

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Histórico" title="Resumo mensal">
        <ToastForm action={saveCurrentMonthSnapshot} successMessage="Snapshot do mês salvo.">
          <FormSubmitButton pendingLabel="Salvando...">Salvar snapshot do mês atual</FormSubmitButton>
        </ToastForm>
      </PageHeading>

      {snapshots.length === 0 ? (
        <EmptyState
          title="Nenhum histórico salvo"
          description="Salve um snapshot do mês atual para guardar receita, contas, faturas, pendências e sobra estimada."
        />
      ) : (
        <div className="grid gap-4">
          <DashboardCard
            description="Receita, comprometido e sobra ao longo dos meses salvos."
            title="Evolução"
          >
            <HistoryTrendChart data={trend} />
          </DashboardCard>
          {snapshots.map((snapshot) => (
            <DashboardCard key={snapshot.id}>
              <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                <div>
                  <div className="flex items-center gap-3">
                    <h2 className="text-xl font-semibold text-text-primary">
                      {monthNames[snapshot.month - 1]} {snapshot.year}
                    </h2>
                    <StatusBadge status={snapshot.monthHealth} />
                  </div>
                  <p className="mt-2 text-sm text-text-muted">
                    Snapshot salvo em {snapshot.updatedAt.toLocaleDateString("pt-BR")}
                  </p>
                </div>
                <p className="num text-3xl font-semibold text-text-primary">
                  {formatCurrency(snapshot.estimatedRemainingCents)}
                </p>
              </div>
              <div className="mt-5 grid gap-3 md:grid-cols-3 xl:grid-cols-6">
                <Metric label="Receita" value={snapshot.totalIncomeCents} />
                <Metric label="Contas" value={snapshot.totalBillsCents} />
                <Metric label="Faturas" value={snapshot.totalInvoicesCents} />
                <Metric label="Pago" value={snapshot.totalPaidCents} />
                <Metric label="Pendente" value={snapshot.totalPendingCents} />
                <Metric label="Vencido" value={snapshot.totalOverdueCents} />
              </div>
            </DashboardCard>
          ))}
        </div>
      )}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-md border border-border-subtle bg-background-elevated p-3">
      <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">{label}</p>
      <p className="num mt-2 text-sm font-semibold text-text-primary">
        {formatCurrency(value)}
      </p>
    </div>
  );
}
