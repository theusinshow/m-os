import { describe, expect, it } from "vitest";
import { SETTINGS_SECTIONS, secaoVisivel } from "./settingsNav";

describe("o catálogo", () => {
  it("põe Sincronização primeiro: é a única que se procura, e não se acha por acaso", () => {
    expect(SETTINGS_SECTIONS[0].id).toBe("sync");
  });

  it("todo id é único — ele vira âncora e alvo de rolagem", () => {
    const ids = SETTINGS_SECTIONS.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe("qual seção está visível", () => {
  const posicoes = [
    { id: "sync", top: 0 },
    { id: "conexoes", top: 400 },
    { id: "aparencia", top: 900 },
  ];

  it("é a primeira quando ainda não rolou", () => {
    expect(secaoVisivel(posicoes, 0)).toBe("sync");
  });

  it("um pouco ANTES do topo já conta, senão a marca fica um passo atrás do olho", () => {
    expect(secaoVisivel(posicoes, 390)).toBe("conexoes");
  });

  it("no fim da página é a última, mesmo que ela seja curta demais para encher a tela", () => {
    expect(secaoVisivel(posicoes, 5000)).toBe("aparencia");
  });

  it("sem seções não quebra", () => {
    expect(secaoVisivel([], 100)).toBe("");
  });
});
