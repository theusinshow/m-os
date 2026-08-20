import { describe, expect, it } from "vitest";
import { cantoPara } from "./argosCorner";

describe("cantoPara", () => {
  it("fica na direita quando a direita esta livre", () => {
    expect(cantoPara({ direitaOcupada: false, esquerdaOcupada: false })).toBe("direita");
  });

  it("fica na direita mesmo com a esquerda ocupada: a esquerda nao e problema dele", () => {
    expect(cantoPara({ direitaOcupada: false, esquerdaOcupada: true })).toBe("direita");
  });

  it("migra para a esquerda quando o toast ou o painel tomam a direita", () => {
    expect(cantoPara({ direitaOcupada: true, esquerdaOcupada: false })).toBe("esquerda");
  });

  it("some quando os dois cantos estao tomados: o aviso do sistema vem primeiro", () => {
    expect(cantoPara({ direitaOcupada: true, esquerdaOcupada: true })).toBe("oculto");
  });

  it("volta sozinho quando um canto vaga", () => {
    expect(cantoPara({ direitaOcupada: true, esquerdaOcupada: true })).toBe("oculto");
    expect(cantoPara({ direitaOcupada: true, esquerdaOcupada: false })).toBe("esquerda");
    expect(cantoPara({ direitaOcupada: false, esquerdaOcupada: false })).toBe("direita");
  });
});
