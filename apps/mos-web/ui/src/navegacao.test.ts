import { describe, expect, it } from "vitest";
import { contagemDe, DESTINOS } from "./navegacao";
import { FALSO } from "./falso";

describe("a barra de baixo", () => {
  it("tem cinco lugares, com a agenda e o capturar no meio", () => {
    expect(DESTINOS.map((d) => d.pagina)).toEqual([
      "home",
      "agenda",
      "capturar",
      "fazer",
      "mais",
    ]);
  });

  // O badge de FAZER soma as duas metades da tela. Se ele contasse só as tasks,
  // o número na barra discordaria do que se vê ao tocar nela — e o badge que
  // mente é pior que badge nenhum.
  it("soma capturas e tasks abertas no badge de fazer", () => {
    expect(contagemDe("fazer", FALSO)).toBe(5);
  });

  it("poe em `mais` o que cobra acao — o lembrete vencido", () => {
    expect(contagemDe("mais", FALSO)).toBe(1);
  });

  it("nao conta nada em home, agenda nem capturar", () => {
    expect(contagemDe("home", FALSO)).toBe(0);
    expect(contagemDe("agenda", FALSO)).toBe(0);
    expect(contagemDe("capturar", FALSO)).toBe(0);
  });
});
