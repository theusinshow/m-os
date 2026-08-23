export type PayableStatus = "pending" | "paid" | "overdue";

/**
 * O denominador comum entre uma conta e uma fatura de cartão: as duas coisas
 * que o dono paga no mesmo dia, na mesma sessão, com o mesmo gesto. A lista
 * de pagamentos trabalha só com isto; cada tela mantém os campos extras do
 * registro original, que o genérico `T` preserva.
 */
export type Payable = {
  id: string;
  type: "bill" | "invoice";
  name: string;
  amountCents: number;
  dueDate: string;
  status: PayableStatus;
};

export type PayableGroupKey = "overdue" | "today" | "week" | "later";

export type PayableGroup<T extends Payable> = {
  key: PayableGroupKey;
  title: string;
  description: string;
  items: T[];
};

const GROUPS: { key: PayableGroupKey; title: string; description: string }[] = [
  { key: "overdue", title: "Vencidas", description: "Resolva antes de olhar o restante." },
  { key: "today", title: "Hoje", description: "Ação principal do dia." },
  { key: "week", title: "Próximos 7 dias", description: "Prepare o pagamento agora." },
  { key: "later", title: "Depois", description: "Contas futuras do mês." },
];

function toDateOnly(date: Date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function diffInDays(dueDate: string, today: Date) {
  const due = new Date(`${dueDate}T12:00:00`);
  return Math.round((toDateOnly(due).getTime() - toDateOnly(today).getTime()) / 86_400_000);
}

/*
 * O grupo sai da DATA, nunca do status. Se o status entrasse na conta, marcar
 * pago atiraria a linha para outro grupo debaixo do dedo — exatamente o
 * contrário do que o gesto promete. Pago fica onde está, só desce dentro do
 * próprio grupo para as pendentes subirem.
 */
function groupKeyFor(dueDate: string, today: Date): PayableGroupKey {
  const days = diffInDays(dueDate, today);
  if (days < 0) return "overdue";
  if (days === 0) return "today";
  if (days <= 7) return "week";
  return "later";
}

export function groupPayables<T extends Payable>(
  items: T[],
  today = new Date(),
): PayableGroup<T>[] {
  const buckets = new Map<PayableGroupKey, T[]>(GROUPS.map((group) => [group.key, []]));

  for (const item of items) {
    buckets.get(groupKeyFor(item.dueDate, today))!.push(item);
  }

  return GROUPS.map((group) => ({
    ...group,
    items: [...buckets.get(group.key)!].sort((a, b) => {
      const aPaid = a.status === "paid" ? 1 : 0;
      const bPaid = b.status === "paid" ? 1 : 0;
      if (aPaid !== bPaid) return aPaid - bPaid;
      return a.dueDate.localeCompare(b.dueDate) || a.name.localeCompare(b.name);
    }),
  })).filter((group) => group.items.length > 0);
}

export function payableProgress(items: Payable[]) {
  return items.reduce(
    (progress, item) => {
      const paid = item.status === "paid";
      return {
        totalCount: progress.totalCount + 1,
        paidCount: progress.paidCount + (paid ? 1 : 0),
        remainingCount: progress.remainingCount + (paid ? 0 : 1),
        totalCents: progress.totalCents + item.amountCents,
        paidCents: progress.paidCents + (paid ? item.amountCents : 0),
        remainingCents: progress.remainingCents + (paid ? 0 : item.amountCents),
      };
    },
    {
      totalCount: 0,
      paidCount: 0,
      remainingCount: 0,
      totalCents: 0,
      paidCents: 0,
      remainingCents: 0,
    },
  );
}
