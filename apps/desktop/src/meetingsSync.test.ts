import { describe, expect, it } from "vitest";
import { EVENTOS_DE_REUNIAO, selecaoAoFocar } from "./meetingsSync";

describe("os eventos que recarregam a lista", () => {
  it("inclui o parar, que era justamente o que faltava", () => {
    // O defeito de 20/08: `meeting_stop` nao emitia nada, a pagina so escutava
    // fim de transcricao, analise e falha — e quem parava de uma pagina ja
    // aberta via a lista continuar dizendo "gravando".
    expect(EVENTOS_DE_REUNIAO).toContain("meeting-stopped");
  });

  it("cobre todo fim de estagio, e nada alem", () => {
    expect([...EVENTOS_DE_REUNIAO].sort()).toEqual([
      "meeting-analyzed",
      "meeting-failed",
      "meeting-stopped",
      "meeting-transcribed",
    ]);
  });
});

describe("selecaoAoFocar", () => {
  it("um foco novo troca a selecao de uma pagina ja montada", () => {
    expect(selecaoAoFocar("recem-parada", "a que estava aberta")).toBe("recem-parada");
  });

  it("sem foco, a escolha da pessoa fica de pe", () => {
    expect(selecaoAoFocar(null, "a que estava aberta")).toBe("a que estava aberta");
    expect(selecaoAoFocar(undefined, "a que estava aberta")).toBe("a que estava aberta");
  });

  it("sem foco e sem escolha, nada fica selecionado", () => {
    expect(selecaoAoFocar(null, null)).toBeNull();
  });
});
