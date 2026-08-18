import { beforeEach, describe, expect, it, vi } from "vitest";

const { envMock, authMock, bridgeMock } = vi.hoisted(() => ({
  envMock: { mosActionSecret: "" },
  authMock: { getWhatsappOwnerUser: vi.fn() },
  bridgeMock: { createBillFromMosAction: vi.fn() },
}));

vi.mock("@/lib/env", () => ({ env: envMock }));
vi.mock("@/lib/whatsapp/auth", () => authMock);
vi.mock("@/lib/mos/action-bridge", () => bridgeMock);

const { POST } = await import("./route");

const SECRET = "secret-de-teste";

function postRequest(body: unknown, authorization?: string) {
  return new Request("https://m-finance.test/api/mos/actions", {
    method: "POST",
    headers: authorization ? { authorization } : {},
    body: JSON.stringify(body),
  });
}

beforeEach(() => {
  envMock.mosActionSecret = SECRET;
  authMock.getWhatsappOwnerUser.mockReset().mockResolvedValue({ id: "user-1" });
  bridgeMock.createBillFromMosAction.mockReset().mockResolvedValue({ ok: true, billId: "bill-1" });
});

describe("autorizacao", () => {
  it("recusa quando nao vem header de authorization", async () => {
    const response = await POST(postRequest({ actionId: "m-finance.create_bill" }));

    expect(response.status).toBe(401);
    expect(bridgeMock.createBillFromMosAction).not.toHaveBeenCalled();
  });

  it("recusa quando o secret nao bate", async () => {
    const response = await POST(
      postRequest({ actionId: "m-finance.create_bill" }, "Bearer secret-errado"),
    );

    expect(response.status).toBe(401);
    expect(bridgeMock.createBillFromMosAction).not.toHaveBeenCalled();
  });

  it("recusa quando o servidor esta sem MOS_ACTION_SECRET", async () => {
    envMock.mosActionSecret = "";

    const response = await POST(postRequest({ actionId: "m-finance.create_bill" }, "Bearer "));

    expect(response.status).toBe(401);
    expect(bridgeMock.createBillFromMosAction).not.toHaveBeenCalled();
  });

  /**
   * O caso acima passa por dois motivos independentes, e so um deles esta no
   * nosso codigo: `Headers` apara o valor, entao `"Bearer "` chega como
   * `"Bearer"` e nunca casa com o template de um secret vazio. Este teste
   * remove essa rede — entrega o header sem aparar — para exercitar de fato o
   * guard `Boolean(env.mosActionSecret)`, que e o que segura o buraco se um
   * proxy na frente nao aparar.
   */
  it("recusa header nao aparado quando o servidor esta sem MOS_ACTION_SECRET", async () => {
    envMock.mosActionSecret = "";

    const request = {
      headers: { get: () => "Bearer " },
      json: () => Promise.resolve({ actionId: "m-finance.create_bill" }),
    } as unknown as Request;

    const response = await POST(request);

    expect(response.status).toBe(401);
    expect(bridgeMock.createBillFromMosAction).not.toHaveBeenCalled();
  });

  it("confirma que o runtime apara o valor do header de authorization", () => {
    const request = postRequest({}, "Bearer ");

    expect(request.headers.get("authorization")).toBe("Bearer");
  });

  it("nao aceita o secret cru sem o prefixo Bearer", async () => {
    const response = await POST(postRequest({ actionId: "m-finance.create_bill" }, SECRET));

    expect(response.status).toBe(401);
  });
});

describe("roteamento da acao", () => {
  it("recusa uma acao desconhecida", async () => {
    const response = await POST(postRequest({ actionId: "m-finance.delete_everything" }, `Bearer ${SECRET}`));

    expect(response.status).toBe(400);
    await expect(response.json()).resolves.toEqual({
      ok: false,
      error: "Ação desconhecida: m-finance.delete_everything",
    });
    expect(bridgeMock.createBillFromMosAction).not.toHaveBeenCalled();
  });

  it("recusa body que nao e JSON valido, sem estourar excecao", async () => {
    const request = new Request("https://m-finance.test/api/mos/actions", {
      method: "POST",
      headers: { authorization: `Bearer ${SECRET}` },
      body: "isto nao e json",
    });

    const response = await POST(request);

    expect(response.status).toBe(400);
  });

  it("responde 500 quando o usuario autorizado nao esta configurado", async () => {
    authMock.getWhatsappOwnerUser.mockResolvedValue(null);

    const response = await POST(postRequest({ actionId: "m-finance.create_bill" }, `Bearer ${SECRET}`));

    expect(response.status).toBe(500);
    expect(bridgeMock.createBillFromMosAction).not.toHaveBeenCalled();
  });
});

describe("m-finance.create_bill", () => {
  it("repassa os args para o bridge junto do dono e devolve 200", async () => {
    const args = { amountCents: 12345, description: "Internet", dueDay: 15, isRecurring: false };

    const response = await POST(postRequest({ actionId: "m-finance.create_bill", args }, `Bearer ${SECRET}`));

    expect(response.status).toBe(200);
    await expect(response.json()).resolves.toEqual({ ok: true, billId: "bill-1" });
    expect(bridgeMock.createBillFromMosAction).toHaveBeenCalledWith("user-1", args);
  });

  it("devolve 422 quando o bridge recusa a acao", async () => {
    bridgeMock.createBillFromMosAction.mockResolvedValue({ ok: false, error: "Os argumentos da ação não batem com o esperado." });

    const response = await POST(postRequest({ actionId: "m-finance.create_bill", args: {} }, `Bearer ${SECRET}`));

    expect(response.status).toBe(422);
    await expect(response.json()).resolves.toEqual({
      ok: false,
      error: "Os argumentos da ação não batem com o esperado.",
    });
  });
});
