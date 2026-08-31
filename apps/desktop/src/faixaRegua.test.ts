import { describe, expect, it } from "vitest";
import { faltaPara, nomeDaRegua, regua, rotuloDaRegua, temRegua } from "./Faixa";
import type { AnelDaFaixa, JanelaDaFaixa } from "./types";

/**
 * A escolha da régua é a decisão inteira da ADR-062, e é o tipo de coisa que
 * some numa refatoração sem que nenhum teste de renderização perceba: o anel
 * continua bonito medindo contra a régua errada.
 */

function anel(over: Partial<AnelDaFaixa> = {}): AnelDaFaixa {
  return {
    nome: "Claude Code",
    peso: 40_000,
    pico: 100_000,
    pesoHoje: 60_000,
    picoDia: 200_000,
    requisicoes: 12,
    requisicoesHoje: 30,
    resetaEm: "2026-08-31T15:50:00Z",
    janelasConhecidas: 9,
    cotaSessao: null,
    cotaSemana: null,
    temHistorico: true,
    ...over,
  };
}

function janela(over: Partial<JanelaDaFaixa> = {}): JanelaDaFaixa {
  return { percentual: 23, resetaEm: "2026-08-31T15:50:00Z", obsoleta: false, ...over };
}

describe("qual régua está valendo", () => {
  it("com cota, é a cota — mesmo havendo pico de sobra", () => {
    const r = regua(anel({ cotaSessao: janela() }), false);
    expect(r.tipo).toBe("cota");
    expect(rotuloDaRegua(r, anel())).toBe("23%");
    expect(nomeDaRegua(r, false)).toBe("DA SESSÃO");
  });

  it("sem cota, cai no pico — a régua da ADR-059 não foi embora", () => {
    const r = regua(anel(), false);
    expect(r.tipo).toBe("pico");
    // 40k contra o pico de 100k.
    expect(rotuloDaRegua(r, anel())).toBe("40%");
    expect(nomeDaRegua(r, false)).toBe("DO PICO");
  });

  it("sem cota e sem histórico, não inventa régua nenhuma", () => {
    const r = regua(anel({ janelasConhecidas: 1 }), false);
    expect(r.tipo).toBe("nenhuma");
    expect(nomeDaRegua(r, false)).toBe("SEM RÉGUA");
    expect(nomeDaRegua(r, true)).toBe("LENDO");
  });

  it("calibrando não impede a cota: ela não depende do histórico local", () => {
    // A varredura de meio giga pode levar minutos, e segurar um número que já
    // chegou do servidor por causa dela seria esconder o dado bom.
    const r = regua(anel({ cotaSessao: janela({ percentual: 71 }) }), true);
    expect(r.tipo).toBe("cota");
    expect(rotuloDaRegua(r, anel())).toBe("71%");
  });
});

describe("o valor velho", () => {
  it("continua aparecendo, marcado com ~", () => {
    const r = regua(anel({ cotaSessao: janela({ obsoleta: true }) }), false);
    expect(rotuloDaRegua(r, anel())).toBe("~23%");
  });

  it("o ~ é o que separa velho de agora — sem ele os dois seriam o mesmo texto", () => {
    const velho = regua(anel({ cotaSessao: janela({ obsoleta: true }) }), false);
    const novo = regua(anel({ cotaSessao: janela({ obsoleta: false }) }), false);
    expect(rotuloDaRegua(velho, anel())).not.toBe(rotuloDaRegua(novo, anel()));
  });
});

describe("acima de cem por cento", () => {
  it("o número passa inteiro: é a hora em que ele mais importa", () => {
    const r = regua(anel({ cotaSessao: janela({ percentual: 103 }) }), false);
    expect(rotuloDaRegua(r, anel())).toBe("103%");
  });
});

describe("temRegua continua sendo só sobre o pico", () => {
  it("uma janela conhecida não é régua: o pico seria a própria sessão", () => {
    expect(temRegua(anel({ janelasConhecidas: 1 }), false)).toBe(false);
    expect(temRegua(anel({ pico: 0 }), false)).toBe(false);
    expect(temRegua(anel(), true)).toBe(false);
    expect(temRegua(anel(), false)).toBe(true);
  });
});

describe("o prazo", () => {
  const agora = Date.parse("2026-08-31T14:00:00Z");

  it("conta em minutos e depois em horas", () => {
    expect(faltaPara("2026-08-31T14:51:00Z", agora)).toBe("reseta em 51 min");
    expect(faltaPara("2026-08-31T16:13:00Z", agora)).toBe("reseta em 2h13");
  });

  it("acima de um dia vira dias: a semana não se lê em 159 horas", () => {
    // Foi o que a foto do painel mostrou antes deste degrau existir.
    expect(faltaPara("2026-09-07T05:50:00Z", agora)).toBe("reseta em 6d15h");
    expect(faltaPara("2026-09-01T14:00:00Z", agora)).toBe("reseta em 1d0h");
  });

  it("logo abaixo de um dia ainda conta em horas", () => {
    expect(faltaPara("2026-09-01T13:59:00Z", agora)).toBe("reseta em 23h59");
  });

  it("vencido não vira número negativo", () => {
    expect(faltaPara("2026-08-31T13:00:00Z", agora)).toBe("reseta agora");
  });
});
