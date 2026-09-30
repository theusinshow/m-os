import { beforeEach, describe, expect, it, vi } from "vitest";

const { entriesMock, markMock, cardMock, expenseMock, monthsMock, moreMock } = vi.hoisted(() => ({
  moreMock: {
    addGoalContribution: vi.fn(),
    cancelSubscriptionEntry: vi.fn(),
    GOAL_STATUSES: ["active", "paused", "completed", "archived"],
    markIncomeReceived: vi.fn(),
    setBudgetEntry: vi.fn(),
    setGoalStatusEntry: vi.fn(),
    updateSubscriptionEntry: vi.fn(),
    upsertInvoiceAmount: vi.fn(),
  },
  entriesMock: {
    createGoalEntry: vi.fn(),
    createIncomeEntry: vi.fn(),
    createSubscriptionEntry: vi.fn(),
    setFinancialPolicy: vi.fn(),
    updateGoalEntry: vi.fn(),
  },
  markMock: { markBillPaid: vi.fn(), markInvoicePaid: vi.fn() },
  cardMock: { getCardById: vi.fn() },
  expenseMock: { createCardExpense: vi.fn() },
  monthsMock: { ensureMonthForUser: vi.fn() },
}));

vi.mock("@/lib/domain/finance-actions/entries", () => entriesMock);
vi.mock("@/lib/domain/finance-actions/mark-paid", () => markMock);
vi.mock("@/lib/domain/finance-actions/more-entries", () => moreMock);
vi.mock("@/lib/card-expenses", () => cardMock);
vi.mock("@/lib/domain/finance-actions/create-card-expense", () => expenseMock);
vi.mock("@/lib/months", () => monthsMock);
vi.mock("@/lib/mos/action-bridge", () => ({ createBillFromMosAction: vi.fn() }));

const { executeMosAction, isMosActionId, MOS_ACTION_IDS } = await import("./action-catalog");

const USER = "user-1";
const CARD = "11111111-1111-4111-8111-111111111111";
const BILL = "22222222-2222-4222-8222-222222222222";
const GOAL = "33333333-3333-4333-8333-333333333333";
const SETEMBRO = { id: "month-sep", month: 9, year: 2026 };

beforeEach(() => {
  for (const mock of [
    ...Object.values(entriesMock),
    ...Object.values(markMock),
    ...Object.values(cardMock),
    ...Object.values(expenseMock),
    ...Object.values(moreMock).filter((value) => typeof value === "function"),
  ] as { mockReset: () => void }[]) {
    mock.mockReset();
  }
  monthsMock.ensureMonthForUser.mockReset().mockResolvedValue(SETEMBRO);
  cardMock.getCardById.mockResolvedValue({ id: CARD, name: "Nubank", dueDay: 15 });
});

describe("catálogo", () => {
  it("dezesseis ações explícitas, nenhuma genérica", () => {
    expect(MOS_ACTION_IDS).toEqual([
      "m-finance.create_bill",
      "m-finance.create_card_expense",
      "m-finance.create_income",
      "m-finance.mark_bill_paid",
      "m-finance.mark_invoice_paid",
      "m-finance.create_subscription",
      "m-finance.create_goal",
      "m-finance.update_goal",
      "m-finance.set_policy",
      "m-finance.update_subscription",
      "m-finance.cancel_subscription",
      "m-finance.mark_income_received",
      "m-finance.add_goal_contribution",
      "m-finance.set_goal_status",
      "m-finance.set_invoice_amount",
      "m-finance.set_budget",
    ]);
    expect(isMosActionId("m-finance.execute")).toBe(false);
    expect(isMosActionId("constructor")).toBe(false);
  });
});

describe("m-finance.create_card_expense", () => {
  const args = { cardId: CARD, cardName: "Nubank", amountCents: 17000, description: "Gasolina" };

  it("usa o serviço de domínio no mês atual", async () => {
    expenseMock.createCardExpense.mockResolvedValue({
      ok: true,
      value: { expenseIds: ["exp-1"], installmentId: null, installments: 1, months: [SETEMBRO] },
    });

    const result = await executeMosAction("m-finance.create_card_expense", USER, args);

    expect(result).toMatchObject({
      ok: true,
      // Intl usa espaço não separável depois do R$.
      receipt: { message: expect.stringMatching(/^Compra "Gasolina" de R\$\s170,00 lançada no Nubank\.$/) },
    });
    expect(expenseMock.createCardExpense).toHaveBeenCalledWith(
      expect.objectContaining({ userId: USER, month: SETEMBRO, installments: 1, purchaseDate: null }),
    );
  });

  it("cartão com outro nome que o do preview é recusado", async () => {
    cardMock.getCardById.mockResolvedValue({ id: CARD, name: "Inter", dueDay: 10 });
    const result = await executeMosAction("m-finance.create_card_expense", USER, args);
    expect(result).toMatchObject({ ok: false, code: "stale_preview" });
    expect(expenseMock.createCardExpense).not.toHaveBeenCalled();
  });

  it("parcelas demais para o valor são recusadas no schema", async () => {
    const result = await executeMosAction("m-finance.create_card_expense", USER, {
      ...args,
      amountCents: 5,
      installments: 10,
    });
    expect(result).toMatchObject({ ok: false, code: "invalid" });
  });
});

