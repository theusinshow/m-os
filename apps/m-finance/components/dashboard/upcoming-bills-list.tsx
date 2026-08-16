import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { MarkPaidButton } from "@/components/payable/mark-paid-button";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { StatusBadge } from "@/components/status-badge";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";
import { cn } from "@/lib/utils";

type Payable = {
  id: string;
  type: "bill" | "invoice";
  name: string;
  label: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
};

type Bill = {
  id: string;
  name: string;
  categoryName: string | null;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
};

type Invoice = {
  id: string;
  name: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
  cardType?: "personal" | "business";
};

function toDateOnly(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function diffInDays(date: string, today = new Date()) {
  const due = new Date(`${date}T12:00:00`);
  return Math.round((toDateOnly(due).getTime() - toDateOnly(today).getTime()) / 86_400_000);
}

function groupPayables(payables: Payable[]) {
  const groups = [
    { key: "overdue", title: "Vencidas", items: [] as Payable[] },
    { key: "today", title: "Hoje", items: [] as Payable[] },
    { key: "week", title: "Próximos 7 dias", items: [] as Payable[] },
    { key: "later", title: "Depois", items: [] as Payable[] },
    { key: "paid", title: "Pagas", items: [] as Payable[] },
  ];

  for (const payable of payables) {
    const days = diffInDays(payable.dueDate);
    if (payable.status === "paid") groups[4].items.push(payable);
    else if (payable.status === "overdue" || days < 0) groups[0].items.push(payable);
    else if (days === 0) groups[1].items.push(payable);
    else if (days <= 7) groups[2].items.push(payable);
    else groups[3].items.push(payable);
  }

  for (const group of groups) {
    group.items.sort((a, b) => a.dueDate.localeCompare(b.dueDate));
  }

  return groups.filter((group) => group.items.length > 0);
}

export function UpcomingBillsList({
  bills,
  invoices = [],
}: {
  bills: Bill[];
  invoices?: Invoice[];
}) {
  const payables: Payable[] = [
    ...bills.map((bill) => ({
      id: bill.id,
      type: "bill" as const,
      name: bill.name,
      label: bill.categoryName ?? "Conta",
      amountCents: bill.amountCents,
      dueDate: bill.dueDate,
      status: bill.status,
    })),
    ...invoices.map((invoice) => ({
      id: invoice.id,
      type: "invoice" as const,
      name: invoice.name,
      label: invoice.cardType === "business" ? "Fatura PJ" : "Fatura",
      amountCents: invoice.amountCents,
      dueDate: invoice.dueDate,
      status: invoice.status,
    })),
  ];
  const groups = groupPayables(payables);

  return (
    <DashboardCard title="Agenda de vencimentos">
      <div className="space-y-5">
        {payables.length === 0 ? (
          <InlineEmpty>Nenhuma conta ou fatura cadastrada para este mês.</InlineEmpty>
        ) : (
          groups.map((group) => (
            <section className="space-y-2" key={group.key}>
              <div className="flex items-center justify-between gap-3">
                <h3 className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                  {group.title}
                </h3>
                <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold text-text-muted">
                  {group.items.length}
                </span>
              </div>
              <div className="space-y-2">
                {group.items.map((payable) => (
                  <div
                    className={cn(
                      "grid gap-3 rounded-lg border border-border-subtle bg-background-elevated p-4 sm:grid-cols-[1fr_auto_auto] sm:items-center",
                      group.key === "overdue" && "border-accent-border bg-accent-soft",
                      group.key === "today" && "border-status-fair/30 bg-status-fair/10",
                      group.key === "paid" && "opacity-70",
                    )}
                    key={`${payable.type}-${payable.id}`}
                  >
                    <div className="min-w-0">
                      <p className="truncate font-semibold text-text-primary">{payable.name}</p>
                      <p className="mt-1 text-sm text-text-muted">
                        {payable.label} · vence {formatShortDate(payable.dueDate)}
                      </p>
                    </div>
                    <p className="num text-lg font-semibold text-text-primary sm:text-sm">
                      {formatCurrency(payable.amountCents)}
                    </p>
                    <div className="flex flex-col items-stretch gap-2 sm:flex-row sm:items-center">
                      <StatusBadge status={payable.status} />
                      {payable.status !== "paid" ? (
                        <MarkPaidButton
                          payableId={payable.id}
                          payableType={payable.type}
                          variant="secondary"
                        >
                          Marcar pago
                        </MarkPaidButton>
                      ) : null}
                    </div>
                  </div>
                ))}
              </div>
            </section>
          ))
        )}
      </div>
    </DashboardCard>
  );
}
