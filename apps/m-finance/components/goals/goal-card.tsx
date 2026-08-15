import { CalendarClock, CheckCircle2 } from "lucide-react";
import { addContribution, deleteGoal, setGoalStatus, updateGoal } from "@/app/actions/goals";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { PriorityBadge } from "@/components/goals/priority-badge";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";
import type { GoalWithProgress } from "@/lib/goals";
import { centsToInput } from "@/lib/money";
import { cn } from "@/lib/utils";

const editClass =
  "focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary";

const statusLabel: Record<GoalWithProgress["status"], string | null> = {
  active: null,
  paused: "Pausada",
  completed: "Concluída",
  archived: "Arquivada",
};

export function GoalCard({ goal }: { goal: GoalWithProgress }) {
  const isCompleted = goal.status === "completed";
  const isArchived = goal.status === "archived";
  const canContribute = goal.status === "active" || goal.status === "paused";
  const label = statusLabel[goal.status];

  return (
    <DashboardCard className={cn(isArchived && "opacity-70")}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-lg font-semibold text-text-primary">{goal.name}</h3>
            <PriorityBadge priority={goal.priority} />
            {label ? (
              <span className="inline-flex h-7 items-center gap-1.5 rounded-md border border-border-default bg-background-elevated px-2.5 text-xs font-semibold text-text-muted">
                {isCompleted ? <CheckCircle2 className="text-status-positive" size={13} /> : null}
                {label}
              </span>
            ) : null}
          </div>
          {goal.deadline ? (
            <p className="mt-1 flex items-center gap-1.5 text-sm text-text-muted">
              <CalendarClock size={14} aria-hidden="true" />
              prazo {formatShortDate(goal.deadline)}
            </p>
          ) : null}
        </div>
        <p
          className={cn(
            "num text-sm font-semibold",
            isCompleted ? "text-status-positive" : "text-text-secondary",
          )}
        >
          {goal.progressPercent}%
        </p>
      </div>

      <div className="mt-4 h-2.5 overflow-hidden rounded-full border border-border-subtle bg-background-elevated">
        <div
          className={cn("h-full rounded-full", isCompleted ? "bg-status-positive" : "bg-accent")}
          style={{ width: `${goal.progressPercent}%` }}
        />
      </div>

      <div className="mt-3 flex flex-wrap items-baseline justify-between gap-2 text-sm">
        <p className="num text-text-primary">
          {formatCurrency(goal.currentAmountCents)}
          <span className="text-text-muted"> / {formatCurrency(goal.targetAmountCents)}</span>
        </p>
        {goal.remainingCents > 0 ? (
          <p className="num text-text-muted">faltam {formatCurrency(goal.remainingCents)}</p>
        ) : (
          <p className="num text-status-positive">objetivo alcançado</p>
        )}
      </div>

      {canContribute ? (
        <ValidatedForm
          action={addContribution}
          successMessage="Contribuição registrada."
          resetOnSuccess
          className="mt-4 flex flex-wrap items-end gap-3"
        >
          <input name="goalId" type="hidden" value={goal.id} />
          <div className="flex-1">
            <label
              className="mb-2 block text-sm font-medium text-text-secondary"
              htmlFor={`contrib-${goal.id}`}
            >
              Guardar um valor
            </label>
            <ValidatedInput
              className="focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted"
              id={`contrib-${goal.id}`}
              inputMode="decimal"
              name="amount"
              placeholder="500,00"
              required
            />
          </div>
          <FormSubmitButton pendingLabel="Guardando...">Contribuir</FormSubmitButton>
        </ValidatedForm>
      ) : null}

      <EditDisclosure className="mt-4">
        <ValidatedForm action={updateGoal} successMessage="Meta atualizada." className="grid gap-3">
          <input name="goalId" type="hidden" value={goal.id} />
          <ValidatedInput className={editClass} defaultValue={goal.name} name="name" required />
          <div className="grid gap-3 sm:grid-cols-2">
            <ValidatedInput
              aria-label="Valor alvo"
              className={editClass}
              defaultValue={centsToInput(goal.targetAmountCents)}
              inputMode="decimal"
              name="targetAmount"
              required
            />
            <ValidatedInput
              aria-label="Valor atual"
              className={editClass}
              defaultValue={centsToInput(goal.currentAmountCents)}
              inputMode="decimal"
              name="currentAmount"
            />
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <ValidatedSelect className={editClass} defaultValue={goal.priority} name="priority">
              <option value="low">Baixa</option>
              <option value="medium">Média</option>
              <option value="high">Alta</option>
            </ValidatedSelect>
            <ValidatedInput
              aria-label="Prazo"
              className={editClass}
              defaultValue={goal.deadline ?? ""}
              name="deadline"
              type="date"
            />
          </div>
          <FormSubmitButton pendingLabel="Salvando...">Salvar meta</FormSubmitButton>
        </ValidatedForm>

        <div className="mt-3 flex flex-wrap gap-2">
          {goal.status === "active" ? (
            <ToastForm action={setGoalStatus} successMessage="Meta pausada.">
              <input name="goalId" type="hidden" value={goal.id} />
              <input name="status" type="hidden" value="paused" />
              <FormSubmitButton pendingLabel="Pausando..." variant="secondary">
                Pausar
              </FormSubmitButton>
            </ToastForm>
          ) : null}
          {goal.status === "paused" ? (
            <ToastForm action={setGoalStatus} successMessage="Meta retomada.">
              <input name="goalId" type="hidden" value={goal.id} />
              <input name="status" type="hidden" value="active" />
              <FormSubmitButton pendingLabel="Retomando..." variant="secondary">
                Retomar
              </FormSubmitButton>
            </ToastForm>
          ) : null}
          {isArchived ? (
            <ToastForm action={setGoalStatus} successMessage="Meta reativada.">
              <input name="goalId" type="hidden" value={goal.id} />
              <input name="status" type="hidden" value="active" />
              <FormSubmitButton pendingLabel="Reativando..." variant="secondary">
                Reativar
              </FormSubmitButton>
            </ToastForm>
          ) : (
            <ToastForm action={setGoalStatus} successMessage="Meta arquivada.">
              <input name="goalId" type="hidden" value={goal.id} />
              <input name="status" type="hidden" value="archived" />
              <FormSubmitButton pendingLabel="Arquivando..." variant="secondary">
                Arquivar
              </FormSubmitButton>
            </ToastForm>
          )}
        </div>

        <ToastForm action={deleteGoal} successMessage="Meta excluída." className="mt-2">
          <input name="goalId" type="hidden" value={goal.id} />
          <ConfirmDeleteButton confirmMessage="Excluir esta meta? O histórico de contribuições será perdido.">
            Excluir meta
          </ConfirmDeleteButton>
        </ToastForm>
      </EditDisclosure>
    </DashboardCard>
  );
}
