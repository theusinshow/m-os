import { generateNextMonthFromReview } from "@/app/actions/month-generation";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { dayFromIsoDate } from "@/lib/due-date";
import { centsToInput } from "@/lib/money";

type Category = {
  id: string;
  name: string;
};

type RecurringBill = {
  id: string;
  categoryId: string | null;
  name: string;
  amountCents: number;
  dueDate: string;
};

export function MonthGenerationReviewCard({
  categories,
  recurringBills,
}: {
  categories: Category[];
  recurringBills: RecurringBill[];
}) {
  if (recurringBills.length === 0) {
    return null;
  }

  return (
    <DashboardCard title="Próximo mês">
      <ToastForm
        action={generateNextMonthFromReview}
        successMessage="Próximo mês gerado."
        className="space-y-4"
      >
        <div>
          <p className="text-lg font-semibold text-text-primary">Revisar recorrências</p>
          <p className="mt-1 text-sm leading-6 text-text-muted">
            Confirme as contas recorrentes antes de gerar o próximo mês. O app não cria o mês
            novo silenciosamente.
          </p>
        </div>

        <div className="space-y-3">
          {recurringBills.map((bill) => (
            <div className="rounded-lg border border-border-subtle bg-background-elevated p-4" key={bill.id}>
              <input name="sourceBillId" type="hidden" value={bill.id} />
              <label className="mb-3 flex items-center gap-2 text-sm font-semibold text-text-secondary">
                <input className="h-4 w-4 accent-accent" defaultChecked name={`include-${bill.id}`} type="checkbox" />
                Incluir no próximo mês
              </label>
              <div className="grid gap-3 md:grid-cols-[1fr_140px_160px_180px]">
                <input
                  className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                  defaultValue={bill.name}
                  name={`name-${bill.id}`}
                  required
                />
                <input
                  className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                  defaultValue={centsToInput(bill.amountCents)}
                  inputMode="decimal"
                  name={`amount-${bill.id}`}
                  required
                />
                <input
                  aria-label="Dia do vencimento"
                  className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                  defaultValue={dayFromIsoDate(bill.dueDate)}
                  inputMode="numeric"
                  max={31}
                  min={1}
                  name={`dueDay-${bill.id}`}
                  placeholder="Dia"
                  type="number"
                />
                <select
                  className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                  defaultValue={bill.categoryId ?? ""}
                  name={`categoryId-${bill.id}`}
                >
                  <option value="">Sem categoria</option>
                  {categories.map((category) => (
                    <option key={category.id} value={category.id}>
                      {category.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          ))}
        </div>

        <FormSubmitButton pendingLabel="Gerando...">Gerar próximo mês</FormSubmitButton>
      </ToastForm>
    </DashboardCard>
  );
}
