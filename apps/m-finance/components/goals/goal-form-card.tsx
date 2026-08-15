import { createGoal } from "@/app/actions/goals";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";

const fieldClass =
  "focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted";

export function GoalFormCard() {
  return (
    <DashboardCard
      description="Objetivos financeiros acompanhados à parte — não entram na sobra do mês."
      title="Nova meta"
    >
      <ValidatedForm
        action={createGoal}
        successMessage="Meta criada."
        resetOnSuccess
        className="grid gap-4 lg:grid-cols-2"
      >
        <div className="lg:col-span-2">
          <label className="mb-2 block text-sm font-medium text-text-secondary" htmlFor="goal-name">
            Nome da meta
          </label>
          <ValidatedInput
            className={fieldClass}
            id="goal-name"
            name="name"
            placeholder="Reserva de emergência"
            required
          />
        </div>

        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor="goal-target"
          >
            Valor alvo
          </label>
          <ValidatedInput
            className={fieldClass}
            id="goal-target"
            inputMode="decimal"
            name="targetAmount"
            placeholder="10000,00"
            required
          />
        </div>

        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor="goal-current"
          >
            Já guardado (opcional)
          </label>
          <ValidatedInput
            className={fieldClass}
            id="goal-current"
            inputMode="decimal"
            name="currentAmount"
            placeholder="0,00"
          />
        </div>

        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor="goal-priority"
          >
            Prioridade
          </label>
          <ValidatedSelect className={fieldClass} defaultValue="medium" id="goal-priority" name="priority">
            <option value="low">Baixa</option>
            <option value="medium">Média</option>
            <option value="high">Alta</option>
          </ValidatedSelect>
        </div>

        <div>
          <label
            className="mb-2 block text-sm font-medium text-text-secondary"
            htmlFor="goal-deadline"
          >
            Prazo (opcional)
          </label>
          <ValidatedInput className={fieldClass} id="goal-deadline" name="deadline" type="date" />
        </div>

        <div className="lg:col-span-2">
          <FormSubmitButton pendingLabel="Criando...">Criar meta</FormSubmitButton>
        </div>
      </ValidatedForm>
    </DashboardCard>
  );
}
