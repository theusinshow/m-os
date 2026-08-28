import { describe, expect, it } from "vitest";
import { daquiA, disponiveis, ESCOLHAS, padrao, porExtenso } from "./instantes";

/**
 * A aritmética de fuso, conferida contra datas fixas.
 *
 * Um lembrete resolvido errado não falha — ele **toca na hora errada**, que é
 * pior: uma falha ensina a olhar, um horário errado ensina a não confiar no
 * relógio. E é o tipo de defeito que só aparece numa segunda-feira, ou às 19h,
 * ou na virada do mês. Nenhum desses momentos chega quando se está olhando.
 */

/** Resolve um rótulo contra uma base fixa. */
function resolver(rotulo: string, base: Date): Date {
  const escolha = ESCOLHAS.find((candidata) => candidata.rotulo === rotulo);
  if (!escolha) throw new Error(`nao ha escolha chamada ${rotulo}`);
  return escolha.resolver(base);
}

describe("as escolhas rápidas", () => {
  it("soma minutos e horas a partir da base, e não do relógio", () => {
    const base = new Date(2026, 7, 28, 10, 0, 0);
    expect(resolver("15 min", base).getTime()).toBe(base.getTime() + 15 * 60_000);
    expect(resolver("1 hora", base).getTime()).toBe(base.getTime() + 60 * 60_000);
    expect(resolver("3 horas", base).getTime()).toBe(base.getTime() + 3 * 60 * 60_000);
  });

  it("'Amanhã 9h' cai às nove do dia seguinte, e não daqui a 24 h", () => {
    // 23h50: somar um dia à hora daria 23h50 de amanhã, que não é "de manhã".
    const base = new Date(2026, 7, 28, 23, 50, 0);
    const quando = resolver("Amanhã 9h", base);
    expect(quando.getDate()).toBe(29);
    expect(quando.getHours()).toBe(9);
    expect(quando.getMinutes()).toBe(0);
  });

  it("'Amanhã 9h' atravessa a virada do mês", () => {
    const base = new Date(2026, 7, 31, 14, 0, 0);
    const quando = resolver("Amanhã 9h", base);
    expect(quando.getMonth()).toBe(8);
    expect(quando.getDate()).toBe(1);
  });

  it("'Segunda 9h' numa segunda quer dizer a PRÓXIMA", () => {
    // 2026-08-31 é uma segunda-feira.
    const base = new Date(2026, 7, 31, 8, 0, 0);
    expect(base.getDay()).toBe(1);
    const quando = resolver("Segunda 9h", base);
    // Sete dias à frente, e não "daqui a uma hora": quem pede segunda numa
    // segunda de manhã está falando da semana que vem.
    expect(quando.getDate()).toBe(7);
    expect(quando.getMonth()).toBe(8);
    expect(quando.getDay()).toBe(1);
    expect(quando.getHours()).toBe(9);
  });

  it("'Segunda 9h' num domingo é o dia seguinte", () => {
    const base = new Date(2026, 7, 30, 20, 0, 0);
    expect(base.getDay()).toBe(0);
    const quando = resolver("Segunda 9h", base);
    expect(quando.getDate()).toBe(31);
    expect(quando.getDay()).toBe(1);
  });
});

describe("o que é oferecido", () => {
  it("esconde 'Hoje 18h' depois das 18h", () => {
    const base = new Date(2026, 7, 28, 19, 30, 0);
    const rotulos = disponiveis(base).map((escolha) => escolha.rotulo);
    // Ela já passou: o servidor a recusaria, e oferecê-la seria oferecer um erro.
    expect(rotulos).not.toContain("Hoje 18h");
    expect(rotulos).toContain("Amanhã 9h");
  });

  it("oferece 'Hoje 18h' de manhã", () => {
    const base = new Date(2026, 7, 28, 9, 0, 0);
    expect(disponiveis(base).map((escolha) => escolha.rotulo)).toContain("Hoje 18h");
  });

  it("toda opção oferecida está no futuro", () => {
    for (const hora of [0, 8, 12, 17, 18, 23]) {
      const base = new Date(2026, 7, 28, hora, 0, 0);
      for (const escolha of disponiveis(base)) {
        expect(escolha.resolver(base).getTime()).toBeGreaterThan(base.getTime());
      }
    }
  });

  it("o padrão nunca cai no passado, nem à meia-noite", () => {
    const base = new Date(2026, 7, 28, 23, 59, 0);
    expect(padrao(base).getTime()).toBeGreaterThan(base.getTime());
  });
});

describe("a leitura do tempo", () => {
  const agora = new Date(2026, 7, 28, 12, 0, 0);

  it("diz quanto falta", () => {
    const daqui = new Date(agora.getTime() + 45 * 60_000);
    expect(daquiA(daqui.toISOString(), agora)).toBe("em 45 min");
  });

  it("diz quanto PASSOU, e não um número negativo", () => {
    const antes = new Date(agora.getTime() - 50 * 60_000);
    expect(daquiA(antes.toISOString(), agora)).toBe("venceu há 50 min");
  });

  it("passa de minutos para horas e dias", () => {
    expect(daquiA(new Date(agora.getTime() + 3 * 3_600_000).toISOString(), agora)).toBe("em 3 h");
    expect(daquiA(new Date(agora.getTime() + 48 * 3_600_000).toISOString(), agora)).toBe("em 2 d");
  });

  it("um lembrete sem hora diz isso, e não uma data inventada", () => {
    expect(daquiA(null, agora)).toBe("sem hora");
  });

  it("o instante por extenso traz o dia E a hora", () => {
    // Sem a hora, a folha diria "sex 28/08" para um lembrete das 9h e para um
    // das 18h — e são decisões diferentes.
    const texto = porExtenso(new Date(2026, 7, 28, 18, 30, 0));
    expect(texto).toContain("28");
    expect(texto).toContain("18:30");
  });
});
