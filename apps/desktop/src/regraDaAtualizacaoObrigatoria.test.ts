import { describe, expect, it, vi } from "vitest";
import {
  comPrazo,
  esquecerFalhas,
  FALHAS_ANTES_DE_LIBERAR,
  liberadaPorFalhas,
  podeReiniciar,
  registrarFalha,
  rotuloDoProgresso,
} from "./regraDaAtualizacaoObrigatoria";

function memoria(): Storage {
  const dados = new Map<string, string>();
  return {
    getItem: (chave) => dados.get(chave) ?? null,
    setItem: (chave, valor) => void dados.set(chave, valor),
    removeItem: (chave) => void dados.delete(chave),
    clear: () => dados.clear(),
    key: () => null,
    get length() {
      return dados.size;
    },
  };
}

describe("falhas de instalação", () => {
  it("libera a abertura depois de duas falhas na mesma versão", () => {
    const local = memoria();
    expect(registrarFalha("0.7.10", local)).toBe(1);
    expect(liberadaPorFalhas("0.7.10", local)).toBe(false);
    expect(registrarFalha("0.7.10", local)).toBe(FALHAS_ANTES_DE_LIBERAR);
    expect(liberadaPorFalhas("0.7.10", local)).toBe(true);
  });

  it("uma versão nova volta a trancar", () => {
    const local = memoria();
    registrarFalha("0.7.10", local);
    registrarFalha("0.7.10", local);
    expect(liberadaPorFalhas("0.7.11", local)).toBe(false);
    expect(registrarFalha("0.7.11", local)).toBe(1);
  });

  it("instalar com sucesso esquece as falhas", () => {
    const local = memoria();
    registrarFalha("0.7.10", local);
    registrarFalha("0.7.10", local);
    esquecerFalhas(local);
    expect(liberadaPorFalhas("0.7.10", local)).toBe(false);
  });

  it("sem armazenamento, nunca libera por falha — cada abertura tenta", () => {
    expect(registrarFalha("0.7.10", null)).toBe(1);
    expect(liberadaPorFalhas("0.7.10", null)).toBe(false);
  });

  it("lixo no armazenamento não derruba", () => {
    const local = memoria();
    local.setItem("mos.atualizacao.falhas", "{quebrado");
    expect(liberadaPorFalhas("0.7.10", local)).toBe(false);
  });
});

describe("podeReiniciar", () => {
  it("espera enquanto há gravação de reunião ou de voz", () => {
    expect(podeReiniciar({ reuniao: null, voz: null })).toBe(true);
    expect(podeReiniciar({ reuniao: { id: "r" }, voz: null })).toBe(false);
    expect(podeReiniciar({ reuniao: null, voz: { id: "v" } })).toBe(false);
  });
});

describe("comPrazo", () => {
  it("devolve a resposta que chega a tempo", async () => {
    await expect(comPrazo(Promise.resolve("ok"), 1_000)).resolves.toBe("ok");
  });

  it("devolve null quando a resposta não chega", async () => {
    vi.useFakeTimers();
    const lenta = comPrazo(new Promise(() => undefined), 4_000);
    vi.advanceTimersByTime(4_000);
    await expect(lenta).resolves.toBeNull();
    vi.useRealTimers();
  });
});

describe("rotuloDoProgresso", () => {
  it("mostra a porcentagem quando sabe o tamanho", () => {
    expect(rotuloDoProgresso({ downloaded: 42, total: 100 })).toBe("Baixando a versão nova: 42%");
    expect(rotuloDoProgresso({ downloaded: 10, total: null })).toBe("Baixando a versão nova…");
  });
});
