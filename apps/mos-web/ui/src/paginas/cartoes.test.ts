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
