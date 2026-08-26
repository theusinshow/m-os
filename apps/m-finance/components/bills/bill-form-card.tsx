import {
  deleteBill,
  deleteBillSeries,
  updateBill,
} from "@/app/actions/bills";
import { QuickAddExpense } from "@/components/bills/quick-add-expense";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { PayableList, type PayableItem } from "@/components/payable/payable-list";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput, ValidatedSelect } from "@/components/ui/validated-form";
import { formatShortDate } from "@/lib/formatters/date";
import { dayFromIsoDate } from "@/lib/due-date";
import { centsToInput } from "@/lib/money";
import { payableKey, payableProgress } from "@/lib/payables";

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

type Invoice = {
  id: string;
  cardId: string;
  name: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
  cardType: "personal" | "business";
};

const editInputClass = "field-input";

function todayIsoDate(today = new Date()) {
  return `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(
    today.getDate(),
  ).padStart(2, "0")}`;
}

function billDetail(bill: Bill) {
  const suffix =
    bill.seriesNumber && bill.seriesTotal
      ? ` · mês ${bill.seriesNumber}/${bill.seriesTotal}`
      : bill.isRecurring
        ? " · recorrente"
        : "";
  return `${bill.categoryName ?? "Sem categoria"} · vence ${formatShortDate(bill.dueDate)}${suffix}`;
}

/**
 * A lista única do mês: contas e faturas de cartão na mesma pilha. O dono põe
 * quase tudo no cartão — deixar a fatura fora daqui faria a lista mentir sobre
 * quanto ainda falta pagar.
 */
export function BillFormCard({
  bills,
  invoices,
  categories,
}: {
  bills: Bill[];
  invoices: Invoice[];
  categories: Category[];
}) {
  const items: PayableItem[] = [
    ...bills.map((bill) => ({
      id: bill.id,
      type: "bill" as const,
      name: bill.name,
      amountCents: bill.amountCents,
      dueDate: bill.dueDate,
      status: bill.status,
      detail: billDetail(bill),
    })),
    ...invoices.map((invoice) => ({
      id: invoice.id,
      type: "invoice" as const,
      name: invoice.name,
      amountCents: invoice.amountCents,
      dueDate: invoice.dueDate,
      status: invoice.status,
      detail: `Fatura${invoice.cardType === "business" ? " PJ" : ""} · vence ${formatShortDate(
        invoice.dueDate,
      )}`,
    })),
  ];

  const editSlots = Object.fromEntries(
    bills.map((bill) => [
      payableKey({ id: bill.id, type: "bill" }),
      <BillEditForm bill={bill} categories={categories} key={bill.id} />,
    ]),
  );

  const progress = payableProgress(items);

  return (
    <div className="space-y-4">
      <DashboardCard accent>
        <QuickAddExpense
          categories={categories}
          paidCount={progress.paidCount}
          pendingCount={progress.remainingCount}
          totalPendingCents={progress.remainingCents}
        />
      </DashboardCard>

      <DashboardCard
        description="Contas e faturas do mês, agrupadas por urgência. Arraste a linha para marcar paga."
        title="A pagar"
      >
        <PayableList editSlots={editSlots} items={items} todayIso={todayIsoDate()} />
      </DashboardCard>
    </div>
  );
}

function BillEditForm({ bill, categories }: { bill: Bill; categories: Category[] }) {
  return (
    <>
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
        <ValidatedSelect
          className={editInputClass}
          defaultValue={bill.categoryId ?? ""}
          name="categoryId"
        >
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
    </>
  );
}
