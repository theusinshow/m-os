import { deleteBill, markBillAsPaid, markBillAsPending, updateBill } from "@/app/actions/bills";
import { QuickAddExpense } from "@/components/bills/quick-add-expense";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { StatusBadge } from "@/components/status-badge";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";
import { dayFromIsoDate } from "@/lib/due-date";
import { centsToInput } from "@/lib/money";

type Category = {
  id: string;
  name: string;
};

type Bill = {
  id: string;
  categoryId: string | null;
  name: string;
  amountCents: number;
  dueDate: string;
  isRecurring: boolean;
  status: "pending" | "paid" | "overdue";
  categoryName: string | null;
};

const editInputClass =
  "focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary";

export function BillFormCard({
  bills,
  categories,
}: {
  bills: Bill[];
  categories: Category[];
}) {
  const pending = bills.filter((bill) => bill.status !== "paid");
  const paid = bills.filter((bill) => bill.status === "paid");
  const totalPendingCents = pending.reduce((total, bill) => total + bill.amountCents, 0);
  const totalPaidCents = paid.reduce((total, bill) => total + bill.amountCents, 0);

  return (
    <div className="space-y-4">
      <DashboardCard accent>
        <QuickAddExpense
          categories={categories}
          paidCount={paid.length}
          pendingCount={pending.length}
          totalPendingCents={totalPendingCents}
        />
      </DashboardCard>

      <DashboardCard description="Despesas pendentes e vencidas deste mês." title="A pagar">
        {pending.length === 0 ? (
          <InlineEmpty>Tudo pago por aqui. Nenhuma despesa em aberto neste mês.</InlineEmpty>
        ) : (
          <div className="space-y-3">
            {pending.map((bill) => (
              <BillRow bill={bill} categories={categories} key={bill.id} />
            ))}
          </div>
        )}
      </DashboardCard>

      <DashboardCard
        description={paid.length > 0 ? `Total pago: ${formatCurrency(totalPaidCents)}` : undefined}
        title="Pagas"
      >
        {paid.length === 0 ? (
          <InlineEmpty>Nenhuma despesa marcada como paga ainda.</InlineEmpty>
        ) : (
          <div className="space-y-3">
            {paid.map((bill) => (
              <BillRow bill={bill} categories={categories} key={bill.id} paid />
            ))}
          </div>
        )}
      </DashboardCard>
    </div>
  );
}

function BillRow({
  bill,
  categories,
  paid = false,
}: {
  bill: Bill;
  categories: Category[];
  paid?: boolean;
}) {
  return (
    <div className={cnRow(paid)}>
      <div className="grid gap-3 sm:grid-cols-[1fr_auto] sm:items-center">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-semibold text-text-primary">{bill.name}</p>
            <StatusBadge status={bill.status} />
          </div>
          <p className="mt-1 text-sm text-text-muted">
            {bill.categoryName ?? "Sem categoria"} · vence {formatShortDate(bill.dueDate)}
            {bill.isRecurring ? " · recorrente" : ""}
          </p>
        </div>
        <div className="flex items-center justify-between gap-3 sm:justify-end">
          <p className="num font-semibold text-text-primary">{formatCurrency(bill.amountCents)}</p>
          {paid ? (
            <ToastForm action={markBillAsPending} successMessage="Conta reaberta.">
              <input name="billId" type="hidden" value={bill.id} />
              <FormSubmitButton pendingLabel="Reabrindo..." variant="secondary">
                Reabrir
              </FormSubmitButton>
            </ToastForm>
          ) : (
            <ToastForm action={markBillAsPaid} successMessage="Conta marcada como paga.">
              <input name="billId" type="hidden" value={bill.id} />
              <FormSubmitButton pendingLabel="Marcando..." variant="success">
                Pago
              </FormSubmitButton>
            </ToastForm>
          )}
        </div>
      </div>

      <EditDisclosure className="mt-3">
        <ValidatedForm action={updateBill} successMessage="Despesa atualizada." className="grid gap-3">
          <input name="billId" type="hidden" value={bill.id} />
          <ValidatedInput className={editInputClass} defaultValue={bill.name} name="name" required />
          <div className="grid gap-3 sm:grid-cols-2">
            <ValidatedInput
              className={editInputClass}
              defaultValue={centsToInput(bill.amountCents)}
              inputMode="decimal"
              name="amount"
              required
            />
            <ValidatedInput
              aria-label="Dia do vencimento"
              className={editInputClass}
              defaultValue={dayFromIsoDate(bill.dueDate)}
              inputMode="numeric"
              max={31}
              min={1}
              name="dueDay"
              placeholder="Dia"
              type="number"
            />
          </div>
          <ValidatedSelect className={editInputClass} defaultValue={bill.categoryId ?? ""} name="categoryId">
            <option value="">Sem categoria</option>
            {categories.map((category) => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
          </ValidatedSelect>
          <label className="flex items-center gap-2 text-sm text-text-secondary">
            <input className="h-4 w-4 accent-accent" defaultChecked={bill.isRecurring} name="isRecurring" type="checkbox" />
            Despesa recorrente
          </label>
          <FormSubmitButton pendingLabel="Salvando...">Salvar despesa</FormSubmitButton>
        </ValidatedForm>
        <ToastForm action={deleteBill} successMessage="Despesa excluída." className="mt-2">
          <input name="billId" type="hidden" value={bill.id} />
          <ConfirmDeleteButton confirmMessage="Excluir esta despesa?">Excluir despesa</ConfirmDeleteButton>
        </ToastForm>
      </EditDisclosure>
    </div>
  );
}

function cnRow(paid: boolean) {
  return [
    "rounded-lg border border-border-subtle bg-background-elevated p-4 transition duration-200",
    paid ? "opacity-75" : "",
  ]
    .filter(Boolean)
    .join(" ");
}
