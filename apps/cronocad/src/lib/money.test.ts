import { describe, expect, it } from "vitest";
import { amountForDuration, fromCents, toCents } from "./money";

describe("amountForDuration", () => {
  it("calcula 1h a R$100,00/h = R$100,00", () => {
    expect(amountForDuration(3600, 10000)).toBe(10000);
  });

  it("calcula 1h30 a R$100,00/h = R$150,00", () => {
    expect(amountForDuration(5400, 10000)).toBe(15000);
  });

  it("arredonda para o centavo mais proximo", () => {
    // 1h07 (4020s) a R$90,00/h -> 4020/3600 * 9000 = 10050 centavos
    expect(amountForDuration(4020, 9000)).toBe(10050);
  });

  it("retorna 0 para duracao zero", () => {
    expect(amountForDuration(0, 10000)).toBe(0);
  });
});

describe("conversao de centavos", () => {
  it("toCents converte reais para centavos", () => {
    expect(toCents(123.45)).toBe(12345);
  });
  it("fromCents converte centavos para reais", () => {
    expect(fromCents(12345)).toBe(123.45);
  });
});
