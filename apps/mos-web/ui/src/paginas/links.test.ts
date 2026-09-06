import { describe, expect, it } from "vitest";
import { dominioDe, enderecoEm } from "./links";

describe("o endereco dentro da captura", () => {
  it("acha o link no meio da frase", () => {
    expect(enderecoEm("tabela de aço https://exemplo.com/ca50 boa pra consultar")).toBe(
      "https://exemplo.com/ca50",
    );
  });

  // Um link colado no fim de uma frase leva a pontuação junto, e um endereço
  // com ponto no fim abre uma página que não existe.
  it("nao leva a pontuacao junto", () => {
    expect(enderecoEm("ver https://exemplo.com/a.")).toBe("https://exemplo.com/a");
    expect(enderecoEm("ver (https://exemplo.com/a)")).toBe("https://exemplo.com/a");
  });

  it("texto sem link nao inventa um", () => {
    expect(enderecoEm("o fck do concreto é 30 MPa")).toBeNull();
    expect(enderecoEm("exemplo.com sem protocolo")).toBeNull();
  });
});

describe("o dominio", () => {
  it("tira o www", () => {
    expect(dominioDe("https://www.exemplo.com/a/b")).toBe("exemplo.com");
  });

  it("endereco quebrado nao derruba a linha", () => {
    expect(dominioDe("nao é url")).toBeNull();
  });
});
