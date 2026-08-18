import { beforeEach, describe, expect, it, vi } from "vitest";

type InsertCall = { table: unknown; values: unknown };

const { dbState, monthsMock } = vi.hoisted(() => ({
  dbState: {
    available: true,
    inserts: [] as InsertCall[],
    /** Uma entrada por chamada de `.returning()`, na ordem em que acontecem. */
    returningQueue: [] as unknown[][],
  },
  monthsMock: {
    getCurrentMonthForUser: vi.fn(),
    ensureConsecutiveMonthsForUser: vi.fn(),
  },
}));

const fakeDb = {
  insert(table: unknown) {
    return {
      values(values: unknown) {
        dbState.inserts.push({ table, values });
        return {
          returning: () => Promise.resolve(dbState.returningQueue.shift() ?? []),
        };
      },
    };
  },
};

vi.mock("@/db/client", () => ({
  // Getter para que o teste do banco indisponivel nao precise remontar o modulo.
  get db() {
    return dbState.available ? fakeDb : null;
  },
}));
vi.mock("@/lib/months", () => monthsMock);

const { createBillFromMosAction } = await import("./action-bridge");
const { bills, recurrenceRules } = await import("@/db/schema");

const USER = "user-1";
const MARCO = { id: "month-mar", month: 3, year: 2026 };

const validArgs = {
  amountCents: 12990,
  description: "Internet",
  dueDay: 10,
  isRecurring: false,
};

/** Os `values` do insert de uma tabela, achatados para uma lista de linhas. */
function rowsInsertedInto(table: unknown) {
  const call = dbState.inserts.find((insert) => insert.table === table);
  if (!call) return [];
  return Array.isArray(call.values) ? call.values : [call.values];
}

beforeEach(() => {
  dbState.available = true;
  dbState.inserts = [];
  dbState.returningQueue = [];
  monthsMock.getCurrentMonthForUser.mockReset().mockResolvedValue(MARCO);
  monthsMock.ensureConsecutiveMonthsForUser.mockReset().mockResolvedValue([MARCO]);
});

describe("pre-condicoes", () => {
  it("recusa quando o banco esta indisponivel", async () => {
    dbState.available = false;

    await expect(createBillFromMosAction(USER, validArgs)).resolves.toEqual({
      ok: false,
      error: "Banco de dados indisponível no momento.",
    });
  });

  it("recusa quando o usuario ainda nao tem o mes atual criado", async () => {
    monthsMock.getCurrentMonthForUser.mockResolvedValue(null);

    const result = await createBillFromMosAction(USER, validArgs);

    expect(result).toEqual({
      ok: false,
      error: "Crie o mês atual no app antes de lançar despesas por aqui.",
    });
    expect(dbState.inserts).toHaveLength(0);
  });
});

describe("validacao dos argumentos", () => {
  const recusados: Array<[string, unknown]> = [
    ["valor negativo", { ...validArgs, amountCents: -1 }],
    ["valor zero", { ...validArgs, amountCents: 0 }],
    ["valor fracionado", { ...validArgs, amountCents: 10.5 }],
    ["descricao vazia", { ...validArgs, description: "   " }],
    ["dia 0", { ...validArgs, dueDay: 0 }],
    ["dia 32", { ...validArgs, dueDay: 32 }],
    ["isRecurring ausente", { amountCents: 100, description: "X", dueDay: 1 }],
    ["objeto vazio", {}],
    ["nao e objeto", "conta de luz"],
    ["nulo", null],
  ];

  it.each(recusados)("recusa %s", async (_nome, args) => {
    const result = await createBillFromMosAction(USER, args);

    expect(result).toEqual({
      ok: false,
      error: "Os argumentos da ação não batem com o esperado.",
    });
    expect(dbState.inserts).toHaveLength(0);
  });

  it("aceita a descricao com espacos nas pontas, gravando ja aparada", async () => {
    dbState.returningQueue = [[{ id: "bill-1" }]];

    await createBillFromMosAction(USER, { ...validArgs, description: "  Internet  " });

    expect(rowsInsertedInto(bills)[0]).toMatchObject({ name: "Internet" });
  });
});

