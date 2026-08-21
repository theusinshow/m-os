import { describe, expect, it } from "vitest";
import {
  diaDaSemanaCurto,
  podeFechar,
  rotuloDaSemana,
  secoesDaSemana,
  semanaVizinha,
} from "./weekly";
import type { WeekSummary } from "./types";

function resumo(over: Partial<WeekSummary> = {}): WeekSummary {
  return {
    week: "2026-08-17",
    daysWithSession: 5,
    dominated: [],
    recurring: [],
    dropped: [],
    blockedDays: [],
    review: null,
    empty: false,
    ...over,
  };
}

describe("rótulo da semana", () => {
  it("diz o intervalo, e não 'a semana passada'", () => {
    // Quem passa duas semanas sem abrir o M/OS vê a linha apontando para a
    // semana retrasada. O rótulo precisa dizer a data.
    expect(rotuloDaSemana("2026-08-17")).toBe("17 a 23 de agosto");
  });

  it("atravessa mês nomeando os dois", () => {
    expect(rotuloDaSemana("2026-09-28")).toBe("28 de setembro a 4 de outubro");
  });

  it("atravessa ano nomeando os dois", () => {
    expect(rotuloDaSemana("2025-12-29")).toBe(
      "29 de dezembro de 2025 a 4 de janeiro de 2026",
    );
  });

  it("data inválida volta como veio, em vez de virar texto quebrado", () => {
    expect(rotuloDaSemana("lixo")).toBe("lixo");
  });
});

describe("seções", () => {
  it("seção vazia não vira rótulo — zero não desenha título", () => {
    expect(secoesDaSemana(resumo())).toEqual([]);
  });

  it("o que dominou vira linha com o peso em palavras", () => {
    const secoes = secoesDaSemana(
      resumo({ dominated: [{ label: "063-26", mainDays: 3, days: 4 }] }),
    );
    expect(secoes).toHaveLength(1);
    expect(secoes[0].titulo).toBe("O QUE DOMINOU");
    expect(secoes[0].linhas[0]).toEqual({ texto: "063-26", detalhe: "principal em 3 dias" });
  });

  it("o que apareceu sem nunca ser principal diz isso, e não 'principal em 0 dias'", () => {
    const secoes = secoesDaSemana(
      resumo({ dominated: [{ label: "Leituras", mainDays: 0, days: 2 }] }),
    );
    expect(secoes[0].linhas[0].detalhe).toBe("em 2 dias");
  });

  it("singular e plural saem certos", () => {
    const secoes = secoesDaSemana(
      resumo({ dominated: [{ label: "Um dia só", mainDays: 1, days: 1 }] }),
    );
    expect(secoes[0].linhas[0].detalhe).toBe("principal em 1 dia");
  });

  it("as quatro seções saem na ordem da leitura", () => {
    const secoes = secoesDaSemana(
      resumo({
        dominated: [{ label: "063-26", mainDays: 2, days: 2 }],
        recurring: [{ title: "Documentação", timesCarried: 4 }],
        dropped: ["Proposta antiga"],
        blockedDays: ["2026-08-19", "2026-08-20"],
      }),
    );
    expect(secoes.map((secao) => secao.chave)).toEqual([
      "dominated",
      "recurring",
      "dropped",
      "blocked",
    ]);
    expect(secoes[1].linhas[0].detalhe).toBe("carregado 4 vezes");
    // Os dias travados saem numa linha so: cada um e uma palavra de tres
    // letras, e empilha-los gastaria cinco linhas para dizer o que cabe em uma.
    expect(secoes[3].linhas).toHaveLength(1);
    expect(secoes[3].linhas[0].texto).toBe("qua, qui");
  });
});

describe("fechar", () => {
  it("semana sem sessão nenhuma não oferece fecho", () => {
    // Não há o que revisar, e um botão ali ensinaria que o M/OS quer um
    // registro por semana mesmo quando não houve semana.
    expect(podeFechar(resumo({ empty: true, daysWithSession: 0 }))).toBe(false);
    expect(podeFechar(resumo())).toBe(true);
  });

  it("semana já fechada continua podendo ser corrigida", () => {
    const fechada = resumo({
      review: {
        id: "w1",
        week: "2026-08-17",
        summary: "foi boa",
        closedAt: "2026-08-23T21:00:00Z",
        createdAt: "2026-08-23T21:00:00Z",
        updatedAt: "2026-08-23T21:00:00Z",
      },
    });
    expect(podeFechar(fechada)).toBe(true);
  });
});

describe("dia da semana", () => {
  it("sai abreviado e sem passar por new Date(texto)", () => {
    // `new Date("2026-08-19")` é meia-noite UTC; em UTC-3 volta como dia 18, e
    // a quarta viraria terça.
    expect(diaDaSemanaCurto("2026-08-19")).toBe("qua");
    expect(diaDaSemanaCurto("2026-08-17")).toBe("seg");
    expect(diaDaSemanaCurto("lixo")).toBe("");
  });
});

describe("navegação", () => {
  it("anda sete dias para os dois lados", () => {
    expect(semanaVizinha("2026-08-17", -1)).toBe("2026-08-10");
    expect(semanaVizinha("2026-08-17", 1)).toBe("2026-08-24");
  });

  it("atravessa mês e ano sem aritmética de string", () => {
    expect(semanaVizinha("2026-03-02", -1)).toBe("2026-02-23");
    expect(semanaVizinha("2025-12-29", 1)).toBe("2026-01-05");
    // 2028 é bissexto: 28/02 mais sete dias é 06/03, e não 07/03.
    expect(semanaVizinha("2028-02-28", 1)).toBe("2028-03-06");
  });

  it("data inválida não vira lixo", () => {
    expect(semanaVizinha("lixo", 1)).toBe("lixo");
  });
});
