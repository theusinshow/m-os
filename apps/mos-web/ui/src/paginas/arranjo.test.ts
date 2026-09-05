import { describe, expect, it } from "vitest";
import { alternarOculto, aplicarArranjo, ordenar, reordenar, ARRANJO_VAZIO } from "./arranjo";
import type { CartaoDaHome } from "./cartoes";

function cartao(chave: string): CartaoDaHome {
  return { chave, rotulo: chave.toUpperCase(), numero: "1", legenda: "", destino: "home" };
}

const CARTOES = [cartao("sync"), cartao("horas"), cartao("inbox"), cartao("tasks")];
const CHAVES = ["sync", "horas", "inbox", "tasks"];

describe("o arranjo da Home", () => {
  it("sem arranjo, a ordem e a que a Home montou", () => {
    expect(aplicarArranjo(CARTOES, ARRANJO_VAZIO).map((c) => c.chave)).toEqual(CHAVES);
  });

  it("respeita a ordem escolhida", () => {
    const arranjo = { ordem: ["horas", "tasks", "sync", "inbox"], ocultos: [] };
    expect(aplicarArranjo(CARTOES, arranjo).map((c) => c.chave)).toEqual([
      "horas",
      "tasks",
      "sync",
      "inbox",
    ]);
  });

  // Cartão novo — de uma versão futura — não pode sumir só por não estar na
  // ordem guardada: invisível, ninguém descobriria que ele existe.
  it("poe no fim o que a ordem nao conhece", () => {
    const arranjo = { ordem: ["tasks", "sync"], ocultos: [] };
    expect(aplicarArranjo(CARTOES, arranjo).map((c) => c.chave)).toEqual([
      "tasks",
      "sync",
      "horas",
      "inbox",
    ]);
  });

  it("esconde e traz de volta pelo mesmo alvo", () => {
    const escondido = alternarOculto(ARRANJO_VAZIO, "inbox");
    expect(aplicarArranjo(CARTOES, escondido).map((c) => c.chave)).not.toContain("inbox");
    const devolvido = alternarOculto(escondido, "inbox");
    expect(aplicarArranjo(CARTOES, devolvido).map((c) => c.chave)).toContain("inbox");
  });

  // Dentro do modo de arrumar o escondido continua na grade, apagado: é assim
  // que se descobre que ele existe para trazê-lo de volta.
  it("no modo de arrumar, o escondido continua na lista", () => {
    const escondido = alternarOculto(ARRANJO_VAZIO, "inbox");
    expect(ordenar(CARTOES, escondido).map((c) => c.chave)).toContain("inbox");
  });
});

describe("reordenar por arrasto", () => {
  it("tira de uma posicao e enfia noutra", () => {
    expect(reordenar(ARRANJO_VAZIO, CHAVES, 2, 0).ordem).toEqual([
      "inbox",
      "sync",
      "horas",
      "tasks",
    ]);
  });

  it("empurra para tras quando desce", () => {
    expect(reordenar(ARRANJO_VAZIO, CHAVES, 0, 3).ordem).toEqual([
      "horas",
      "inbox",
      "tasks",
      "sync",
    ]);
  });

  // O arrasto dispara a cada pixel: soltar no mesmo lugar não pode gravar um
  // arranjo novo, senão a Home passa a "ter arranjo" só por ter sido tocada.
  it("soltar no mesmo lugar nao muda nada", () => {
    expect(reordenar(ARRANJO_VAZIO, CHAVES, 1, 1)).toBe(ARRANJO_VAZIO);
  });

  it("ignora posicao fora da grade", () => {
    expect(reordenar(ARRANJO_VAZIO, CHAVES, 0, 9)).toBe(ARRANJO_VAZIO);
    expect(reordenar(ARRANJO_VAZIO, CHAVES, -1, 0)).toBe(ARRANJO_VAZIO);
  });
});
