import { createCurrentMonth } from "@/app/actions/months";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { formatMonthLabel } from "@/lib/formatters/date";

export function CreateCurrentMonthCard() {
  return (
    <DashboardCard className="border-accent-border bg-accent-soft">
      <div className="flex flex-col gap-5 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <p className="text-sm font-semibold uppercase tracking-[0.16em] text-accent">
            Mês não criado
          </p>
          <h2 className="mt-3 text-2xl font-semibold text-text-primary">
            Criar {formatMonthLabel()}
          </h2>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-text-secondary">
            Crie o mês atual para começar a cadastrar receitas, contas e faturas reais.
            O app não gera meses automaticamente sem sua confirmação.
          </p>
        </div>
        <ToastForm action={createCurrentMonth} successMessage="Mês atual criado.">
          <FormSubmitButton pendingLabel="Criando...">Criar mês atual</FormSubmitButton>
        </ToastForm>
      </div>
    </DashboardCard>
  );
}
