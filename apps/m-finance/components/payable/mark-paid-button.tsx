"use client";

import { useTransition } from "react";
import { LoaderCircle } from "lucide-react";
import { markBillAsPaid, markBillAsPending } from "@/app/actions/bills";
import { markInvoiceAsPaid, markInvoiceAsPending } from "@/app/actions/invoices";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { useToast } from "@/components/ui/toast";
import { cn } from "@/lib/utils";

/**
 * A ação mais usada do app (PRODUCT.md §6.2): marcar como pago.
 * Mostra toast com "Desfazer", que reverte para pendente sem sair da tela.
 */
export function MarkPaidButton({
  payableType,
  payableId,
  variant = "secondary",
  children,
}: {
  payableType: "bill" | "invoice";
  payableId: string;
  variant?: "secondary" | "success";
  children: React.ReactNode;
}) {
  const [pending, startTransition] = useTransition();
  const { addToast } = useToast();

  const idField = payableType === "bill" ? "billId" : "invoiceId";
  const paidMessage = payableType === "bill" ? "Conta marcada como paga." : "Fatura marcada como paga.";
  const reopenedMessage = payableType === "bill" ? "Conta reaberta." : "Fatura reaberta.";

  function revert() {
    startTransition(async () => {
      try {
        const formData = new FormData();
        formData.set(idField, payableId);
        if (payableType === "bill") {
          await markBillAsPending(formData);
        } else {
          await markInvoiceAsPending(formData);
        }
        addToast(reopenedMessage, "info");
      } catch {
        addToast("Não foi possível desfazer. Tente reabrir na lista.", "error");
      }
    });
  }

  function handleClick() {
    startTransition(async () => {
      try {
        const formData = new FormData();
        formData.set(idField, payableId);
        if (payableType === "bill") {
          await markBillAsPaid(formData);
        } else {
          await markInvoiceAsPaid(formData);
        }
        addToast(paidMessage, "success", { label: "Desfazer", onClick: revert });
      } catch {
        addToast("Não foi possível marcar como pago.", "error");
      }
    });
  }

  return (
    <button
      aria-busy={pending}
      className={cn(
        "clip-notch sheen group focus-ring relative inline-flex min-h-11 w-full items-center justify-center gap-2 px-4 text-sm font-semibold tracking-tight transition duration-200 disabled:cursor-wait disabled:opacity-75 sm:w-auto",
        variant === "secondary" &&
          "border border-border-default bg-background-elevated text-text-secondary hover:border-border-strong hover:bg-background-hover hover:text-text-primary active:scale-[0.985]",
        variant === "success" &&
          "bg-status-positive text-text-inverse shadow-lg shadow-status-positive/20 hover:brightness-110 active:scale-[0.985]",
      )}
      disabled={pending}
      onClick={handleClick}
      type="button"
    >
      {pending ? (
        <LoaderCircle className="animate-spin" size={16} aria-hidden="true" />
      ) : (
        <TriangleMark
          className="rotate-90 opacity-60 transition-transform duration-300 group-hover:translate-x-0.5"
          size={11}
          variant="solid"
        />
      )}
      <span>{pending ? "Marcando..." : children}</span>
    </button>
  );
}
