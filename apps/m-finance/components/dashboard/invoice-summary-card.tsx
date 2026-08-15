import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { deleteInvoice, markInvoiceAsPaid, markInvoiceAsPending, updateInvoice } from "@/app/actions/invoices";
import { ConfirmDeleteButton } from "@/components/confirm-delete-button";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { ValidatedForm, ValidatedInput } from "@/components/ui/validated-form";
import { CardBrandMark } from "@/components/cards/card-brand-mark";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { StatusBadge } from "@/components/status-badge";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";
import { dayFromIsoDate } from "@/lib/due-date";
import { centsToInput } from "@/lib/money";

type Invoice = {
  id: string;
  name: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
  cardType?: "personal" | "business";
};

export function InvoiceSummaryCard({ invoices }: { invoices: Invoice[] }) {
  return (
    <DashboardCard title="Faturas">
      <div className="space-y-3">
        {invoices.length === 0 ? (
          <InlineEmpty>Nenhuma fatura cadastrada para este mês.</InlineEmpty>
        ) : invoices.map((invoice) => (
          <div className="rounded-lg border border-border-subtle bg-background-elevated p-4" key={invoice.id}>
            <div className="flex items-start justify-between gap-4">
              <div className="flex items-start gap-3">
                <CardBrandMark name={invoice.name} />
                <div>
                  <div className="flex items-center gap-2">
                    <p className="font-semibold text-text-primary">{invoice.name}</p>
                    {invoice.cardType === "business" ? (
                      <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                        PJ
                      </span>
                    ) : null}
                  </div>
                  <p className="mt-1 text-sm text-text-muted">
                    Vence {formatShortDate(invoice.dueDate)}
                  </p>
                </div>
              </div>
              <StatusBadge status={invoice.status} />
            </div>
            <div className="mt-4 flex items-center justify-between gap-3">
              <p className="num text-xl font-semibold text-text-primary">
                {formatCurrency(invoice.amountCents)}
              </p>
              {invoice.status !== "paid" ? (
                <ToastForm action={markInvoiceAsPaid} successMessage="Fatura marcada como paga.">
                  <input name="invoiceId" type="hidden" value={invoice.id} />
                  <FormSubmitButton pendingLabel="Marcando..." variant="secondary">
                    Marcar como paga
                  </FormSubmitButton>
                </ToastForm>
              ) : (
                <ToastForm action={markInvoiceAsPending} successMessage="Fatura reaberta.">
                  <input name="invoiceId" type="hidden" value={invoice.id} />
                  <FormSubmitButton pendingLabel="Reabrindo..." variant="secondary">
                    Marcar como pendente
                  </FormSubmitButton>
                </ToastForm>
              )}
            </div>
            <EditDisclosure className="mt-4">
              <ValidatedForm action={updateInvoice} successMessage="Fatura atualizada." className="grid gap-3">
                <input name="invoiceId" type="hidden" value={invoice.id} />
                <div className="grid gap-3 sm:grid-cols-2">
                  <ValidatedInput
                    className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                    defaultValue={centsToInput(invoice.amountCents)}
                    inputMode="decimal"
                    name="amount"
                    required
                  />
                  <ValidatedInput
                    aria-label="Dia do vencimento"
                    className="focus-ring min-h-11 rounded-md border border-border-subtle bg-background-card px-3 text-sm text-text-primary"
                    defaultValue={dayFromIsoDate(invoice.dueDate)}
                    inputMode="numeric"
                    max={31}
                    min={1}
                    name="dueDay"
                    placeholder="Dia"
                    type="number"
                  />
                </div>
                <FormSubmitButton pendingLabel="Salvando...">
                  Salvar fatura
                </FormSubmitButton>
              </ValidatedForm>
              <ToastForm action={deleteInvoice} successMessage="Fatura excluída." className="mt-2">
                <input name="invoiceId" type="hidden" value={invoice.id} />
                <ConfirmDeleteButton confirmMessage="Excluir esta fatura? Essa ação não pode ser desfeita.">
                  Excluir fatura
                </ConfirmDeleteButton>
              </ToastForm>
            </EditDisclosure>
          </div>
        ))}
      </div>
    </DashboardCard>
  );
}
