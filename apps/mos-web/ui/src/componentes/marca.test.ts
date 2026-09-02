import { describe, expect, it } from "vitest";
import { pontosDoPoligono, poligonoPara } from "./marca";

// O brief (`docs/BRIEF-SISTEMA-DE-LOGOS.md`) manda corrigir o ANGULO por
// escala, e proibe escalar um SVG so. A mesma inclinacao geometrica le mais
// fina conforme o desenho encolhe; o angulo abre para compensar.
describe("a barra escolhe o poligono pelo tamanho", () => {
  it("usa 22 graus de 128 para cima", () => {
    expect(pontosDoPoligono(512)).toBe("38,8 53,8 26,56 11,56");
    expect(pontosDoPoligono(128)).toBe("38,8 53,8 26,56 11,56");
  });

  it("usa 18 graus entre 48 e 127", () => {
    expect(pontosDoPoligono(64)).toBe("40,10 54,10 24,54 10,54");
    expect(pontosDoPoligono(48)).toBe("40,10 54,10 24,54 10,54");
  });

  it("usa 14 graus abaixo de 48", () => {
    expect(pontosDoPoligono(32)).toBe("42,12 56,12 22,52 8,52");
    expect(pontosDoPoligono(16)).toBe("42,12 56,12 22,52 8,52");
  });

  it("mantem os quatro vertices no viewBox de 64", () => {
    for (const tamanho of [512, 64, 16]) {
      const pontos = poligonoPara(tamanho);
      expect(pontos).toHaveLength(4);
      for (const [x, y] of pontos) {
        expect(x).toBeGreaterThanOrEqual(0);
        expect(x).toBeLessThanOrEqual(64);
        expect(y).toBeGreaterThanOrEqual(0);
        expect(y).toBeLessThanOrEqual(64);
      }
    }
  });
});