describe("m-finance.mark_bill_paid", () => {
  it("passa o valor do preview como expectativa", async () => {
    markMock.markBillPaid.mockResolvedValue({
      ok: true,
      value: { id: BILL, name: "Internet", amountCents: 12000, dueDate: "2026-09-20" },
    });
    const result = await executeMosAction("m-finance.mark_bill_paid", USER, {
      billId: BILL,
      billName: "Internet",
      amountCents: 12000,
    });
    expect(result.ok).toBe(true);
    expect(markMock.markBillPaid).toHaveBeenCalledWith(USER, BILL, { amountCents: 12000 });
  });

  it("recusa do domínio vira recusa com código", async () => {
    markMock.markBillPaid.mockResolvedValue({ ok: false, code: "already_paid", message: "já paga" });
    const result = await executeMosAction("m-finance.mark_bill_paid", USER, {
      billId: BILL,
      billName: "Internet",
      amountCents: 12000,
    });
    expect(result).toEqual({ ok: false, error: "já paga", code: "already_paid" });
  });
});

describe("m-finance.mark_invoice_paid", () => {
  it("resolve o mês pedido e paga só a fatura lançada", async () => {
    monthsMock.ensureMonthForUser.mockResolvedValue({ id: "month-oct", month: 10, year: 2026 });
    markMock.markInvoicePaid.mockResolvedValue({
      ok: true,
      value: { id: "inv", cardId: CARD, amountCents: 150000, dueDate: "2026-10-15" },
    });
    await executeMosAction("m-finance.mark_invoice_paid", USER, { cardId: CARD, cardName: "Nubank", month: "2026-10" });
    expect(monthsMock.ensureMonthForUser).toHaveBeenCalledWith(USER, 10, 2026);
    expect(markMock.markInvoicePaid).toHaveBeenCalledWith(USER, { cardId: CARD, monthId: "month-oct" }, { amountCents: null });
  });
});

describe("m-finance.create_income", () => {
  it("lança no mês pedido", async () => {
    entriesMock.createIncomeEntry.mockResolvedValue({ ok: true, value: { id: "inc" } });
    monthsMock.ensureMonthForUser.mockResolvedValue({ id: "month-oct", month: 10, year: 2026 });
    const result = await executeMosAction("m-finance.create_income", USER, {
      name: "NF outubro",
      amountCents: 500000,
      incomeType: "main",
      month: "2026-10",
    });
    expect(result).toMatchObject({ ok: true, receipt: { message: expect.stringContaining("10/2026") } });
  });

  it("tipo de receita desconhecido é recusado", async () => {
    const result = await executeMosAction("m-finance.create_income", USER, {
      name: "x",
      amountCents: 1,
      incomeType: "salario",
    });
    expect(result).toMatchObject({ ok: false, code: "invalid" });
  });
});

describe("m-finance.update_goal", () => {
  it("edição vazia é recusada", async () => {
    const result = await executeMosAction("m-finance.update_goal", USER, { goalId: GOAL, goalName: "Mac" });
    expect(result).toMatchObject({ ok: false, code: "invalid" });
  });

  it("confere o nome do preview", async () => {
    entriesMock.updateGoalEntry.mockResolvedValue({ ok: true, value: { id: GOAL, name: "Mac", status: "active" } });
    await executeMosAction("m-finance.update_goal", USER, { goalId: GOAL, goalName: "Mac", deadline: "2027-06-30" });
    expect(entriesMock.updateGoalEntry).toHaveBeenCalledWith(
      USER,
      GOAL,
      expect.objectContaining({ deadline: "2027-06-30" }),
      { name: "Mac" },
    );
  });
});

describe("m-finance.set_policy", () => {
  it("chave fora do catálogo é recusada", async () => {
    const result = await executeMosAction("m-finance.set_policy", USER, { key: "free_text", value: "x" });
    expect(result).toMatchObject({ ok: false, code: "invalid" });
    expect(entriesMock.setFinancialPolicy).not.toHaveBeenCalled();
  });

  it("grava com origem hermes", async () => {
    entriesMock.setFinancialPolicy.mockResolvedValue({ ok: true, value: {} });
    const value = { main: 1, freelance: 0, extra: 0 };
    await executeMosAction("m-finance.set_policy", USER, { key: "reliable_income_rules", value });
    expect(entriesMock.setFinancialPolicy).toHaveBeenCalledWith(USER, "reliable_income_rules", value, "hermes");
  });
});

