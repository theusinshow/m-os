import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { EmptyState } from "@/components/empty-state";
import { GoalCard } from "@/components/goals/goal-card";
import { GoalFormCard } from "@/components/goals/goal-form-card";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { formatCurrency } from "@/lib/formatters/currency";
import { getGoals } from "@/lib/goals";
import { getAppUserBySupabaseId } from "@/lib/months";

export default async function GoalsPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const goals = appUser ? await getGoals(appUser.id) : [];

  const trackedGoals = goals.filter((goal) => goal.status !== "archived");
  const totalSavedCents = trackedGoals.reduce((total, goal) => total + goal.currentAmountCents, 0);
  const totalTargetCents = trackedGoals.reduce((total, goal) => total + goal.targetAmountCents, 0);
  const completedCount = goals.filter((goal) => goal.status === "completed").length;

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Metas" title="Estou avançando?" />

      {trackedGoals.length > 0 ? (
        <DashboardCard title="Resumo das metas">
          <div className="grid gap-4 sm:grid-cols-3">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
                Guardado
              </p>
              <p className="num mt-1.5 text-2xl font-semibold text-text-primary">
                {formatCurrency(totalSavedCents)}
              </p>
              <p className="num mt-1 text-xs text-text-muted">
                de {formatCurrency(totalTargetCents)}
              </p>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
                Em acompanhamento
              </p>
              <p className="num mt-1.5 text-2xl font-semibold text-text-primary">
                {trackedGoals.length}
              </p>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
                Concluídas
              </p>
              <p className="num mt-1.5 text-2xl font-semibold text-status-positive">
                {completedCount}
              </p>
            </div>
          </div>
          <p className="mt-4 text-xs leading-5 text-text-muted">
            Metas são acompanhamento separado: não entram no cálculo de contas nem na sobra do mês.
          </p>
        </DashboardCard>
      ) : null}

      <GoalFormCard />

      {goals.length === 0 ? (
        <EmptyState
          title="Nenhuma meta ainda"
          description="Crie uma meta acima para acompanhar objetivos como reserva de emergência, viagem ou quitar uma dívida — sem afetar o orçamento do mês."
        />
      ) : (
        <div className="space-y-4">
          {goals.map((goal) => (
            <GoalCard goal={goal} key={goal.id} />
          ))}
        </div>
      )}
    </div>
  );
}
