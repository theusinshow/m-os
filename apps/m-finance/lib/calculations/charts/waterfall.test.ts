import { describe, expect, it } from "vitest";
import { toWaterfallSteps } from "@/lib/calculations/charts/waterfall";

describe("toWaterfallSteps", () => {
  it("encadeia receita, contas, faturas e sobra", () => {
    const steps = toWaterfallSteps({
      incomeCents: 500_000,
      billsCents: 200_000,
      invoicesCents: 100_000,
    });

    expect(steps.map((step) => [step.label, step.offset, step.delta])).toEqual([
      ["Receita", 0, 500_000],
      ["Contas", 300_000, 200_000],
      ["Faturas", 200_000, 100_000],
      ["Sobra", 0, 200_000],
    ]);
  });

  it("marca o papel de cada passo", () => {
    const steps = toWaterfallSteps({
      incomeCents: 500_000,
      billsCents: 200_000,
      invoicesCents: 100_000,
    });

    expect(steps.map((step) => step.kind)).toEqual(["in", "out", "out", "total"]);
  });

  it("guarda o valor com sinal, separado da altura da barra", () => {
    const steps = toWaterfallSteps({
      incomeCents: 500_000,
      billsCents: 200_000,
      invoicesCents: 100_000,
    });

    expect(steps.map((step) => step.value)).toEqual([500_000, -200_000, -100_000, 200_000]);
  });

  it("desenha a sobra negativa abaixo do zero", () => {
    const steps = toWaterfallSteps({
      incomeCents: 100_000,
      billsCents: 200_000,
      invoicesCents: 50_000,
    });

    const sobra = steps[3];
    expect(sobra.value).toBe(-150_000);
    expect(sobra.offset).toBe(-150_000);
    expect(sobra.delta).toBe(150_000);
  });

  it("nao quebra num mes zerado", () => {
    const steps = toWaterfallSteps({ incomeCents: 0, billsCents: 0, invoicesCents: 0 });

    expect(steps).toHaveLength(4);
    expect(steps.every((step) => step.delta === 0)).toBe(true);
  });
});
