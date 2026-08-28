import { describe, expect, it } from "vitest";
import { estadoDaFaixa, frasePorTipo } from "./syncFaixa";
import type { SyncStatus } from "./types";

function status(over: Partial<SyncStatus> = {}): SyncStatus {
  return {
    endpoint: "http://127.0.0.1:9120",
    hasToken: true,
    pending: 0,
    enabled: true,
    running: false,
    lastSyncAt: "2026-08-28T09:00:00Z",
    lastError: null,
    daySummary: null,
    ...over,
  };
}

describe("quando a faixa não aparece", () => {
  it("com o sync desligado: isso não é problema, é feature desligada", () => {
    expect(estadoDaFaixa(status({ endpoint: "", hasToken: false }))).toBeNull();
  });

  it("quando está tudo em dia", () => {
    expect(estadoDaFaixa(status())).toBeNull();
  });

  it("numa rodada silenciosa — piscar a cada 15 min é o ruído que queremos evitar", () => {
    expect(estadoDaFaixa(status({ running: true }))).toBeNull();
  });

  it("com resumo de zero recebidas: não ter notícia não é notícia", () => {
    expect(estadoDaFaixa(status({ daySummary: { byKind: {}, at: "2026-08-28T09:00:00Z" } }))).toBeNull();
  });
});

describe("quando aparece", () => {
  it("conta o que chegou, e pode ser dispensada", () => {
    const faixa = estadoDaFaixa(status({
      daySummary: { byKind: { task: 3, capture: 1 }, at: "2026-08-28T09:00:00Z" },
    }));
    expect(faixa?.tipo).toBe("chegou");
    expect(faixa?.corpo).toBe("3 tasks · 1 capture");
    expect(faixa?.dispensavel).toBe(true);
  });

  it("o erro ganha da fila: a fila é consequência, o erro é a causa", () => {
    const faixa = estadoDaFaixa(status({ pending: 47, lastError: "connection refused" }));
    expect(faixa?.tipo).toBe("erro");
    expect(faixa?.corpo).toContain("connection refused");
  });

  it("a notícia ganha do erro velho: a rodada que trouxe coisa funcionou", () => {
    const faixa = estadoDaFaixa(status({
      daySummary: { byKind: { task: 2 }, at: "2026-08-28T09:00:00Z" },
    }));
    expect(faixa?.tipo).toBe("chegou");
  });

  it("erro e pendente NÃO se dispensam: somem quando a causa some", () => {
    expect(estadoDaFaixa(status({ lastError: "x" }))?.dispensavel).toBe(false);
    expect(estadoDaFaixa(status({ pending: 5 }))?.dispensavel).toBe(false);
  });

  it("gira quando uma rodada corre por cima de uma faixa que já estava lá", () => {
    expect(estadoDaFaixa(status({ pending: 47, running: true }))?.girando).toBe(true);
  });
});

describe("a frase", () => {
  it("pluraliza, e um de cada não vira plural", () => {
    expect(frasePorTipo({ task: 1 })).toBe("1 task");
  });

  it("um tipo que esta versão não conhece aparece pelo id em vez de sumir", () => {
    expect(frasePorTipo({ tipo_do_futuro: 2 })).toBe("2 tipo_do_futuro");
  });

  it("ordena do maior para o menor, para a notícia grande vir primeiro", () => {
    expect(frasePorTipo({ capture: 1, task: 5 })).toBe("5 tasks · 1 capture");
  });
});