describe("m-finance.create_subscription", () => {
  it("teste grátis", async () => {
    entriesMock.createSubscriptionEntry.mockResolvedValue({ ok: true, value: { id: "sub" } });
    const result = await executeMosAction("m-finance.create_subscription", USER, {
      name: "Notion",
      amountCents: 4000,
      nextChargeDate: "2026-10-01",
      isTrial: true,
    });
    expect(result).toMatchObject({ ok: true, receipt: { message: expect.stringContaining("Teste grátis") } });
    expect(entriesMock.createSubscriptionEntry).toHaveBeenCalledWith(
      expect.objectContaining({ cycle: "monthly", reminderDaysBefore: 1, isTrial: true }),
    );
  });
});

const SUB = "44444444-4444-4444-8444-444444444444";
const INCOME = "55555555-5555-4555-8555-555555555555";

describe("ações que completam a tela", () => {
  it("assinatura: edição vazia é recusada; cancelar confere o nome", async () => {
    expect(
      await executeMosAction("m-finance.update_subscription", USER, { subscriptionId: SUB, subscriptionName: "Claude" }),
    ).toMatchObject({ ok: false, code: "invalid" });

    moreMock.cancelSubscriptionEntry.mockResolvedValue({ ok: true, value: { id: SUB, name: "Claude Max" } });
    await executeMosAction("m-finance.cancel_subscription", USER, { subscriptionId: SUB, subscriptionName: "Claude Max" });
    expect(moreMock.cancelSubscriptionEntry).toHaveBeenCalledWith(USER, SUB, { name: "Claude Max" });
  });

  it("receita recebida leva nome e valor do preview", async () => {
    moreMock.markIncomeReceived.mockResolvedValue({ ok: true, value: { id: INCOME, name: "NF", amountCents: 500000 } });
    await executeMosAction("m-finance.mark_income_received", USER, { incomeId: INCOME, incomeName: "NF", amountCents: 500000 });
    expect(moreMock.markIncomeReceived).toHaveBeenCalledWith(USER, INCOME, { amountCents: 500000, name: "NF" });
  });

  it("contribuição sem data usa hoje em São Paulo", async () => {
    moreMock.addGoalContribution.mockResolvedValue({
      ok: true,
      value: { id: GOAL, name: "Mac", status: "active", currentAmountCents: 400000 },
    });
    await executeMosAction("m-finance.add_goal_contribution", USER, { goalId: GOAL, goalName: "Mac", amountCents: 100000 });
    expect(moreMock.addGoalContribution).toHaveBeenCalledWith(
      USER,
      GOAL,
      100000,
      expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/),
      { name: "Mac" },
    );
  });

  it("status de meta fora da lista é recusado", async () => {
    expect(
      await executeMosAction("m-finance.set_goal_status", USER, { goalId: GOAL, goalName: "Mac", status: "deleted" }),
    ).toMatchObject({ ok: false, code: "invalid" });
  });

  it("valor da fatura no mês pedido, conferindo o cartão", async () => {
    monthsMock.ensureMonthForUser.mockResolvedValue({ id: "month-oct", month: 10, year: 2026 });
    moreMock.upsertInvoiceAmount.mockResolvedValue({
      ok: true,
      value: { cardName: "Nubank", amountCents: 150000, dueDate: "2026-10-15", created: true },
    });
    const result = await executeMosAction("m-finance.set_invoice_amount", USER, {
      cardId: CARD,
      cardName: "Nubank",
      amountCents: 150000,
      month: "2026-10",
    });
    expect(result).toMatchObject({ ok: true, receipt: { message: expect.stringContaining("lançada") } });
    expect(moreMock.upsertInvoiceAmount).toHaveBeenCalledWith(
      expect.objectContaining({ month: { id: "month-oct", month: 10, year: 2026 }, expectation: { cardName: "Nubank" } }),
    );
  });

  it("orçamento por categoria exige o nome; por cartão, o id", async () => {
    expect(await executeMosAction("m-finance.set_budget", USER, { budgetType: "category", limitCents: 1000 })).toMatchObject({
      ok: false,
      code: "invalid",
    });
    expect(await executeMosAction("m-finance.set_budget", USER, { budgetType: "card", limitCents: 1000 })).toMatchObject({
      ok: false,
      code: "invalid",
    });
    moreMock.setBudgetEntry.mockResolvedValue({ ok: true, value: { id: "b", label: "Mercado", created: false } });
    const result = await executeMosAction("m-finance.set_budget", USER, {
      budgetType: "category",
      limitCents: 80000,
      categoryName: "Mercado",
    });
    expect(result).toMatchObject({ ok: true, receipt: { message: expect.stringContaining("ajustado") } });
  });
});
