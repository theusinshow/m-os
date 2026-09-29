import { beforeEach, describe, expect, it, vi } from "vitest";
import { baseSnapshot } from "@/lib/finance-intelligence/__fixtures__/snapshot";

const { envMock, authMock, loadMock, runMock } = vi.hoisted(() => ({
  envMock: {
    mosActionSecret: "",
    mosFinanceReadSecret: "",
    financeAiNarrateInsights: false,
  } as Record<string, unknown>,
  authMock: { getWhatsappOwnerUser: vi.fn() },
  loadMock: { loadFinanceSnapshot: vi.fn() },
  runMock: { getPreviousSafeToSpend: vi.fn(), getStoredInsights: vi.fn() },
}));

vi.mock("@/lib/env", () => ({ env: envMock, isFinanceAiConfigured: () => false }));
vi.mock("@/lib/whatsapp/auth", () => authMock);
vi.mock("@/lib/finance-intelligence/snapshot/load", async () => {
  class FinanceDataUnavailableError extends Error {}
  return { ...loadMock, FinanceDataUnavailableError };
});
vi.mock("@/lib/finance-intelligence/observer/run", () => runMock);
// O banco nunca é tocado por uma ferramenta de leitura: se alguém importar o
// client e chamar insert/update, o teste explode.
vi.mock("@/db/client", () => ({
  db: new Proxy(
    {},
    {
      get() {
        throw new Error("ferramenta de leitura tocou o banco");
      },
    },
  ),
}));

const { POST } = await import("./route");
const { resetRateLimitForTests } = await import("@/lib/mos/rate-limit");

const READ = "read-secret";
const ACTION = "action-secret";

function post(body: unknown, authorization?: string) {
  return POST(
    new Request("https://m-finance.test/api/mos/finance/query", {
      method: "POST",
      headers: authorization ? { authorization } : {},
      body: typeof body === "string" ? body : JSON.stringify(body),
    }),
  );
}

beforeEach(() => {
  envMock.mosActionSecret = ACTION;
  envMock.mosFinanceReadSecret = READ;
  resetRateLimitForTests();
  authMock.getWhatsappOwnerUser.mockReset().mockResolvedValue({ id: "user-1" });
  loadMock.loadFinanceSnapshot.mockReset().mockResolvedValue(baseSnapshot());
  runMock.getPreviousSafeToSpend.mockReset().mockResolvedValue(null);
  runMock.getStoredInsights.mockReset().mockResolvedValue([]);
});

describe("autorização", () => {
  it("sem header é 401", async () => {
    const response = await post({ tool: "finance.get_context_pack" });
    expect(response.status).toBe(401);
    expect(loadMock.loadFinanceSnapshot).not.toHaveBeenCalled();
  });

  it("secret errado é 401", async () => {
    expect((await post({ tool: "finance.get_context_pack" }, "Bearer nope")).status).toBe(401);
  });

  it("secret de leitura lê; secret de ação também", async () => {
    expect((await post({ tool: "finance.get_goals" }, `Bearer ${READ}`)).status).toBe(200);
    expect((await post({ tool: "finance.get_goals" }, `Bearer ${ACTION}`)).status).toBe(200);
  });

  it("servidor sem secret de leitura não aceita header vazio", async () => {
    envMock.mosFinanceReadSecret = "";
    envMock.mosActionSecret = "";
    const request = {
      headers: { get: () => "Bearer " },
      text: () => Promise.resolve(JSON.stringify({ tool: "finance.get_goals" })),
    } as unknown as Request;
    expect((await POST(request)).status).toBe(401);
  });
});

describe("contrato", () => {
  it("ferramenta desconhecida é 400", async () => {
    const response = await post({ tool: "finance.drop_tables" }, `Bearer ${READ}`);
    expect(response.status).toBe(400);
    await expect(response.json()).resolves.toMatchObject({ ok: false, error: { code: "unknown_tool" } });
  });

  it("não aceita nome de protótipo como ferramenta", async () => {
    expect((await post({ tool: "constructor" }, `Bearer ${READ}`)).status).toBe(400);
    expect((await post({ tool: "__proto__" }, `Bearer ${READ}`)).status).toBe(400);
  });

  it("args fora do schema é 400, com o campo", async () => {
    const response = await post({ tool: "finance.get_upcoming_commitments", args: { days: 900 } }, `Bearer ${READ}`);
    expect(response.status).toBe(400);
    const body = await response.json();
    expect(body.error.code).toBe("invalid_args");
    expect(body.error.message).toContain("days");
  });

  it("campo extra é recusado (strict)", async () => {
    const response = await post({ tool: "finance.get_goals", args: { sql: "select 1" } }, `Bearer ${READ}`);
    expect(response.status).toBe(400);
  });

  it("JSON inválido é 400", async () => {
    expect((await post("{nao e json", `Bearer ${READ}`)).status).toBe(400);
  });

  it("corpo grande demais é 413", async () => {
    const response = await post({ tool: "finance.get_goals", pad: "x".repeat(20_000) }, `Bearer ${READ}`);
    expect(response.status).toBe(413);
  });

  it("dono não configurado é 500", async () => {
    authMock.getWhatsappOwnerUser.mockResolvedValue(null);
    expect((await post({ tool: "finance.get_goals" }, `Bearer ${READ}`)).status).toBe(500);
  });

  it("limite por minuto", async () => {
    for (let index = 0; index < 60; index += 1) {
      await post({ tool: "finance.get_goals" }, `Bearer ${READ}`);
    }
    expect((await post({ tool: "finance.get_goals" }, `Bearer ${READ}`)).status).toBe(429);
  });
});

describe("respostas", () => {
  it("context pack vem com asOf e dados do kernel", async () => {
    const response = await post({ tool: "finance.get_context_pack" }, `Bearer ${READ}`);
    const body = await response.json();
    expect(body.ok).toBe(true);
    expect(body.tool).toBe("finance.get_context_pack");
    expect(Date.parse(body.asOf)).not.toBeNaN();
    expect(body.data.currentMonth.safeToSpend.safeToSpendCents).toBe(190000);
    expect(body.data.recentInsights.length).toBeGreaterThan(0);
  });

  it("cenário simulado não toca o banco e se declara projeção", async () => {
    const response = await post(
      {
        tool: "finance.simulate_scenario",
        args: {
          horizonMonths: 10,
          changes: [{ type: "installment_purchase", label: "X", totalCents: 600000, installments: 10 }],
        },
      },
      `Bearer ${READ}`,
    );
    const body = await response.json();
    expect(response.status).toBe(200);
    expect(body.data.kind).toBe("projection");
    expect(body.data.totalAddedCommitmentCents).toBe(600000);
  });

  it("análise sem IA configurada devolve código próprio", async () => {
    const response = await post({ tool: "finance.analyze", args: { task: "monthly_review" } }, `Bearer ${READ}`);
    expect(response.status).toBe(422);
    await expect(response.json()).resolves.toMatchObject({ error: { code: "ai_not_configured" } });
  });

  it("carrega o snapshot uma vez por pedido", async () => {
    await post({ tool: "finance.get_context_pack" }, `Bearer ${READ}`);
    expect(loadMock.loadFinanceSnapshot).toHaveBeenCalledTimes(1);
  });
});
