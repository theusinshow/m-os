import { describe, expect, it } from "vitest";
import { TENTATIVAS_DE_ABERTURA, deveEsperarAbertura, esperaDaTentativa } from "./abertura";

const aindaAbrindo = { message: "O M/OS ainda esta abrindo.", retryable: true };
const bancoQuebrado = { message: "O banco nao abriu.", retryable: false };

describe("deveEsperarAbertura", () => {
  it("espera enquanto o backend disser que ainda esta abrindo", () => {
    // A corrida se resolve sozinha em um segundo: tratar isso como falha
    // definitiva foi o que pos a tela de erro no lugar da Home.
    expect(deveEsperarAbertura(aindaAbrindo, 0)).toBe(true);
    expect(deveEsperarAbertura(aindaAbrindo, 3)).toBe(true);
  });

  it("nao espera por falha que nao passa sozinha", () => {
    expect(deveEsperarAbertura(bancoQuebrado, 0)).toBe(false);
  });

  it("desiste depois do limite, porque esperar para sempre e travar", () => {
    expect(deveEsperarAbertura(aindaAbrindo, TENTATIVAS_DE_ABERTURA)).toBe(false);
  });
});

describe("esperaDaTentativa", () => {
  it("comeca curta, porque a abertura normal leva menos de um segundo", () => {
    expect(esperaDaTentativa(0)).toBeLessThanOrEqual(80);
  });

  it("cresce, e para de crescer", () => {
    expect(esperaDaTentativa(1)).toBeGreaterThan(esperaDaTentativa(0));
    expect(esperaDaTentativa(20)).toBe(esperaDaTentativa(10));
  });

  it("o total de espera cabe em poucos segundos", () => {
    let total = 0;
    for (let i = 0; i < TENTATIVAS_DE_ABERTURA; i += 1) total += esperaDaTentativa(i);
    expect(total).toBeLessThan(6000);
    expect(total).toBeGreaterThan(1500);
  });
});
