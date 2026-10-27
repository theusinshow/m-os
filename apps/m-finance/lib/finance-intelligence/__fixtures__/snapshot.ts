import { DEFAULT_POLICIES, type ResolvedPolicies } from "@/lib/finance-intelligence/policies";
import type {
  FinanceSnapshot,
  SnapshotBill,
  SnapshotCardExpense,
  SnapshotIncome,
  SnapshotInvoice,
} from "@/lib/finance-intelligence/types";

/**
 * Um mês de verdade, pequeno o bastante para fazer a conta de cabeça.
 *
 * Hoje: 15/09/2026.
 *
 * Setembro: NF 5.000 + freelance 1.000 (nenhuma recebida); aluguel 1.500
 * (pago), internet 120 (vence 20/09), luz 180 (venceu 05/09); fatura Nubank
 * 1.500 lançada, Inter sem fatura (estimada em 300 pela média).
 *
 * Outubro: aluguel e internet; nenhuma NF nem fatura — tudo estimado.
 * Parcelamento Nubank de 3× R$ 200: set, out, nov.
 */
export function baseSnapshot(overrides: Partial<FinanceSnapshot> = {}): FinanceSnapshot {
  const incomes: SnapshotIncome[] = [
    income("inc-jun", 6, 480000, "main", true),
    income("inc-jul", 7, 500000, "main", true),
    income("inc-aug", 8, 520000, "main", true),
    income("inc-nf-sep", 9, 500000, "main", false),
    income("inc-free-sep", 9, 100000, "freelance", false),
  ];

  const bills: SnapshotBill[] = [
    bill("bill-rent-sep", "Aluguel", 9, 150000, "2026-09-10", "paid"),
    bill("bill-net-sep", "Internet", 9, 12000, "2026-09-20", "pending"),
    bill("bill-light-sep", "Conta de luz", 9, 18000, "2026-09-05", "pending"),
    bill("bill-rent-oct", "Aluguel", 10, 150000, "2026-10-10", "pending"),
    bill("bill-net-oct", "Internet", 10, 12000, "2026-10-20", "pending"),
  ];

  const invoices: SnapshotInvoice[] = [
    invoice("inv-nu-jun", "card-nu", 6, 100000, "2026-06-15", "paid"),
    invoice("inv-nu-jul", "card-nu", 7, 120000, "2026-07-15", "paid"),
    invoice("inv-nu-aug", "card-nu", 8, 140000, "2026-08-15", "paid"),
    invoice("inv-nu-sep", "card-nu", 9, 150000, "2026-09-15", "pending"),
    invoice("inv-inter-aug", "card-inter", 8, 30000, "2026-08-20", "paid"),
  ];

  const cardExpenses: SnapshotCardExpense[] = [9, 10, 11].map((month, index) => ({
    id: `exp-tv-${month}`,
    cardId: "card-nu",
    description: "Televisão",
    amountCents: 20000,
    purchaseDate: "2026-09-02",
    installmentId: "inst-tv",
    installmentNumber: index + 1,
    installmentTotal: 3,
    month,
    year: 2026,
  }));

  return {
    today: "2026-09-15",
    current: { month: 9, year: 2026 },
    incomes,
    bills,
    cards: [
      { id: "card-nu", name: "Nubank", cardType: "personal", dueDay: 15, isActive: true },
      { id: "card-inter", name: "Inter PJ", cardType: "business", dueDay: 20, isActive: true },
    ],
    invoices,
    cardExpenses,
    subscriptions: [
      {
        id: "sub-claude",
        name: "Claude Max",
        amountCents: 50000,
        nextChargeDate: "2026-09-25",
        cycle: "monthly",
        status: "active",
      },
      {
        id: "sub-icloud",
        name: "iCloud",
        amountCents: 12000,
        nextChargeDate: "2027-01-10",
        cycle: "yearly",
        status: "active",
      },
      {
        id: "sub-trial",
        name: "Notion",
        amountCents: 4000,
        nextChargeDate: "2026-09-18",
        cycle: "monthly",
        status: "trial",
      },
    ],
    goals: [
      {
        id: "goal-mac",
        name: "MacBook",
        targetAmountCents: 900000,
        currentAmountCents: 300000,
        deadline: "2026-12-31",
        priority: "high",
        status: "active",
      },
    ],
    policies: withPolicies(),
    ...overrides,
  };
}

export function withPolicies(overrides: Partial<ResolvedPolicies> = {}): ResolvedPolicies {
  return {
    ...DEFAULT_POLICIES,
    reliableIncomeRules: { ...DEFAULT_POLICIES.reliableIncomeRules },
    safeToSpend: { ...DEFAULT_POLICIES.safeToSpend },
    configured: [],
    ...overrides,
  };
}

function income(
  id: string,
  month: number,
  amountCents: number,
  incomeType: SnapshotIncome["incomeType"],
  received: boolean,
): SnapshotIncome {
  return { id, name: incomeType === "main" ? "NF" : "Freela", amountCents, incomeType, expectedDate: null, received, month, year: 2026 };
}

function bill(
  id: string,
  name: string,
  month: number,
  amountCents: number,
  dueDate: string,
  status: SnapshotBill["status"],
): SnapshotBill {
  return {
    id,
    name,
    amountCents,
    dueDate,
    status,
    isRecurring: false,
    recurrenceRuleId: null,
    seriesId: null,
    seriesNumber: null,
    seriesTotal: null,
    categoryName: null,
    month,
    year: 2026,
  };
}

function invoice(
  id: string,
  cardId: string,
  month: number,
  amountCents: number,
  dueDate: string,
  status: SnapshotInvoice["status"],
): SnapshotInvoice {
  return { id, cardId, amountCents, dueDate, status, month, year: 2026 };
}
