import { describe, expect, it } from "vitest";
import { BARRAS, DEGRAUS, alturaDaBarra, degrausAcesos, empurrar } from "./ondaSonora";

describe("a janela", () => {
  it("nasce cheia de silêncio, para a onda não crescer da esquerda", () => {
    const janela = empurrar([], 500);
    expect(janela).toHaveLength(BARRAS);
    expect(janela[BARRAS - 1]).toBe(500);
    expect(janela[0]).toBe(0);
  });

  it("empurra pela direita e descarta a mais velha", () => {
    let janela = Array.from({ length: BARRAS }, (_, i) => i);
    janela = empurrar(janela, 999);
    expect(janela).toHaveLength(BARRAS);
    expect(janela[BARRAS - 1]).toBe(999);
    expect(janela[0]).toBe(1);
  });

  it("nunca cresce além de BARRAS, por mais que se empurre", () => {
    let janela: number[] = [];
    for (let i = 0; i < BARRAS * 3; i += 1) janela = empurrar(janela, i);
    expect(janela).toHaveLength(BARRAS);
  });
});

describe("o modo sem movimento", () => {
  it("silêncio acende pelo menos um degrau", () => {
    // Mesma razão do PISO: zero degraus leria como "morreu".
    expect(degrausAcesos(0)).toBe(1);
  });

  it("cresce por degraus e satura em DEGRAUS", () => {
    expect(degrausAcesos(1000)).toBe(DEGRAUS);
    expect(degrausAcesos(5000)).toBe(DEGRAUS);
    expect(degrausAcesos(500)).toBeGreaterThan(degrausAcesos(100));
    expect(degrausAcesos(500)).toBeLessThan(DEGRAUS);
  });
});

describe("a altura", () => {
  it("silêncio ainda desenha um traço, e não some", () => {
    // Uma barra de altura zero leria como "morreu", e silêncio não é queda.
    expect(alturaDaBarra(0)).toBeGreaterThan(0);
  });

  it("cresce com o nível e satura no teto", () => {
    expect(alturaDaBarra(500)).toBeGreaterThan(alturaDaBarra(100));
    expect(alturaDaBarra(1000)).toBe(1);
    // Nível acima do esperado não estoura a barra.
    expect(alturaDaBarra(5000)).toBe(1);
  });
});
