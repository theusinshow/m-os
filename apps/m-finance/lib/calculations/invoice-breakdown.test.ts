import { describe, expect, it } from "vitest";
import { breakdownInvoice, nextInvoiceTotal } from "@/lib/calculations/invoice-breakdown";

const compra = (description: string, amountCents: number, extra = {}) => ({
  id: description + amountCents,
  description,
  amountCents,
  installmentId: null,
  installmentNumber: null,
  installmentTotal: null,
  ...extra,
});

describe("breakdownInvoice", () => {
  it("classifica parte da fatura e joga o resto em Outros", () => {
    const result = breakdownInvoice(1_027_00, [
      compra("iFood", 120_00),
      compra("Netflix", 55_00),
    ]);

    expect(result.totalCents).toBe(1_027_00);
    expect(result.classifiedCents).toBe(175_00);
    expect(result.unclassifiedCents).toBe(852_00);
    expect(result.slices).toEqual([
      { name: "Outros", valueCents: 852_00, percent: 83, isRemainder: true },
      { name: "iFood", valueCents: 120_00, percent: 12, isRemainder: false },
      { name: "Netflix", valueCents: 55_00, percent: 5, isRemainder: false },
    ]);
  });

  it("agrupa lançamentos com a mesma origem", () => {
    const result = breakdownInvoice(300_00, [
      compra("iFood", 50_00),
      compra("ifood", 30_00),
      compra("  iFood ", 20_00),
    ]);

    expect(result.slices.find((slice) => !slice.isRemainder)).toEqual({
      name: "iFood",
      valueCents: 100_00,
      percent: 33,
      isRemainder: false,
    });
    expect(result.unclassifiedCents).toBe(200_00);
  });

  it("fatura toda classificada não inventa a fatia Outros", () => {
    const result = breakdownInvoice(100_00, [compra("Mercado", 100_00)]);

    expect(result.unclassifiedCents).toBe(0);
    expect(result.slices.map((slice) => slice.name)).toEqual(["Mercado"]);
    expect(result.isOverclassified).toBe(false);
  });

  it("classificado acima do total avisa em vez de mostrar Outros negativo", () => {
    const result = breakdownInvoice(100_00, [compra("Mercado", 150_00)]);

    expect(result.unclassifiedCents).toBe(0);
    expect(result.isOverclassified).toBe(true);
    expect(result.classifiedCents).toBe(150_00);
  });

  it("fatura sem nenhuma compra é uma fatia Outros só", () => {
    const result = breakdownInvoice(400_00, []);

    expect(result.slices).toEqual([
      { name: "Outros", valueCents: 400_00, percent: 100, isRemainder: true },
    ]);
  });

  it("fatura zerada não divide por zero", () => {
    expect(breakdownInvoice(0, []).slices).toEqual([]);
  });
});

describe("nextInvoiceTotal", () => {
  it("a primeira compra cria a fatura com o próprio valor", () => {
    expect(nextInvoiceTotal({ currentTotal: null, previousSum: 0, newSum: 50_00 })).toBe(50_00);
  });

  it("total que era a soma continua sendo a soma — inclusive ao excluir", () => {
    expect(nextInvoiceTotal({ currentTotal: 80_00, previousSum: 80_00, newSum: 50_00 })).toBe(
      50_00,
    );
  });

  it("total digitado à mão manda: classificar não mexe nele", () => {
    expect(
      nextInvoiceTotal({ currentTotal: 1_027_00, previousSum: 0, newSum: 175_00 }),
    ).toBe(1_027_00);
  });

  it("não dá para classificar mais do que a fatura tem: o total sobe", () => {
    expect(
      nextInvoiceTotal({ currentTotal: 100_00, previousSum: 0, newSum: 150_00 }),
    ).toBe(150_00);
  });

  it("sem fatura e sem compra, não há fatura", () => {
    expect(nextInvoiceTotal({ currentTotal: null, previousSum: 0, newSum: 0 })).toBe(0);
  });
});
