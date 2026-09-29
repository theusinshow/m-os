import { beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Banco falso que registra o que cada serviço escreveria. Cada chamada de
 * `select` consome uma resposta da fila; `insert`/`update` são gravados.
 */
const { state, monthsMock, syncMock } = vi.hoisted(() => ({
  state: {
    available: true,
    selects: [] as unknown[][],
    inserts: [] as { values: unknown }[],
    updates: [] as { set: unknown }[],
    returning: [] as unknown[][],
    conflicts: [] as unknown[],
  },
  monthsMock: { ensureConsecutiveMonthsForUser: vi.fn() },
  syncMock: { sumCardExpenses: vi.fn(), syncInvoiceTotal: vi.fn() },
}));

function chain(result: () => unknown) {
  const node: Record<string, unknown> = {};
  for (const method of ["from", "where", "limit", "orderBy", "innerJoin", "leftJoin"]) {
    node[method] = () => node;
  }
  node.then = (resolve: (value: unknown) => void) => resolve(result());
  return node;
}

const fakeDb = {
  select: () => chain(() => state.selects.shift() ?? []),
  insert: () => ({
    values(values: unknown) {
      state.inserts.push({ values });
      const next = {
        returning: () => Promise.resolve(state.returning.shift() ?? [{ id: "new-id" }]),
        onConflictDoUpdate: (conflict: unknown) => {
          state.conflicts.push(conflict);
          return Promise.resolve();
        },
      };
      return next;
    },
  }),
  update: () => ({
    set(set: unknown) {
      state.updates.push({ set });
      return { where: () => Promise.resolve() };
    },
  }),
  transaction: (fn: (tx: unknown) => unknown) => fn(fakeDb),
};

vi.mock("@/db/client", () => ({
  get db() {
    return state.available ? fakeDb : null;
  },
}));
vi.mock("@/lib/months", () => monthsMock);
vi.mock("@/lib/invoice-sync", () => syncMock);

const { createBillEntries, scheduleFromFlags } = await import("./create-bill");
const { createCardExpense, splitInstallments } = await import("./create-card-expense");
const { markBillPaid, markInvoicePaid } = await import("./mark-paid");
const { normalizeGoalAmounts, setFinancialPolicy, updateGoalEntry } = await import("./entries");

const USER = "user-1";
const SEP = { id: "m9", month: 9, year: 2026 };

beforeEach(() => {
  state.available = true;
  state.selects = [];
  state.inserts = [];
  state.updates = [];
  state.returning = [];
  state.conflicts = [];
  monthsMock.ensureConsecutiveMonthsForUser.mockReset();
  syncMock.sumCardExpenses.mockReset().mockResolvedValue(0);
  syncMock.syncInvoiceTotal.mockReset().mockResolvedValue(undefined);
});

describe("createBillEntries", () => {
  it("agendamento a partir das flags do WhatsApp/M/OS", () => {
    expect(scheduleFromFlags(false, 10)).toEqual({ kind: "once" });
    expect(scheduleFromFlags(true, 10)).toEqual({ kind: "ongoing" });
    expect(scheduleFromFlags(true, null)).toEqual({ kind: "flagged" });
  });

  it("série fixa: N meses com seriesId e numeração", async () => {
    monthsMock.ensureConsecutiveMonthsForUser.mockResolvedValue([
      SEP,
      { id: "m10", month: 10, year: 2026 },
      { id: "m11", month: 11, year: 2026 },
    ]);
    state.returning = [[{ id: "b1" }, { id: "b2" }, { id: "b3" }]];

    const result = await createBillEntries({
      userId: USER,
      month: SEP,
      name: "Curso",
      amountCents: 30000,
      dueDay: 31,
      schedule: { kind: "fixed", months: 3 },
    });

    expect(result).toMatchObject({ ok: true, value: { billIds: ["b1", "b2", "b3"], months: 3 } });
    const rows = state.inserts[0].values as { seriesNumber: number; seriesTotal: number; dueDate: string }[];
    expect(rows.map((row) => row.seriesNumber)).toEqual([1, 2, 3]);
    expect(rows.map((row) => row.dueDate)).toEqual(["2026-09-30", "2026-10-31", "2026-11-30"]);
    expect(new Set(rows.map((row) => (row as unknown as { seriesId: string }).seriesId)).size).toBe(1);
  });

  it("banco indisponível", async () => {
    state.available = false;
    const result = await createBillEntries({
      userId: USER,
      month: SEP,
      name: "x",
      amountCents: 1,
      dueDay: null,
      schedule: { kind: "once" },
    });
    expect(result).toMatchObject({ ok: false, code: "db_unavailable" });
  });
});

describe("createCardExpense", () => {
  it("divide sem perder centavo", () => {
    expect(splitInstallments(1000, 3)).toEqual([334, 333, 333]);
    expect(splitInstallments(1000, 3).reduce((a, b) => a + b, 0)).toBe(1000);
  });

  it("parcelado: uma linha por mês, fatura reconciliada em cada um", async () => {
    const months = [SEP, { id: "m10", month: 10, year: 2026 }];
    monthsMock.ensureConsecutiveMonthsForUser.mockResolvedValue(months);
    syncMock.sumCardExpenses.mockResolvedValueOnce(5000).mockResolvedValueOnce(0);
    state.returning = [[{ id: "e1" }, { id: "e2" }]];

    const result = await createCardExpense({
      userId: USER,
      card: { id: "card", name: "Nubank", dueDay: 15 },
      month: SEP,
      description: "Fone",
      amountCents: 1001,
      purchaseDate: null,
      installments: 2,
    });

    expect(result).toMatchObject({ ok: true, value: { expenseIds: ["e1", "e2"], installments: 2 } });
    const rows = state.inserts[0].values as { amountCents: number; installmentNumber: number }[];
    expect(rows.map((row) => row.amountCents)).toEqual([501, 500]);
    expect(rows.map((row) => row.installmentNumber)).toEqual([1, 2]);
    // A soma ANTERIOR de cada mês vai para a reconciliação.
    expect(syncMock.syncInvoiceTotal).toHaveBeenNthCalledWith(1, fakeDb, USER, "card", SEP, 15, 5000);
    expect(syncMock.syncInvoiceTotal).toHaveBeenNthCalledWith(2, fakeDb, USER, "card", months[1], 15, 0);
  });

  it("à vista não cria parcelamento nem meses", async () => {
    state.returning = [[{ id: "e1" }]];
    const result = await createCardExpense({
      userId: USER,
      card: { id: "card", name: "Nubank", dueDay: 15 },
      month: SEP,
      description: "Gasolina",
      amountCents: 17000,
      purchaseDate: "2026-09-15",
      installments: 1,
    });
    expect(result.ok).toBe(true);
    expect(monthsMock.ensureConsecutiveMonthsForUser).not.toHaveBeenCalled();
    expect((state.inserts[0].values as { installmentId: null }[])[0].installmentId).toBeNull();
  });
});

describe("markBillPaid", () => {
  const bill = { id: "b", name: "Internet", amountCents: 12000, dueDate: "2026-09-20", status: "pending" };

  it("paga a conta pelo id", async () => {
    state.selects = [[bill]];
    const result = await markBillPaid(USER, "b");
    expect(result).toMatchObject({ ok: true, value: { name: "Internet" } });
    expect(state.updates[0].set).toMatchObject({ status: "paid" });
  });

  it("valor diferente do preview não paga", async () => {
    state.selects = [[bill]];
    const result = await markBillPaid(USER, "b", { amountCents: 30000 });
    expect(result).toMatchObject({ ok: false, code: "stale_preview" });
    expect(state.updates).toHaveLength(0);
  });

  it("já paga e inexistente são respostas, não exceções", async () => {
    state.selects = [[{ ...bill, status: "paid" }]];
    expect(await markBillPaid(USER, "b")).toMatchObject({ ok: false, code: "already_paid" });
    state.selects = [[]];
    expect(await markBillPaid(USER, "b")).toMatchObject({ ok: false, code: "not_found" });
  });
});

describe("markInvoicePaid", () => {
  it("sem fatura lançada não paga estimativa", async () => {
    state.selects = [[]];
    const result = await markInvoicePaid(USER, { cardId: "card", monthId: "m10" });
    expect(result).toMatchObject({ ok: false, code: "not_found" });
  });
});

describe("metas", () => {
  it("valor guardado limitado ao alvo; conclui quando chega", () => {
    expect(normalizeGoalAmounts({ targetAmountCents: 1000, currentAmountCents: 1500 })).toEqual({
      reached: true,
      currentAmountCents: 1000,
    });
  });

  it("editar não despausa", async () => {
    state.selects = [
      [
        {
          id: "g",
          name: "Mac",
          targetAmountCents: 900000,
          currentAmountCents: 100000,
          deadline: null,
          priority: "high",
          status: "paused",
          notes: null,
        },
      ],
    ];
    const result = await updateGoalEntry(USER, "g", { currentAmountCents: 950000 });
    expect(result).toMatchObject({ ok: true, value: { status: "paused" } });
    expect(state.updates[0].set).toMatchObject({ currentAmountCents: 900000, status: "paused" });
  });

  it("nome diferente do preview recusa", async () => {
    state.selects = [[{ id: "g", name: "Viagem", status: "active" }]];
    const result = await updateGoalEntry(USER, "g", { priority: "low" }, { name: "Mac" });
    expect(result).toMatchObject({ ok: false, code: "stale_preview" });
  });
});

describe("setFinancialPolicy", () => {
  it("valida o valor pela chave antes de gravar", async () => {
    const bad = await setFinancialPolicy(USER, "reliable_income_rules", { main: 2, freelance: 0, extra: 0 }, "hermes");
    expect(bad).toMatchObject({ ok: false, code: "invalid" });
    expect(state.inserts).toHaveLength(0);

    const good = await setFinancialPolicy(USER, "minimum_month_end_buffer", { amountCents: 80000 }, "hermes");
    expect(good).toMatchObject({ ok: true });
    expect(state.inserts[0].values).toMatchObject({
      key: "minimum_month_end_buffer",
      value: { amountCents: 80000 },
      source: "hermes",
    });
    expect(state.conflicts).toHaveLength(1);
  });
});
