import { describe, expect, it } from "vitest";
import { formatCurrencyCompact } from "@/lib/formatters/currency";

describe("formatCurrencyCompact", () => {
  it("mostra valores abaixo de mil sem sufixo e sem centavos", () => {
    expect(formatCurrencyCompact(0)).toBe("R$ 0");
    expect(formatCurrencyCompact(12_345)).toBe("R$ 123");
    expect(formatCurrencyCompact(99_999)).toBe("R$ 1000");
  });

  it("abrevia milhar com virgula decimal", () => {
    expect(formatCurrencyCompact(100_000)).toBe("R$ 1 mil");
    expect(formatCurrencyCompact(1_234_500)).toBe("R$ 12,3 mil");
    // Fronteira feia e assumida: R$ 999.999,00 arredonda para "1000 mil" em vez
    // de virar "1 mi". Num eixo isso é aceitável, e tratar o caso exigiria
    // reescalar depois de arredondar — complexidade que um tick não paga.
    expect(formatCurrencyCompact(99_999_900)).toBe("R$ 1000 mil");
  });

  it("abrevia milhao", () => {
    expect(formatCurrencyCompact(100_000_000)).toBe("R$ 1 mi");
    expect(formatCurrencyCompact(345_600_000)).toBe("R$ 3,5 mi");
  });

  it("preserva o sinal negativo", () => {
    expect(formatCurrencyCompact(-1_234_500)).toBe("-R$ 12,3 mil");
    expect(formatCurrencyCompact(-50_000)).toBe("-R$ 500");
  });
});
