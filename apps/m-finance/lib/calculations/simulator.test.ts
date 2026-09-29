import { describe, expect, it } from "vitest";
import { computeSimulation } from "@/lib/calculations/simulator";

describe("computeSimulation", () => {
  const base = {
    totalAmountCents: 60000,
    paymentType: "installment" as const,
    installments: 3,
    startMonth: 9,
    startYear: 2026,
    baselineRemainingCents: 100000,
  };

  it("sem mapa, a base é plana (comportamento anterior)", () => {
    const result = computeSimulation(base);
    expect(result.months.map((row) => row.baselineRemainingCents)).toEqual([100000, 100000, 100000]);
  });

  it("com o mapa do engine, cada mês usa a própria sobra; o que falta cai na plana", () => {
    const result = computeSimulation({
      ...base,
      baselineByMonth: new Map([
        ["2026-09", 240000],
        ["2026-10", 20000],
      ]),
    });
    expect(result.months.map((row) => row.baselineRemainingCents)).toEqual([240000, 20000, 100000]);
    // Outubro fica negativo: o risco sai do mês mais apertado, não da média.
    expect(result.months[1].remainingWithCents).toBe(0);
    expect(result.riskLevel).toBe("tight");
  });
});
