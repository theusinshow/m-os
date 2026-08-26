"use client";

import { useOptimistic, useTransition, type ReactNode } from "react";
import { Check, LoaderCircle } from "lucide-react";
import { markBillAsPaid, markBillAsPending } from "@/app/actions/bills";
import { markInvoiceAsPaid, markInvoiceAsPending } from "@/app/actions/invoices";
import { SwipeToPay } from "@/components/payable/swipe-to-pay";
import { EditDisclosure } from "@/components/ui/edit-disclosure";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { StatusBadge } from "@/components/status-badge";
import { useToast } from "@/components/ui/toast";
import { formatCurrency } from "@/lib/formatters/currency";
import {
  groupPayables,
  payableKey,
  payableProgress,
  type Payable,
  type PayableStatus,
} from "@/lib/payables";
import { cn } from "@/lib/utils";

export type PayableItem = Payable & {
  /** Linha de apoio já montada no servidor: categoria, vencimento, parcela. */
  detail: string;
};

/* `payableKey` vive em `@/lib/payables`, e não é reexportada daqui de
   propósito: reexportar devolveria ao servidor o mesmo caminho de import que
   quebrou a página de Contas — de um módulo `"use client"`, o que chega do
   outro lado é uma referência, não a função. */

export function PayableList({
  items,
  todayIso,
  editSlots,
}: {
  items: PayableItem[];
  /** O "hoje" vem do servidor para servidor e cliente agruparem igual. */
  todayIso: string;
  editSlots?: Record<string, ReactNode>;
}) {
  const { addToast } = useToast();
  const [isPending, startTransition] = useTransition();
  const [optimisticItems, applyOptimistic] = useOptimistic(
    items,
    (state, update: { key: string; status: PayableStatus }) =>
      state.map((item) =>
        payableKey(item) === update.key ? { ...item, status: update.status } : item,
      ),
  );

  const progress = payableProgress(optimisticItems);
  const groups = groupPayables(optimisticItems, new Date(`${todayIso}T12:00:00`));

  function run(item: PayableItem, next: PayableStatus) {
    const isBill = item.type === "bill";
    const paying = next === "paid";

    startTransition(async () => {
      // A linha fica verde no dedo; o servidor confirma depois. Se falhar, o
      // useOptimistic devolve o estado real sozinho e o toast explica.
      applyOptimistic({ key: payableKey(item), status: next });

      const formData = new FormData();
      formData.set(isBill ? "billId" : "invoiceId", item.id);

      try {
        if (isBill) {
          await (paying ? markBillAsPaid(formData) : markBillAsPending(formData));
        } else {
          await (paying ? markInvoiceAsPaid(formData) : markInvoiceAsPending(formData));
        }

        if (paying) {
          addToast(isBill ? "Conta marcada como paga." : "Fatura marcada como paga.", "success", {
            label: "Desfazer",
            onClick: () => run(item, "pending"),
          });
        } else {
          addToast(isBill ? "Conta reaberta." : "Fatura reaberta.", "info");
        }
      } catch {
        addToast(
          paying ? "Não foi possível marcar como pago." : "Não foi possível reabrir.",
          "error",
        );
      }
    });
  }

  if (items.length === 0) {
    return <InlineEmpty>Nada para pagar neste mês. Nenhuma conta ou fatura em aberto.</InlineEmpty>;
  }

  return (
    <div className="space-y-5">
      <ProgressHeader isPending={isPending} progress={progress} />

      {groups.map((group) => (
        <section className="space-y-2" key={group.key}>
          <div className="flex flex-wrap items-end justify-between gap-2">
            <div>
              <h3 className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
                {group.title}
              </h3>
              <p className="mt-1 text-xs text-text-muted">{group.description}</p>
            </div>
            <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold text-text-muted">
              {group.items.filter((item) => item.status !== "paid").length}/{group.items.length}
            </span>
          </div>
          <div className="space-y-3">
            {group.items.map((item) => {
              const key = payableKey(item);
              return (
                <SwipeToPay
                  key={key}
                  onPay={() => run(item, "paid")}
                  onReopen={() => run(item, "pending")}
                  paid={item.status === "paid"}
                >
                  <PayableRow
                    edit={editSlots?.[key]}
                    groupKey={group.key}
                    item={item}
                    onToggle={() => run(item, item.status === "paid" ? "pending" : "paid")}
                  />
                </SwipeToPay>
              );
            })}
          </div>
        </section>
      ))}
    </div>
  );
}

