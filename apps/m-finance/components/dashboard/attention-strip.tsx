import { AlertTriangle, CalendarClock, CheckCircle2 } from "lucide-react";
import { MarkPaidButton } from "@/components/payable/mark-paid-button";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatShortDate } from "@/lib/formatters/date";
import { cn } from "@/lib/utils";

type AttentionItem = {
  id: string;
  type: "bill" | "invoice";
  title: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
};

function toDateOnly(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function diffInDays(date: string, today = new Date()) {
  const due = new Date(`${date}T12:00:00`);
  return Math.round((toDateOnly(due).getTime() - toDateOnly(today).getTime()) / 86_400_000);
}

function urgencyScore(item: AttentionItem) {
  if (item.status === "overdue") return -1000;
  return diffInDays(item.dueDate);
}

function getItemCopy(item: AttentionItem) {
  const days = diffInDays(item.dueDate);
  const label = item.type === "invoice" ? "Fatura" : "Conta";

  if (item.status === "overdue" || days < 0) {
    return {
      eyebrow: `${label} vencida`,
      title: item.title,
      detail: `Venceu em ${formatShortDate(item.dueDate)}`,
      tone: "danger" as const,
    };
  }

  if (days === 0) {
    return {
      eyebrow: `${label} vence hoje`,
      title: item.title,
      detail: "Resolve isso primeiro",
      tone: "warning" as const,
    };
  }

  if (days === 1) {
    return {
      eyebrow: `${label} vence amanhã`,
      title: item.title,
      detail: `Vencimento em ${formatShortDate(item.dueDate)}`,
      tone: "warning" as const,
    };
  }

  return {
    eyebrow: `Próximo vencimento em ${days} dias`,
    title: item.title,
    detail: `${label} em ${formatShortDate(item.dueDate)}`,
    tone: "neutral" as const,
  };
}

export function AttentionStrip({ items }: { items: AttentionItem[] }) {
  const openItems = items
    .filter((item) => item.status !== "paid")
    .sort((a, b) => urgencyScore(a) - urgencyScore(b));
  const next = openItems[0];
  const dueSoonTotal = openItems
    .filter((item) => diffInDays(item.dueDate) <= 7)
    .reduce((total, item) => total + item.amountCents, 0);

  if (!next) {
    return (
      <section className="rounded-xl border border-status-positive/30 bg-status-positive/10 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <CheckCircle2 className="shrink-0 text-status-positive" size={20} aria-hidden="true" />
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-status-positive">
                Próxima ação
              </p>
              <p className="mt-1 font-semibold text-text-primary">
                Nenhuma conta ou fatura pendente neste mês.
              </p>
            </div>
          </div>
          <p className="text-sm text-text-muted">O mês está limpo no radar de vencimentos.</p>
        </div>
      </section>
    );
  }

  const copy = getItemCopy(next);

  return (
    <section
      className={cn(
        "rounded-xl border p-4",
        copy.tone === "danger" && "border-accent-border bg-accent-soft",
        copy.tone === "warning" && "border-status-fair/30 bg-status-fair/10",
        copy.tone === "neutral" && "border-border-subtle bg-background-card/95",
      )}
    >
      <div className="grid gap-4 lg:grid-cols-[1fr_auto] lg:items-center">
        <div className="flex items-start gap-3">
          {copy.tone === "danger" ? (
            <AlertTriangle className="mt-1 shrink-0 text-accent" size={20} aria-hidden="true" />
          ) : (
            <CalendarClock className="mt-1 shrink-0 text-status-fair" size={20} aria-hidden="true" />
          )}
          <div className="min-w-0">
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
              {copy.eyebrow}
            </p>
            <div className="mt-1 flex flex-wrap items-baseline gap-x-3 gap-y-1">
              <h2 className="text-xl font-semibold text-text-primary">{copy.title}</h2>
              <span className="num text-lg font-semibold text-text-secondary">
                {formatCurrency(next.amountCents)}
              </span>
            </div>
            <p className="mt-1 text-sm text-text-muted">
              {copy.detail}
              {dueSoonTotal > 0
                ? ` · ${formatCurrency(dueSoonTotal)} vencem nos próximos 7 dias`
                : ""}
            </p>
          </div>
        </div>
        <MarkPaidButton payableId={next.id} payableType={next.type} variant="success">
          Marcar pago
        </MarkPaidButton>
      </div>
    </section>
  );
}
