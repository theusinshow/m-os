import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { markBillAsPaid } from "@/app/actions/bills";
import { FormSubmitButton } from "@/components/form-submit-button";
import { ToastForm } from "@/components/toast-form";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { StatusBadge } from "@/components/status-badge";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";

type Bill = {
  id: string;
  name: string;
  categoryName: string | null;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
};

export function UpcomingBillsList({ bills }: { bills: Bill[] }) {
  return (
    <DashboardCard title="Próximos vencimentos">
      <div className="space-y-3">
        {bills.length === 0 ? (
          <InlineEmpty>Nenhuma despesa cadastrada para este mês.</InlineEmpty>
        ) : (
          bills.map((bill) => (
            <div
              className="grid gap-3 rounded-lg border border-border-subtle bg-background-elevated p-4 sm:grid-cols-[1fr_auto_auto] sm:items-center"
              key={bill.id}
            >
              <div>
                <p className="font-semibold text-text-primary">{bill.name}</p>
                <p className="mt-1 text-sm text-text-muted">
                  {bill.categoryName ?? "Sem categoria"} · vence {formatShortDate(bill.dueDate)}
                </p>
              </div>
              <p className="num text-sm font-semibold text-text-primary">
                {formatCurrency(bill.amountCents)}
              </p>
              <div className="flex items-center gap-2">
                <StatusBadge status={bill.status} />
                {bill.status !== "paid" ? (
                  <ToastForm action={markBillAsPaid} successMessage="Conta marcada como paga.">
                    <input name="billId" type="hidden" value={bill.id} />
                    <FormSubmitButton pendingLabel="Marcando..." variant="secondary">
                      Marcar como pago
                    </FormSubmitButton>
                  </ToastForm>
                ) : null}
              </div>
            </div>
          ))
        )}
      </div>
    </DashboardCard>
  );
}
