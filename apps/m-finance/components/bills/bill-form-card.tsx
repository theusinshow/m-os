import {
  deleteBill,
  deleteBillSeries,
  markBillAsPending,
  updateBill,
} from "@/app/actions/bills";
import { QuickAddExpense } from "@/components/bills/quick-add-expense";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { FormSubmitButton } from "@/components/form-submit-button";
import { MarkPaidButton } from "@/components/payable/mark-paid-button";
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
  seriesId: string | null;
  seriesNumber: number | null;
  seriesTotal: number | null;
  status: "pending" | "paid" | "overdue";
  categoryName: string | null;
};

const editInputClass = "field-input";

function toDateOnly(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function diffInDays(date: string, today = new Date()) {
  const due = new Date(`${date}T12:00:00`);
  return Math.round((toDateOnly(due).getTime() - toDateOnly(today).getTime()) / 86_400_000);
}

function groupPendingBills(bills: Bill[]) {
  const groups = [
    { key: "overdue", title: "Vencidas", description: "Resolva antes de olhar o restante.", items: [] as Bill[] },
    { key: "today", title: "Hoje", description: "Ação principal do dia.", items: [] as Bill[] },
    { key: "week", title: "Próximos 7 dias", description: "Prepare o pagamento agora.", items: [] as Bill[] },
    { key: "later", title: "Depois", description: "Contas futuras do mês.", items: [] as Bill[] },
  ];

  for (const bill of bills) {
    const days = diffInDays(bill.dueDate);
    if (bill.status === "overdue" || days < 0) groups[0].items.push(bill);
    else if (days === 0) groups[1].items.push(bill);
    else if (days <= 7) groups[2].items.push(bill);
    else groups[3].items.push(bill);
  }

  for (const group of groups) {
    group.items.sort((a, b) => a.dueDate.localeCompare(b.dueDate));
  }

  return groups.filter((group) => group.items.length > 0);
}

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
  const pendingGroups = groupPendingBills(pending);

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

      <DashboardCard description="Contas abertas agrupadas por urgência." title="A pagar">
        {pending.length === 0 ? (
          <InlineEmpty>Tudo pago por aqui. Nenhuma conta em aberto neste mês.</InlineEmpty>
        ) : (
          <div className="space-y-5">
            {pendingGroups.map((group) => (
              <section className="space-y-2" key={group.key}>
                <div className="flex flex-wrap items-end justify-between gap-2">
                  <div>
                    <h3 className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                      {group.title}
                    </h3>
                    <p className="mt-1 text-xs text-text-muted">{group.description}</p>
                  </div>
                  <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold text-text-muted">
                    {group.items.length}
                  </span>
                </div>
                <div className="space-y-3">
                  {group.items.map((bill) => (
                    <BillRow bill={bill} categories={categories} key={bill.id} groupKey={group.key} />
                  ))}
                </div>
              </section>
            ))}
          </div>
        )}
      </DashboardCard>

      <DashboardCard
        description={paid.length > 0 ? `Total pago: ${formatCurrency(totalPaidCents)}` : undefined}
        title="Pagas"
      >
        {paid.length === 0 ? (
          <InlineEmpty>Nenhuma conta marcada como paga ainda.</InlineEmpty>
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
  groupKey = "later",
}: {
  bill: Bill;
  categories: Category[];
  paid?: boolean;
  groupKey?: string;
}) {
  return (
    <div className={cnRow(paid, groupKey)}>
      <div className="grid gap-3 sm:grid-cols-[1fr_auto] sm:items-center">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-semibold text-text-primary">{bill.name}</p>
            <StatusBadge status={bill.status} />
          </div>
          <p className="mt-1 text-sm text-text-muted">
            {bill.categoryName ?? "Sem categoria"} · vence {formatShortDate(bill.dueDate)}
            {bill.seriesNumber && bill.seriesTotal
              ? ` · mês ${bill.seriesNumber}/${bill.seriesTotal}`
              : bill.isRecurring
                ? " · recorrente"
                : ""}
          </p>
        </div>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-end">
          <p className="num text-lg font-semibold text-text-primary sm:text-base">
            {formatCurrency(bill.amountCents)}
          </p>
          {paid ? (
            <ToastForm action={markBillAsPending} successMessage="Conta reaberta.">
              <input name="billId" type="hidden" value={bill.id} />
              <FormSubmitButton pendingLabel="Reabrindo..." variant="secondary">
                Reabrir
              </FormSubmitButton>
            </ToastForm>
          ) : (
            <MarkPaidButton payableId={bill.id} payableType="bill" variant="success">
              Marcar pago
            </MarkPaidButton>
          )}
        </div>
      </div>

      <EditDisclosure className="mt-3">
        <ValidatedForm action={updateBill} successMessage="Conta atualizada." className="grid gap-3">
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
          {!bill.seriesId ? (
            <label className="flex items-center gap-2 text-sm text-text-secondary">
              <input
                className="h-4 w-4 accent-accent"
                defaultChecked={bill.isRecurring}
                name="isRecurring"
                type="checkbox"
              />
              Conta recorrente
            </label>
          ) : (
            <p className="text-xs leading-5 text-text-muted">
              Esta ocorrência pertence a uma série. A edição altera somente este mês.
            </p>
          )}
          <FormSubmitButton pendingLabel="Salvando...">Salvar conta</FormSubmitButton>
        </ValidatedForm>
        <div className="mt-2 flex flex-wrap gap-2">
          <ToastForm action={deleteBill} successMessage="Conta excluída.">
            <input name="billId" type="hidden" value={bill.id} />
            <ConfirmDeleteButton confirmMessage="Excluir apenas esta conta?">
              Excluir este mês
            </ConfirmDeleteButton>
          </ToastForm>
          {bill.seriesId ? (
            <ToastForm action={deleteBillSeries} successMessage="Série excluída.">
              <input name="seriesId" type="hidden" value={bill.seriesId} />
              <ConfirmDeleteButton confirmMessage="Excluir todas as contas desta série?">
                Excluir série
              </ConfirmDeleteButton>
            </ToastForm>
          ) : null}
        </div>
      </EditDisclosure>
    </div>
  );
}

function cnRow(paid: boolean, groupKey: string) {
  return [
    "rounded-lg border border-border-subtle bg-background-elevated p-4 transition duration-200",
    groupKey === "overdue" ? "border-accent-border bg-accent-soft" : "",
    groupKey === "today" ? "border-status-fair/30 bg-status-fair/10" : "",
    paid ? "opacity-75" : "",
  ]
    .filter(Boolean)
    .join(" ");
}