function ProgressHeader({
  progress,
  isPending,
}: {
  progress: ReturnType<typeof payableProgress>;
  isPending: boolean;
}) {
  const done = progress.remainingCount === 0;

  return (
    <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border-subtle bg-background-elevated px-4 py-3">
      <div className="flex items-center gap-2">
        {isPending ? (
          <LoaderCircle className="animate-spin text-text-muted" size={16} aria-hidden="true" />
        ) : null}
        <p
          aria-live="polite"
          className={cn(
            "text-sm font-semibold",
            done ? "text-status-positive" : "text-text-primary",
          )}
        >
          {done
            ? "Tudo pago neste mês."
            : `Faltam ${progress.remainingCount} de ${progress.totalCount}`}
        </p>
      </div>
      <p className="num text-sm font-semibold text-text-secondary">
        {done ? formatCurrency(progress.paidCents) : formatCurrency(progress.remainingCents)}
      </p>
    </div>
  );
}

function PayableRow({
  item,
  groupKey,
  edit,
  onToggle,
}: {
  item: PayableItem;
  groupKey: string;
  edit?: ReactNode;
  onToggle: () => void;
}) {
  const paid = item.status === "paid";

  return (
    <div
      className={cn(
        "rounded-lg border p-4 transition duration-200",
        paid
          ? "border-status-positive/45 bg-status-positive/10"
          : "border-border-subtle bg-background-elevated",
        !paid && groupKey === "overdue" && "border-accent-border bg-accent-soft",
        !paid && groupKey === "today" && "border-status-fair/30 bg-status-fair/10",
      )}
      data-paid={paid ? "true" : undefined}
    >
      <div className="grid gap-3 sm:grid-cols-[1fr_auto] sm:items-center">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            {paid ? (
              <Check className="shrink-0 text-status-positive" size={16} aria-hidden="true" />
            ) : null}
            <p
              className={cn(
                "font-semibold",
                paid ? "text-status-positive" : "text-text-primary",
              )}
            >
              {item.name}
            </p>
            <StatusBadge status={item.status} />
            {item.type === "invoice" ? (
              <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-[0.12em] text-text-muted">
                Fatura
              </span>
            ) : null}
          </div>
          <p className="mt-1 text-sm text-text-muted">{item.detail}</p>
        </div>
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-end">
          <p
            className={cn(
              "num text-lg font-semibold sm:text-base",
              paid ? "text-status-positive" : "text-text-primary",
            )}
          >
            {formatCurrency(item.amountCents)}
          </p>
          {/* Discreto de propósito: o verde desta lista significa "pago". Um
              botão verde em cada linha pendente roubaria esse significado —
              e some dentro da faixa verde do arrasto. */}
          <button
            className="clip-notch sheen focus-ring relative inline-flex min-h-11 w-full items-center justify-center gap-2 border border-border-default bg-background-card px-4 text-sm font-semibold tracking-tight text-text-secondary transition duration-200 hover:border-border-strong hover:bg-background-hover hover:text-text-primary active:scale-[0.985] sm:w-auto"
            onClick={onToggle}
            type="button"
          >
            {paid ? "Reabrir" : "Marcar pago"}
          </button>
        </div>
      </div>

      {edit ? <EditDisclosure className="mt-3">{edit}</EditDisclosure> : null}
    </div>
  );
}
