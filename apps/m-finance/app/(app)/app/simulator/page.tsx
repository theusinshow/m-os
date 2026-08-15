import { CreateCurrentMonthCard } from "@/components/dashboard/create-current-month-card";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { PageHeading } from "@/components/page-heading";
import { SimulationList } from "@/components/simulator/simulation-list";
import { SimulatorForm } from "@/components/simulator/simulator-form";
import { requireUser } from "@/lib/auth/guard";
import { formatCurrency } from "@/lib/formatters/currency";
import { getAppUserBySupabaseId, getCurrentMonthForUser } from "@/lib/months";
import { getSimulationBaseline, getSimulations } from "@/lib/simulations";

export default async function SimulatorPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const currentMonth = appUser ? await getCurrentMonthForUser(appUser.id) : null;

  if (!appUser || !currentMonth) {
    return (
      <div className="space-y-6">
        <PageHeading eyebrow="Simulador" title="Posso comprar isso agora?" />
        <CreateCurrentMonthCard />
      </div>
    );
  }

  const [baseline, simulations] = await Promise.all([
    getSimulationBaseline(appUser.id, currentMonth.id),
    getSimulations(appUser.id),
  ]);

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Simulador" title="Posso comprar isso agora?" />

      <DashboardCard title="Base da projeção">
        <p className="text-sm leading-6 text-text-muted">
          A projeção assume que as recorrências do mês atual se repetem. Sobra mensal estimada como
          base de cálculo:
        </p>
        <p className="num mt-3 text-3xl font-semibold text-text-primary">
          {formatCurrency(baseline.baselineRemainingCents)}
        </p>
        <p className="mt-2 text-xs text-text-muted">
          Receita {formatCurrency(baseline.totalIncomeCents)} − recorrentes{" "}
          {formatCurrency(baseline.recurringBillsCents)} − faturas{" "}
          {formatCurrency(baseline.invoicesCents)}
        </p>
      </DashboardCard>

      <SimulatorForm defaultMonth={currentMonth.month} defaultYear={currentMonth.year} />

      <div className="space-y-3">
        <h2 className="text-sm font-semibold uppercase tracking-[0.16em] text-text-muted">
          Simulações salvas
        </h2>
        <SimulationList simulations={simulations} />
      </div>
    </div>
  );
}
