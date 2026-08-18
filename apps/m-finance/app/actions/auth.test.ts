import { beforeEach, describe, expect, it, vi } from "vitest";

const { envMock, supabaseMock, redirectMock } = vi.hoisted(() => ({
  envMock: { authorizedEmail: "dono@exemplo.com" },
  supabaseMock: {
    cliente: {
      auth: {
        signInWithOtp: vi.fn(),
        verifyOtp: vi.fn(),
        signOut: vi.fn(),
      },
    } as { auth: Record<string, ReturnType<typeof vi.fn>> } | null,
  },
  redirectMock: vi.fn(() => {
    // `redirect` do Next sinaliza por excecao; imitar isso e o que garante que
    // nenhum codigo nosso continue rodando depois dele.
    throw new Error("NEXT_REDIRECT");
  }),
}));

vi.mock("@/lib/env", () => ({ env: envMock }));
vi.mock("next/navigation", () => ({ redirect: redirectMock }));
vi.mock("@/lib/supabase/server", () => ({
  createServerSupabaseClient: () => Promise.resolve(supabaseMock.cliente),
}));

const { continuarLogin } = await import("./auth");

const NO_EMAIL = { step: "email" as const, email: "" };
const NO_CODIGO = { step: "code" as const, email: "dono@exemplo.com" };

function form(campos: Record<string, string>) {
  const dados = new FormData();
  for (const [chave, valor] of Object.entries(campos)) dados.set(chave, valor);
  return dados;
}

const auth = () => supabaseMock.cliente!.auth;

beforeEach(() => {
  envMock.authorizedEmail = "dono@exemplo.com";
  supabaseMock.cliente = {
    auth: {
      signInWithOtp: vi.fn().mockResolvedValue({ error: null }),
      verifyOtp: vi.fn().mockResolvedValue({ error: null }),
      signOut: vi.fn().mockResolvedValue({ error: null }),
    },
  };
  redirectMock.mockClear();
});

describe("pedido do codigo", () => {
  it("manda o codigo para o e-mail autorizado e avanca de passo", async () => {
    const estado = await continuarLogin(NO_EMAIL, form({ email: "dono@exemplo.com" }));

    expect(estado).toMatchObject({ step: "code", email: "dono@exemplo.com", sent: true });
    expect(auth().signInWithOtp).toHaveBeenCalledWith({
      email: "dono@exemplo.com",
      options: { shouldCreateUser: false },
    });
  });

  it("aceita o e-mail com maiusculas e espacos, normalizando", async () => {
    const estado = await continuarLogin(NO_EMAIL, form({ email: "  Dono@Exemplo.COM " }));

    expect(estado.step).toBe("code");
    expect(auth().signInWithOtp).toHaveBeenCalledWith(
      expect.objectContaining({ email: "dono@exemplo.com" }),
    );
  });

  /**
   * O app e de um dono so. Mandar codigo para outro endereco seria usar o
   * servidor de e-mail dele para alcancar terceiros, e ainda criaria a duvida
   * de por que o codigo nunca chega.
   */
  it("nao manda codigo para e-mail nao autorizado", async () => {
    const estado = await continuarLogin(NO_EMAIL, form({ email: "outro@exemplo.com" }));

    expect(estado.step).toBe("email");
    expect(estado.error).toMatch(/apenas o e-mail autorizado/);
    expect(auth().signInWithOtp).not.toHaveBeenCalled();
  });

  /** Sem `AUTHORIZED_EMAIL` no ambiente ninguem entra — falha fechada. */
  it("recusa todo mundo quando nao ha e-mail autorizado configurado", async () => {
    envMock.authorizedEmail = "";

    const estado = await continuarLogin(NO_EMAIL, form({ email: "dono@exemplo.com" }));

    expect(estado.step).toBe("email");
    expect(auth().signInWithOtp).not.toHaveBeenCalled();
  });

  it("cobra o e-mail quando vem vazio", async () => {
    const estado = await continuarLogin(NO_EMAIL, form({ email: "   " }));

    expect(estado.error).toMatch(/Informe o e-mail/);
    expect(auth().signInWithOtp).not.toHaveBeenCalled();
  });

  it("nao avanca de passo quando o envio falha", async () => {
    auth().signInWithOtp.mockResolvedValue({ error: { message: "rate limit" } });

    const estado = await continuarLogin(NO_EMAIL, form({ email: "dono@exemplo.com" }));

    expect(estado.step).toBe("email");
    expect(estado.error).toMatch(/Não consegui enviar/);
  });
});

describe("confirmacao do codigo", () => {
  it("verifica e redireciona para o dashboard", async () => {
    await expect(
      continuarLogin(NO_CODIGO, form({ codigo: "123456" })),
    ).rejects.toThrow("NEXT_REDIRECT");

    expect(auth().verifyOtp).toHaveBeenCalledWith({
      email: "dono@exemplo.com",
      token: "123456",
      type: "email",
    });
    expect(redirectMock).toHaveBeenCalledWith("/app/dashboard");
  });

  it("aceita o codigo digitado com separadores", async () => {
    await expect(
      continuarLogin(NO_CODIGO, form({ codigo: "123 456" })),
    ).rejects.toThrow("NEXT_REDIRECT");

    expect(auth().verifyOtp).toHaveBeenCalledWith(
      expect.objectContaining({ token: "123456" }),
    );
  });

  it("recusa codigo com tamanho errado sem chamar o Supabase", async () => {
    const estado = await continuarLogin(NO_CODIGO, form({ codigo: "12345" }));

    expect(estado).toMatchObject({ step: "code", error: "O código tem 6 dígitos." });
    expect(auth().verifyOtp).not.toHaveBeenCalled();
    expect(redirectMock).not.toHaveBeenCalled();
  });

  it("mantem o usuario na tela quando o codigo e invalido", async () => {
    auth().verifyOtp.mockResolvedValue({ error: { message: "invalid otp" } });

    const estado = await continuarLogin(NO_CODIGO, form({ codigo: "000000" }));

    expect(estado).toMatchObject({ step: "code", email: "dono@exemplo.com" });
    expect(estado.error).toMatch(/inválido ou expirado/);
    expect(redirectMock).not.toHaveBeenCalled();
  });

  /**
   * O e-mail viaja num campo escondido do formulario, entao o cliente pode
   * reescrever. Sem rechecar aqui, um codigo valido de outra caixa entraria.
   */
  it("recusa quando o e-mail do formulario nao e o autorizado", async () => {
    const estado = await continuarLogin(
      { step: "code", email: "outro@exemplo.com" },
      form({ email: "outro@exemplo.com", codigo: "123456" }),
    );

    expect(estado.step).toBe("email");
    expect(auth().verifyOtp).not.toHaveBeenCalled();
    expect(redirectMock).not.toHaveBeenCalled();
  });
});

describe("troca de e-mail", () => {
  it("volta para o primeiro passo sem tocar no Supabase", async () => {
    const estado = await continuarLogin(NO_CODIGO, form({ intent: "trocar-email" }));

    expect(estado).toEqual({ step: "email", email: "" });
    expect(auth().signInWithOtp).not.toHaveBeenCalled();
    expect(auth().verifyOtp).not.toHaveBeenCalled();
  });
});
