type Payable = {
  id: string;
  type: "bill" | "invoice";
  title: string;
  amountCents: number;
  dueDate: string;
  status: "pending" | "paid" | "overdue";
};

export type InternalAlert = {
  id: string;
  type: "due_in_3_days" | "due_today" | "overdue";
  severity: "info" | "warning" | "danger";
  title: string;
  message: string;
  entityType: "bill" | "invoice";
  entityId: string;
};

function toDateOnly(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function diffInDays(date: string, today = new Date()) {
  const due = new Date(`${date}T12:00:00`);
  return Math.round((toDateOnly(due).getTime() - toDateOnly(today).getTime()) / 86_400_000);
}

export function calculateInternalAlerts(
  payables: Payable[],
  options: { today?: Date; daysBefore?: number } = {},
): InternalAlert[] {
  const today = options.today ?? new Date();
  const daysBefore = options.daysBefore ?? 3;

  return payables
    .filter((payable) => payable.status !== "paid")
    .flatMap((payable): InternalAlert[] => {
      const days = diffInDays(payable.dueDate, today);
      const label = payable.type === "invoice" ? "Fatura" : "Despesa";

      if (days < 0 || payable.status === "overdue") {
        return [{
          id: `${payable.type}-${payable.id}-overdue`,
          type: "overdue" as const,
          severity: "danger" as const,
          title: `${label} vencida`,
          message: `${payable.title} passou do vencimento.`,
          entityType: payable.type,
          entityId: payable.id,
        }];
      }

      if (days === 0) {
        return [{
          id: `${payable.type}-${payable.id}-today`,
          type: "due_today" as const,
          severity: "warning" as const,
          title: `${label} vence hoje`,
          message: `${payable.title} vence hoje.`,
          entityType: payable.type,
          entityId: payable.id,
        }];
      }

      if (daysBefore > 0 && days === daysBefore) {
        const dayWord = daysBefore === 1 ? "dia" : "dias";
        return [{
          id: `${payable.type}-${payable.id}-due-soon`,
          type: "due_in_3_days" as const,
          severity: "info" as const,
          title: `${label} vence em ${daysBefore} ${dayWord}`,
          message: `${payable.title} vence em ${daysBefore} ${dayWord}.`,
          entityType: payable.type,
          entityId: payable.id,
        }];
      }

      return [];
    });
}
