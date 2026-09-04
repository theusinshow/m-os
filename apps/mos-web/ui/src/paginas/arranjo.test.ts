import { describe, expect, it } from "vitest";
import { aplicarArranjo, mostrar, mover, ocultar, ARRANJO_VAZIO } from "./arranjo";
import type { CartaoDaHome } from "./cartoes";

function cartao(chave: string): CartaoDaHome {
  return { chave, rotulo: chave.toUpperCase(), numero: "1", legenda: "", destino: "home" };
}

const CARTOES = [cartao("sync"), cartao("horas"), cartao("inbox"), cartao("tasks")];

describe("o arranjo da Home", () => {
  it("sem arranjo, a ordem e a que a Home montou", () => {
    expect(aplicarArranjo(CARTOES, ARRANJO_VAZIO).map((c) => c.chave)).toEqual([
      "sync",
      "horas",
      "inbox",
      "tasks",
    ]);
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

  it("esconde e traz de volta", () => {
    const escondido = ocultar(ARRANJO_VAZIO, "inbox");
    expect(aplicarArranjo(CARTOES, escondido).map((c) => c.chave)).not.toContain("inbox");
    const devolvido = mostrar(escondido, "inbox");
    expect(aplicarArranjo(CARTOES, devolvido).map((c) => c.chave)).toContain("inbox");
  });

  it("nao esconde duas vezes", () => {
    const uma = ocultar(ARRANJO_VAZIO, "inbox");
    expect(ocultar(uma, "inbox").ocultos).toEqual(["inbox"]);
  });
});

describe("mover um cartao", () => {
  const VISIVEIS = ["sync", "horas", "inbox", "tasks"];

  it("sobe uma posicao", () => {
    expect(mover(ARRANJO_VAZIO, VISIVEIS, "inbox", "cima").ordem).toEqual([
      "sync",
      "inbox",
      "horas",
      "tasks",
    ]);
  });

  it("desce uma posicao", () => {
    expect(mover(ARRANJO_VAZIO, VISIVEIS, "sync", "baixo").ordem).toEqual([
      "horas",
      "sync",
      "inbox",
      "tasks",
    ]);
  });

  // Nas bordas o toque não faz nada, e não dá a volta: um cartão que salta do
  // topo para o fim parece que sumiu.
  it("no topo, subir nao faz nada", () => {
    expect(mover(ARRANJO_VAZIO, VISIVEIS, "sync", "cima")).toEqual(ARRANJO_VAZIO);
  });

  it("no fim, descer nao faz nada", () => {
    expect(mover(ARRANJO_VAZIO, VISIVEIS, "tasks", "baixo")).toEqual(ARRANJO_VAZIO);
  });

  it("ignora chave que nao esta na tela", () => {
    expect(mover(ARRANJO_VAZIO, VISIVEIS, "fantasma", "cima")).toEqual(ARRANJO_VAZIO);
  });
});