describe("conta avulsa", () => {
  it("grava a conta no mes atual com a data composta a partir do dia", async () => {
    dbState.returningQueue = [[{ id: "bill-1" }]];

    const result = await createBillFromMosAction(USER, validArgs);

    expect(result).toEqual({ ok: true, billId: "bill-1" });
    expect(rowsInsertedInto(bills)).toEqual([
      {
        userId: USER,
        monthId: MARCO.id,
        name: "Internet",
        amountCents: 12990,
        dueDate: "2026-03-10",
        isRecurring: false,
        status: "pending",
      },
    ]);
    expect(rowsInsertedInto(recurrenceRules)).toHaveLength(0);
  });

  it("sem dia informado, vence no ultimo dia do mes", async () => {
    monthsMock.getCurrentMonthForUser.mockResolvedValue({ id: "month-fev", month: 2, year: 2026 });
    dbState.returningQueue = [[{ id: "bill-1" }]];

    await createBillFromMosAction(USER, { ...validArgs, dueDay: null });

    // 2026 nao e bissexto: o dia 31 default e limitado a 28 por composeMonthDate.
    expect(rowsInsertedInto(bills)[0]).toMatchObject({ dueDate: "2026-02-28" });
  });

  it("recusa quando o insert nao devolve a linha criada", async () => {
    dbState.returningQueue = [[]];

    await expect(createBillFromMosAction(USER, validArgs)).resolves.toEqual({
      ok: false,
      error: "Não consegui gravar a conta agora.",
    });
  });
});

describe("conta recorrente", () => {
  const recorrente = { ...validArgs, isRecurring: true };

  it("cria a regra e pre-gera as contas dos proximos 12 meses", async () => {
    const meses = Array.from({ length: 12 }, (_, index) => ({
      id: `month-${index}`,
      month: ((MARCO.month - 1 + index) % 12) + 1,
      year: MARCO.year + Math.floor((MARCO.month - 1 + index) / 12),
    }));
    monthsMock.ensureConsecutiveMonthsForUser.mockResolvedValue(meses);
    dbState.returningQueue = [
      [{ id: "rule-1" }],
      meses.map((_, index) => ({ id: `bill-${index}` })),
    ];

    const result = await createBillFromMosAction(USER, recorrente);

    expect(result).toEqual({ ok: true, billId: "bill-0" });
    expect(monthsMock.ensureConsecutiveMonthsForUser).toHaveBeenCalledWith(USER, 3, 2026, 12);
    expect(rowsInsertedInto(recurrenceRules)).toEqual([
      {
        userId: USER,
        name: "Internet",
        defaultAmountCents: 12990,
        dueDay: 10,
        isVariableAmount: false,
        isActive: true,
      },
    ]);

    const contas = rowsInsertedInto(bills);
    expect(contas).toHaveLength(12);
    expect(contas.every((conta) => (conta as { recurrenceRuleId: string }).recurrenceRuleId === "rule-1")).toBe(true);
    expect(contas.map((conta) => (conta as { dueDate: string }).dueDate)).toEqual([
      "2026-03-10", "2026-04-10", "2026-05-10", "2026-06-10",
      "2026-07-10", "2026-08-10", "2026-09-10", "2026-10-10",
      "2026-11-10", "2026-12-10", "2027-01-10", "2027-02-10",
    ]);
  });

  it("recusa quando a regra de recorrencia nao e criada", async () => {
    dbState.returningQueue = [[]];

    await expect(createBillFromMosAction(USER, recorrente)).resolves.toEqual({
      ok: false,
      error: "Não consegui criar a regra de recorrência agora.",
    });
    expect(rowsInsertedInto(bills)).toHaveLength(0);
  });

  /**
   * Recorrente sem dia de vencimento nao tem como virar regra — nao ha em que
   * dia repetir. Cai no ramo avulso, mas preservando `isRecurring: true`.
   */
  it("sem dia de vencimento, grava uma conta unica ainda marcada como recorrente", async () => {
    dbState.returningQueue = [[{ id: "bill-1" }]];

    const result = await createBillFromMosAction(USER, { ...recorrente, dueDay: null });

    expect(result).toEqual({ ok: true, billId: "bill-1" });
    expect(rowsInsertedInto(recurrenceRules)).toHaveLength(0);
    expect(rowsInsertedInto(bills)[0]).toMatchObject({ isRecurring: true, dueDate: "2026-03-31" });
  });
});
