/**
 * O snapshot financeiro: tudo que o kernel precisa, lido do banco de uma vez.
 *
 * O kernel inteiro é função pura sobre este tipo. É o que permite testar o
 * Safe-to-Spend, os cenários e os detectores sem banco — e o que garante que
 * duas ferramentas chamadas no mesmo pedido falam do mesmo instante.
 *
 * Status de conta e fatura chega CRU (como está gravado). "Vencido" depende do
 * dia de hoje, e o dia de hoje é `today` — derivado no kernel, e não no
 * loader, para um teste poder dizer "hoje é 29/09" sem mexer no relógio.
 */
import type { ResolvedPolicies } from "@/lib/finance-intelligence/policies";

export type MonthParts = { month: number; year: number };
export type PayableStatus = "pending" | "paid" | "overdue";
export type IncomeType = "main" | "extra" | "freelance";

export type SnapshotIncome = MonthParts & {
  id: string;
  name: string;
  amountCents: number;
  incomeType: IncomeType;
  expectedDate: string | null;
  received: boolean;
};

export type SnapshotBill = MonthParts & {
  id: string;
  name: string;
  amountCents: number;
  dueDate: string;
  status: PayableStatus;
  isRecurring: boolean;
  recurrenceRuleId: string | null;
  seriesId: string | null;
  seriesNumber: number | null;
  seriesTotal: number | null;
  categoryName: string | null;
};

export type SnapshotCard = {
  id: string;
  name: string;
  cardType: "personal" | "business";
  dueDay: number;
  isActive: boolean;
};

export type SnapshotInvoice = MonthParts & {
  id: string;
  cardId: string;
  amountCents: number;
  dueDate: string;
  status: PayableStatus;
};

export type SnapshotCardExpense = MonthParts & {
  id: string;
  cardId: string;
  description: string;
  amountCents: number;
  purchaseDate: string | null;
  installmentId: string | null;
  installmentNumber: number | null;
  installmentTotal: number | null;
};

export type SnapshotSubscription = {
  id: string;
  name: string;
  amountCents: number;
  nextChargeDate: string;
  cycle: "once" | "monthly" | "yearly";
  status: "trial" | "active" | "canceled";
};

export type SnapshotGoal = {
  id: string;
  name: string;
  targetAmountCents: number;
  currentAmountCents: number;
  deadline: string | null;
  priority: "low" | "medium" | "high";
  status: "active" | "paused" | "completed" | "archived";
};

export type FinanceSnapshot = {
  /** `yyyy-mm-dd`, no fuso de quem usa. */
  today: string;
  /** O mês do calendário de `today`. */
  current: MonthParts;
  incomes: SnapshotIncome[];
  bills: SnapshotBill[];
  cards: SnapshotCard[];
  invoices: SnapshotInvoice[];
  cardExpenses: SnapshotCardExpense[];
  subscriptions: SnapshotSubscription[];
  goals: SnapshotGoal[];
  policies: ResolvedPolicies;
};
