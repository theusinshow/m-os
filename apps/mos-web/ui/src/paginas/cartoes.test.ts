import { describe, expect, it } from "vitest";
import { cartoesDaHome } from "./cartoes";
import { FALSO } from "../falso";

const AGORA = new Date("2026-09-02T14:30:00Z");

describe("os cartoes da Home", () => {
  it("mostra a fila do sync, a inbox, as tasks abertas e o que vence", () => {
    const chaves = cartoesDaHome(FALSO.estado, FALSO, AGORA).map((c) => c.chave);
    expect(chaves).toEqual(["sync", "hoje", "inbox", "tasks", "ultima"]);
  });

  it("conta como task aberta o que nao esta feito", () => {
    const tasks = cartoesDaHome(FALSO.estado, FALSO, AGORA).find((c) => c.chave === "tasks");
    expect(tasks?.numero).toBe("2");
    expect(tasks?.destino).toBe("tasks");
  });

  it("marca urgente o cartao de hoje quando ha lembrete cobrando", () => {
    const hoje = cartoesDaHome(FALSO.estado, FALSO, AGORA).find((c) => c.chave === "hoje");
    expect(hoje?.urgente).toBe(true);
  });

  // Cartao vazio prometendo conteudo e pior que a ausencia dele: ele ensina que
  // a Home tem lugares que nunca dizem nada.
  it("omite o cartao que nao tem o que dizer", () => {
    const vazio = { capturas: [], tasks: [], lembretes: [] };
    const chaves = cartoesDaHome({ ...FALSO.estado, pendentes: 0 }, vazio, AGORA).map(
      (c) => c.chave,
    );
    expect(chaves).toEqual(["sync"]);
  });

  it("diz EM DIA quando nao ha nada na fila", () => {
    const sync = cartoesDaHome({ ...FALSO.estado, pendentes: 0 }, FALSO, AGORA).find(
      (c) => c.chave === "sync",
    );
    expect(sync?.numero).toBe("EM DIA");
  });

  it("avisa quando o aparelho nao tem hub", () => {
    const sync = cartoesDaHome({ ...FALSO.estado, sincroniza: false }, FALSO, AGORA).find(
      (c) => c.chave === "sync",
    );
    expect(sync?.numero).toBe("SEM HUB");
    expect(sync?.urgente).toBe(true);
  });
});

describe("os cartoes do panorama", () => {
  const PANORAMA = {
    horas: { semanaSegundos: 32_880, semanaValorCents: 27_400, hojeSegundos: 3_600 },
    proximos: [
      {
        titulo: "Prova de Cálculo III",
        disciplina: "Cálculo III",
        quando: "2026-09-06T14:00:00Z",
        tipo: "exam",
      },
    ],
  };

  it("mostra as horas da semana com o valor na legenda", () => {
    const horas = cartoesDaHome(FALSO.estado, FALSO, AGORA, PANORAMA).find(
      (c) => c.chave === "horas",
    );
    expect(horas?.numero).toBe("9h08");
    expect(horas?.legenda).toBe("R$ 274,00 nesta semana");
    expect(horas?.destino).toBe("home");
  });

  it("mostra o proximo compromisso do academico", () => {
    const academico = cartoesDaHome(FALSO.estado, FALSO, AGORA, PANORAMA).find(
      (c) => c.chave === "academico",
    );
    expect(academico?.numero).toBe("1");
    expect(academico?.legenda).toBe("Prova de Cálculo III");
  });

  // Semana sem hora nao vira cartao vazio: a regra da Home ja era essa, e o
  // panorama nao abre excecao para si mesmo.
  it("omite as horas quando a semana esta zerada", () => {
    const vazio = {
      horas: { semanaSegundos: 0, semanaValorCents: 0, hojeSegundos: 0 },
      proximos: [],
    };
    const chaves = cartoesDaHome(FALSO.estado, FALSO, AGORA, vazio).map((c) => c.chave);
    expect(chaves).not.toContain("horas");
    expect(chaves).not.toContain("academico");
  });

  // Sem panorama — servidor velho, ou a chamada falhou — a Home continua a
  // mesma de antes, e nao uma tela quebrada.
  it("sem panorama, a Home e a de antes", () => {
    const chaves = cartoesDaHome(FALSO.estado, FALSO, AGORA, null).map((c) => c.chave);
    expect(chaves).toEqual(["sync", "hoje", "inbox", "tasks", "ultima"]);
  });
});
