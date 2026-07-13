import { describe, expect, it } from "vitest";
import { formatClock, formatCurrency, formatDuration } from "./format";

describe("formatDuration", () => {
  it("formata horas e minutos", () => {
    expect(formatDuration(9300)).toBe("2h 35min"); // 2h35
  });
  it("formata apenas minutos quando < 1h", () => {
    expect(formatDuration(2100)).toBe("35min");
  });
  it("formata segundos quando < 1min", () => {
    expect(formatDuration(45)).toBe("45s");
  });
  it("trata valores negativos como zero", () => {
    expect(formatDuration(-10)).toBe("0s");
  });
});

describe("formatClock", () => {
  it("formata HH:MM:SS com zero a esquerda", () => {
    expect(formatClock(9305)).toBe("02:35:05");
  });
});

describe("formatCurrency", () => {
  it("formata centavos como BRL", () => {
    // Normaliza qualquer tipo de espaco entre simbolo e valor,
    // pois o Intl pode usar NBSP (U+00A0) ou narrow NBSP (U+202F).
    const normalized = formatCurrency(12345).replace(/\s/gu, " ");
    expect(normalized).toBe("R$ 123,45");
  });
});
