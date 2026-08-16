import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { EmptyState } from "@/components/empty-state";
import { BudgetCard } from "@/components/budgets/budget-card";
import { BudgetFormDrawer } from "@/components/budgets/budget-form-drawer";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { getBillCategories } from "@/lib/bills";
import { getCreditCards } from "@/lib/cards";
import { getBudgetsByMonth } from "@/lib/budgets";
import { formatCurrency } from "@/lib/formatters/currency";
import { getAppUserBySupabaseId } from "@/lib/months";
import { getActiveMonthForUser } from "@/lib/active-month";

export default async function BudgetsPage() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!appUser) {
    return (
      <div className="space-y-6">
        <PageHeading eyebrow="Orçamento" title="Quanto posso gastar?" />
        <EmptyState title="Usuário não configurado" description="Faça login no app para acessar o orçamento." />
      </div>
    );
  }

  const month = await getActiveMonthForUser(appUser.id);
  const [categories, cards] = await Promise.all([
    getBillCategories(appUser.id),
    getCreditCards(appUser.id),
  ]);

  if (!month) {
      return (
      <div className="space-y-6">
        <PageHeading eyebrow="Orçamento" title="Quanto posso gastar?" />
        <EmptyState
          title="Nenhum mês ativo"
          description="Crie o mês atual no dashboard antes de definir orçamentos."
        />
      </div>
    );
  }

  const budgetList = await getBudgetsByMonth(month.id, appUser.id);
  const overBudgetCount = budgetList.filter((b) => b.isOverBudget).length;
  const warningCount = budgetList.filter((b) => b.isWarning).length;
  const totalLimit = budgetList.reduce((acc, b) => acc + b.limitCents, 0);
  const totalSpent = budgetList.reduce((acc, b) => acc + b.spentCents, 0);

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Orçamento" title="Quanto posso gastar?">
        <BudgetFormDrawer categories={categories} cards={cards} />
      </PageHeading>

      {budgetList.length > 0 ? (
        <DashboardCard title="Resumo dos orçamentos">
          <div className="grid gap-4 sm:grid-cols-3">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
                Orçado
              </p>
              <p className="num mt-1.5 text-2xl font-semibold text-text-primary">
                {formatCurrency(totalLimit)}
              </p>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
                Gasto
              </p>
              <p className="num mt-1.5 text-2xl font-semibold text-text-primary">
                {formatCurrency(totalSpent)}
              </p>
            </div>
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
                Alertas
              </p>
              <p className="num mt-1.5 text-2xl font-semibold text-status-negative">
                {overBudgetCount + warningCount}
              </p>
              <p className="num mt-1 text-xs text-text-muted">
                {overBudgetCount > 0 ? `${overBudgetCount} estourado(s)` : ""}
                {overBudgetCount > 0 && warningCount > 0 ? ", " : ""}
                {warningCount > 0 ? `${warningCount} em alerta` : ""}
              </p>
            </div>
          </div>
        </DashboardCard>
      ) : null}

      {budgetList.length === 0 ? (
        <EmptyState
          title="Nenhum orçamento ainda"
          description="Defina um teto de gasto para acompanhar quanto ainda pode gastar no mês."
        />
      ) : (
        <div className="space-y-4">
          {budgetList.map((budget) => (
            <BudgetCard budget={budget} key={budget.id} />
          ))}
        </div>
      )}
    </div>
  );
}
