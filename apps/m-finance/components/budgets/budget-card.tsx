import { deleteBudget, updateBudget } from "@/app/actions/budgets";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput } from "@/components/ui/validated-form";
import { formatCurrency } from "@/lib/formatters/currency";
import { centsToInput } from "@/lib/money";
import { cn } from "@/lib/utils";
import type { Budget } from "@/lib/budgets";

const editClass = "field-input";

export function BudgetCard({ budget }: { budget: Budget }) {
  const barColor = budget.isOverBudget
    ? "bg-status-negative"
    : budget.isWarning
      ? "bg-status-fair"
      : "bg-accent";

  return (
    <DashboardCard>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="text-lg font-semibold text-text-primary">{budget.label}</h3>
          <p className="mt-1 text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">
            {budget.budgetType === "total"
              ? "Total"
              : budget.budgetType === "category"
                ? "Categoria"
                : "Cartão"}
          </p>
        </div>
        <p
          className={cn(
            "num text-sm font-semibold",
            budget.isOverBudget ? "text-status-negative" : "text-text-secondary",
          )}
        >
          {budget.percentage}%
        </p>
      </div>

      <div className="mt-4 h-2.5 overflow-hidden rounded-full border border-border-subtle bg-background-elevated">
        <div
          className={cn("h-full rounded-full transition-all", barColor)}
          style={{ width: `${Math.min(budget.percentage, 100)}%` }}
        />
      </div>

      <div className="mt-3 flex flex-wrap items-baseline justify-between gap-2 text-sm">
        <p className="num text-text-primary">
          {formatCurrency(budget.spentCents)}
          <span className="text-text-muted"> / {formatCurrency(budget.limitCents)}</span>
        </p>
        {budget.isOverBudget ? (
          <p className="num text-status-negative">
            estourou {formatCurrency(Math.abs(budget.remainingCents))}
          </p>
        ) : budget.remainingCents > 0 ? (
          <p className="num text-text-muted">resta {formatCurrency(budget.remainingCents)}</p>
        ) : (
          <p className="num text-status-positive">no limite</p>
        )}
      </div>

      <EditDisclosure className="mt-4">
        <ValidatedForm action={updateBudget} successMessage="Orçamento atualizado." className="grid gap-3">
          <input name="budgetId" type="hidden" value={budget.id} />
          <input name="budgetType" type="hidden" value={budget.budgetType} />
          {budget.categoryId ? (
            <input name="categoryId" type="hidden" value={budget.categoryId} />
          ) : null}
          {budget.cardId ? (
            <input name="cardId" type="hidden" value={budget.cardId} />
          ) : null}
          <div>
            <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor={`limit-${budget.id}`}>
              Novo limite
            </label>
            <ValidatedInput
              aria-label="Novo limite"
              className={editClass}
              defaultValue={centsToInput(budget.limitCents)}
              inputMode="decimal"
              name="limit"
              required
            />
          </div>
          <FormSubmitButton pendingLabel="Salvando...">Salvar orçamento</FormSubmitButton>
        </ValidatedForm>

        <ToastForm action={deleteBudget} successMessage="Orçamento excluído." className="mt-2">
          <input name="budgetId" type="hidden" value={budget.id} />
          <ConfirmDeleteButton confirmMessage="Excluir este orçamento?">
            Excluir orçamento
          </ConfirmDeleteButton>
        </ToastForm>
      </EditDisclosure>
    </DashboardCard>
  );
}
