import { describe, expect, it } from "vitest";
import {
  firstNegativeMonth,
  toSimulationSeries,
} from "@/lib/calculations/charts/simulation-series";
import type { SimulationMonth } from "@/lib/calculations/simulator";

function month(
  overrides: Partial<SimulationMonth> & Pick<SimulationMonth, "month">,
): SimulationMonth {
  return {
    year: 2026,
    baselineRemainingCents: 100_000,
    impactCents: 40_000,
    remainingWithCents: 60_000,
    health: "positive",
    ...overrides,
  };
}

describe("toSimulationSeries", () => {
  it("rotula com mes abreviado e ano de dois digitos", () => {
    const points = toSimulationSeries([month({ month: 8 }), month({ month: 9 })]);
    expect(points.map((point) => point.label)).toEqual(["ago/26", "set/26"]);
  });

  it("separa a sobra sem e com a compra", () => {
    const points = toSimulationSeries([
      month({ month: 8, baselineRemainingCents: 100_000, remainingWithCents: 60_000 }),
    ]);

    expect(points[0].semCompra).toBe(100_000);
    expect(points[0].comCompra).toBe(60_000);
  });

  it("aguenta uma projecao de um mes so", () => {
    expect(toSimulationSeries([month({ month: 8 })])).toHaveLength(1);
  });

  it("aguenta uma projecao vazia", () => {
    expect(toSimulationSeries([])).toEqual([]);
  });
});

describe("firstNegativeMonth", () => {
  it("aponta o primeiro mes em que a compra derruba a sobra abaixo de zero", () => {
    const points = toSimulationSeries([
      month({ month: 8, remainingWithCents: 20_000 }),
      month({ month: 9, remainingWithCents: -5_000 }),
      month({ month: 10, remainingWithCents: -30_000 }),
    ]);

    expect(firstNegativeMonth(points)).toBe("set/26");
  });

  it("devolve nulo quando a sobra aguenta a projecao inteira", () => {
    const points = toSimulationSeries([
      month({ month: 8, remainingWithCents: 20_000 }),
      month({ month: 9, remainingWithCents: 10_000 }),
    ]);

    expect(firstNegativeMonth(points)).toBeNull();
  });

  it("nao considera zero como negativo", () => {
    const points = toSimulationSeries([month({ month: 8, remainingWithCents: 0 })]);
    expect(firstNegativeMonth(points)).toBeNull();
  });
});
