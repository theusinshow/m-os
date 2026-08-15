import { addCardExpense, deleteCardExpense } from "@/app/actions/card-expenses";
import { markInvoiceAsPaid, markInvoiceAsPending } from "@/app/actions/invoices";
import { CardBrandMark } from "@/components/cards/card-brand-mark";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { FormSubmitButton } from "@/components/form-submit-button";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { StatusBadge } from "@/components/status-badge";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput } from "@/components/ui/validated-form";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";

type Card = { id: string; name: string; cardType: "personal" | "business"; dueDay: number };
type Expense = {
  id: string;
  description: string;
  amountCents: number;
  purchaseDate: string | null;
};
type Invoice = {
  id: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
} | null;

const fieldClass =
  "focus-ring min-h-11 w-full rounded-md border border-border-subtle bg-background-elevated px-3 text-sm text-text-primary placeholder:text-text-muted";

export function CardDetail({
  card,
  expenses,
  invoice,
  monthLabel,
}: {
  card: Card;
  expenses: Expense[];
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
              <ToastForm action={markInvoiceAsPaid} successMessage="Fatura marcada como paga.">
                <input name="invoiceId" type="hidden" value={invoice.id} />
                <FormSubmitButton pendingLabel="Marcando..." variant="success">
                  Marcar fatura como paga
                </FormSubmitButton>
              </ToastForm>
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
        description="Lance cada compra do cartão. O total da fatura passa a ser a soma delas."
        title="Lançar compra"
      >
        <ValidatedForm
          action={addCardExpense}
          successMessage="Compra lançada."
          resetOnSuccess
          className="grid gap-4 md:grid-cols-[1fr_180px_170px_auto] md:items-end"
        >
          <input name="cardId" type="hidden" value={card.id} />
          <div>
            <label
              className="mb-2 block text-sm font-medium text-text-secondary"
              htmlFor="expense-description"
            >
              Descrição
            </label>
            <ValidatedInput
              autoComplete="off"
              className={fieldClass}
              id="expense-description"
              name="description"
              placeholder="Mercado, assinatura, gasolina…"
              required
            />
          </div>
          <div>
            <label
              className="mb-2 block text-sm font-medium text-text-secondary"
              htmlFor="expense-amount"
            >
              Valor
            </label>
            <ValidatedInput
              autoComplete="off"
              className={fieldClass}
              id="expense-amount"
              inputMode="decimal"
              name="amount"
              placeholder="120,00"
              required
            />
          </div>
          <div>
            <label
              className="mb-2 block text-sm font-medium text-text-secondary"
              htmlFor="expense-date"
            >
              Data (opcional)
            </label>
            <ValidatedInput className={fieldClass} id="expense-date" name="purchaseDate" type="date" />
          </div>
          <FormSubmitButton pendingLabel="Lançando...">Lançar</FormSubmitButton>
        </ValidatedForm>
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
                className="flex items-center justify-between gap-3 rounded-lg border border-border-subtle bg-background-elevated px-4 py-3"
                key={expense.id}
              >
                <div className="min-w-0">
                  <p className="truncate font-medium text-text-primary">{expense.description}</p>
                  {expense.purchaseDate ? (
                    <p className="mt-0.5 text-xs text-text-muted">
                      {formatShortDate(expense.purchaseDate)}
                    </p>
                  ) : null}
                </div>
                <div className="flex shrink-0 items-center gap-3">
                  <p className="num font-semibold text-text-primary">
                    {formatCurrency(expense.amountCents)}
                  </p>
                  <ToastForm action={deleteCardExpense} successMessage="Compra excluída.">
                    <input name="expenseId" type="hidden" value={expense.id} />
                    <input name="cardId" type="hidden" value={card.id} />
                    <ConfirmDeleteButton confirmMessage="Excluir esta compra?">
                      Excluir
                    </ConfirmDeleteButton>
                  </ToastForm>
                </div>
              </div>
            ))}
          </div>
        )}
      </DashboardCard>
    </div>
  );
}
