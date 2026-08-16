import {
  deleteCardExpense,
  deleteCardExpenseSeries,
} from "@/app/actions/card-expenses";
import { markInvoiceAsPending } from "@/app/actions/invoices";
import { CardBrandMark } from "@/components/cards/card-brand-mark";
import { CardExpenseForm } from "@/components/cards/card-expense-form";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { MarkPaidButton } from "@/components/payable/mark-paid-button";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { StatusBadge } from "@/components/status-badge";
import { ToastForm } from "@/components/toast-form";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";

type Card = { id: string; name: string; cardType: "personal" | "business"; dueDay: number };
type Expense = {
  id: string;
  description: string;
  amountCents: number;
  purchaseDate: string | null;
  installmentId: string | null;
  installmentNumber: number | null;
  installmentTotal: number | null;
};
type HistoryExpense = Expense & {
  monthLabel: string;
};
type Invoice = {
  id: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
} | null;

export function CardDetail({
  card,
  expenses,
  history,
  invoice,
  monthLabel,
}: {
  card: Card;
  expenses: Expense[];
  history: HistoryExpense[];
  invoice: Invoice;
  monthLabel: string;
}) {
  const itemsTotalCents = expenses.reduce((total, item) => total + item.amountCents, 0);
  const hasItems = expenses.length > 0;
  const displayTotalCents = hasItems ? itemsTotalCents : (invoice?.amountCents ?? 0);
  const caption = hasItems
    ? `Total derivado de ${expenses.length} compra${expenses.length === 1 ? "" : "s"}`
    : invoice
      ? "Total lançado manualmente"
      : "Nenhuma compra lançada neste mês";

  return (
    <div className="space-y-6">
      <DashboardCard accent>
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="flex items-start gap-3">
            <CardBrandMark name={card.name} size={18} />
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-lg font-semibold text-text-primary">{card.name}</h2>
                {card.cardType === "business" ? (
                  <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                    PJ
                  </span>
                ) : null}
              </div>
              <p className="mt-1 text-sm text-text-muted">
                {monthLabel} · vence dia {card.dueDay}
              </p>
            </div>
          </div>
          {invoice ? <StatusBadge status={invoice.status} /> : null}
        </div>

        <p className="num mt-5 text-4xl font-semibold text-text-primary">
          {formatCurrency(displayTotalCents)}
        </p>
        <p className="mt-1.5 text-sm text-text-muted">{caption}</p>

        {invoice ? (
          <div className="mt-5">
            {invoice.status !== "paid" ? (
              <MarkPaidButton payableId={invoice.id} payableType="invoice" variant="success">
                Marcar fatura como paga
              </MarkPaidButton>
            ) : (
              <ToastForm action={markInvoiceAsPending} successMessage="Fatura reaberta.">
                <input name="invoiceId" type="hidden" value={invoice.id} />
                <FormSubmitButton pendingLabel="Reabrindo..." variant="secondary">
                  Reabrir fatura
                </FormSubmitButton>
              </ToastForm>
            )}
          </div>
        ) : null}
      </DashboardCard>

      <DashboardCard
        description="Compras parceladas são distribuídas a partir do mês selecionado."
        title="Lançar compra"
      >
        <CardExpenseForm cardId={card.id} />
      </DashboardCard>

      <DashboardCard title="Compras do mês">
        {expenses.length === 0 ? (
          <InlineEmpty>
            Nenhuma compra lançada. Adicione acima ou deixe só o total manual na tela de Cartões.
          </InlineEmpty>
        ) : (
          <div className="space-y-2">
            {expenses.map((expense) => (
              <div
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border-subtle bg-background-elevated px-4 py-3"
                key={expense.id}
              >
                <div className="min-w-0">
                  <p className="truncate font-medium text-text-primary">{expense.description}</p>
                  <p className="mt-0.5 text-xs text-text-muted">
                    {expense.installmentNumber && expense.installmentTotal
                      ? `Parcela ${expense.installmentNumber}/${expense.installmentTotal}`
                      : "À vista"}
                    {expense.purchaseDate
                      ? ` · ${formatShortDate(expense.purchaseDate)}`
                      : ""}
                  </p>
                </div>
                <div className="flex flex-wrap items-center justify-end gap-2">
                  <p className="num font-semibold text-text-primary">
                    {formatCurrency(expense.amountCents)}
                  </p>
                  <ToastForm action={deleteCardExpense} successMessage="Compra excluída.">
                    <input name="expenseId" type="hidden" value={expense.id} />
                    <input name="cardId" type="hidden" value={card.id} />
                    <ConfirmDeleteButton confirmMessage="Excluir esta compra?">
                      {expense.installmentId ? "Excluir parcela" : "Excluir"}
                    </ConfirmDeleteButton>
                  </ToastForm>
                  {expense.installmentId ? (
                    <ToastForm
                      action={deleteCardExpenseSeries}
                      successMessage="Parcelamento excluído."
                    >
                      <input
                        name="installmentId"
                        type="hidden"
                        value={expense.installmentId}
                      />
                      <input name="cardId" type="hidden" value={card.id} />
                      <ConfirmDeleteButton confirmMessage="Excluir todas as parcelas desta compra?">
                        Excluir todas
                      </ConfirmDeleteButton>
                    </ToastForm>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        )}
      </DashboardCard>

      <DashboardCard
        description="Tudo que já foi lançado neste cartão, independente do mês selecionado."
        title="Histórico do cartão"
      >
        {history.length === 0 ? (
          <InlineEmpty>Nenhuma compra cadastrada neste cartão.</InlineEmpty>
        ) : (
          <div className="space-y-2">
            {history.map((expense) => (
              <div
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border-subtle bg-background-elevated px-4 py-3"
                key={expense.id}
              >
                <div className="min-w-0">
                  <p className="truncate font-medium text-text-primary">{expense.description}</p>
                  <p className="mt-0.5 text-xs text-text-muted">
                    {expense.monthLabel}
                    {expense.installmentNumber && expense.installmentTotal
                      ? ` · parcela ${expense.installmentNumber}/${expense.installmentTotal}`
                      : " · à vista"}
                    {expense.purchaseDate ? ` · ${formatShortDate(expense.purchaseDate)}` : ""}
                  </p>
                </div>
                <p className="num font-semibold text-text-primary">
                  {formatCurrency(expense.amountCents)}
                </p>
              </div>
            ))}
          </div>
        )}
      </DashboardCard>
    </div>
  );
}
