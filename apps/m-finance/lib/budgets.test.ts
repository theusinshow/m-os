import { beforeEach, describe, expect, it, vi } from "vitest";

type SelectCall = { projection: Record<string, unknown> };

const { dbState } = vi.hoisted(() => ({
  dbState: {
    selects: [] as SelectCall[],
    /** Uma entrada por consulta resolvida, na ordem em que acontecem. */
    resultQueue: [] as unknown[][],
  },
}));

/**
 * Encadeamento mínimo do drizzle usado por `budgets.ts`:
 * `.select().from().where()`, `.select().from().where().orderBy()` e
 * `.select().from().where().limit()`. O elo final é "thenable" para que o
 * `await` resolva a fila.
 */
const fakeDb = {
  select(projection: Record<string, unknown>) {
    dbState.selects.push({ projection });
    const resolve = () => Promise.resolve(dbState.resultQueue.shift() ?? []);
    const tail = {
      limit: resolve,
      orderBy: resolve,
      then: (onFulfilled: (rows: unknown[]) => unknown) => resolve().then(onFulfilled),
    };
    return { from: () => ({ where: () => tail }) };
  },
};

vi.mock("@/db/client", () => ({
  get db() {
    return fakeDb;
  },
}));

const { getBudgetsByMonth } = await import("./budgets");

/** Uma expressão SQL do drizzle carrega `queryChunks`; uma coluna crua, não. */
function isSqlExpression(value: unknown) {
  return typeof value === "object" && value !== null && "queryChunks" in value;
}

beforeEach(() => {
  dbState.selects = [];
  dbState.resultQueue = [];
});

describe("getSpentForBudget, por getBudgetsByMonth", () => {
  it("pede um agregado ao banco em vez da primeira linha", async () => {
    // 1: as linhas de budgets. 2: a soma das contas. 3: o rótulo da categoria.
    dbState.resultQueue = [
      [
        {
          id: "budget-1",
          budgetType: "category",
          categoryId: "cat-1",
          cardId: null,
          limitCents: 100_000,
        },
      ],
      [{ total: 75_000 }],
      [{ name: "Mercado" }],
    ];

    const budgets = await getBudgetsByMonth("month-1", "user-1");

    // A consulta do gasto é a segunda: a primeira busca os budgets.
    const spentProjection = dbState.selects[1].projection;
    expect(isSqlExpression(spentProjection.total)).toBe(true);

    expect(budgets[0].spentCents).toBe(75_000);
    expect(budgets[0].remainingCents).toBe(25_000);
    expect(budgets[0].percentage).toBe(75);
    expect(budgets[0].isOverBudget).toBe(false);
    expect(budgets[0].isWarning).toBe(false);
  });

  it("marca estouro acima do limite", async () => {
    dbState.resultQueue = [
      [
        {
          id: "budget-2",
          budgetType: "category",
          categoryId: "cat-1",
          cardId: null,
          limitCents: 100_000,
        },
      ],
      [{ total: 120_000 }],
      [{ name: "Mercado" }],
    ];

    const [budget] = await getBudgetsByMonth("month-1", "user-1");

    expect(budget.isOverBudget).toBe(true);
    expect(budget.isWarning).toBe(false);
    expect(budget.remainingCents).toBe(-20_000);
  });

  it("soma contas e faturas no orcamento total", async () => {
    dbState.resultQueue = [
      [
        {
          id: "budget-3",
          budgetType: "total",
          categoryId: null,
          cardId: null,
          limitCents: 500_000,
        },
      ],
      [{ total: 200_000 }], // contas
      [{ total: 150_000 }], // faturas
    ];

    const [budget] = await getBudgetsByMonth("month-1", "user-1");

    expect(budget.spentCents).toBe(350_000);
    expect(isSqlExpression(dbState.selects[1].projection.total)).toBe(true);
    expect(isSqlExpression(dbState.selects[2].projection.total)).toBe(true);
  });
});
