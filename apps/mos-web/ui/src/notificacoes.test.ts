import { afterEach, describe, expect, it, vi } from "vitest";
import { marcarIcone } from "./notificacoes";

/** Instala uma `navigator` de mentira e devolve o que ela registrou. */
function comNavigator(implementacao: Partial<Navigator> & Record<string, unknown>) {
  vi.stubGlobal("navigator", implementacao);
  return implementacao;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("o numero no icone", () => {
  it("escreve o numero quando ha o que cobrar", async () => {
    const marcado: number[] = [];
    comNavigator({ setAppBadge: async (n?: number) => void marcado.push(n ?? 0) });
    await marcarIcone(3);
    expect(marcado).toEqual([3]);
  });

  // Zero não é "escreva 0": é apagar. Um ícone com "0" no canto é um ícone
  // dizendo que há zero coisas — e a ausência já diz isso melhor.
  it("apaga em vez de escrever zero", async () => {
    let limpou = false;
    comNavigator({
      setAppBadge: async () => {
        throw new Error("não deveria escrever");
      },
      clearAppBadge: async () => void (limpou = true),
    });
    await marcarIcone(0);
    expect(limpou).toBe(true);
  });

  // No iOS a API só existe com o app instalado e a permissão concedida. Um app
  // que quebrasse por não conseguir escrever um número no ícone trocaria um
  // enfeite por um defeito.
  it("nao quebra onde a API nao existe", async () => {
    comNavigator({});
    await expect(marcarIcone(2)).resolves.toBeUndefined();
  });

  it("nao quebra quando a API lanca", async () => {
    comNavigator({
      setAppBadge: async () => {
        throw new Error("permissão negada");
      },
    });
    await expect(marcarIcone(2)).resolves.toBeUndefined();
  });
});
