import { describe, expect, it } from "vitest";
import { crossingDay, toBudgetBurndown } from "@/lib/calculations/charts/budget-burndown";

const LIMITE = 100_000;

describe("toBudgetBurndown", () => {
  it("devolve uma entrada por dia do mes", () => {
    expect(toBudgetBurndown([], LIMITE, 2026, 8)).toHaveLength(31);
    expect(toBudgetBurndown([], LIMITE, 2026, 2)).toHaveLength(28);
  });

  it("acumula e nunca decresce", () => {
    const points = toBudgetBurndown(
      [
        { dueDate: "2026-08-05", amountCents: 30_000 },
        { dueDate: "2026-08-15", amountCents: 25_000 },
      ],
      LIMITE,
      2026,
      8,
    );

    expect(points[3].spentCents).toBe(0);
    expect(points[4].spentCents).toBe(30_000);
    expect(points[13].spentCents).toBe(30_000);
    expect(points[14].spentCents).toBe(55_000);
    expect(points[30].spentCents).toBe(55_000);

    const decrescente = points.some(
      (point, index) => index > 0 && point.spentCents < points[index - 1].spentCents,
    );
    expect(decrescente).toBe(false);
  });

  it("repete o limite em todos os pontos, para virar linha reta", () => {
    const points = toBudgetBurndown([], LIMITE, 2026, 8);
    expect(points.every((point) => point.limitCents === LIMITE)).toBe(true);
  });

  it("ignora lancamento de outro mes", () => {
    const points = toBudgetBurndown(
      [{ dueDate: "2026-07-05", amountCents: 30_000 }],
      LIMITE,
      2026,
      8,
    );

    expect(points[30].spentCents).toBe(0);
  });
});

describe("crossingDay", () => {
  it("aponta o primeiro dia em que o acumulado passa do limite", () => {
    const points = toBudgetBurndown(
      [
        { dueDate: "2026-08-05", amountCents: 60_000 },
        { dueDate: "2026-08-12", amountCents: 50_000 },
      ],
      LIMITE,
      2026,
      8,
    );

    expect(crossingDay(points)).toBe(12);
  });

  it("devolve nulo quando o limite aguenta o mes", () => {
    const points = toBudgetBurndown(
      [{ dueDate: "2026-08-05", amountCents: 60_000 }],
      LIMITE,
      2026,
      8,
    );

    expect(crossingDay(points)).toBeNull();
  });

  it("nao considera cruzamento quando o gasto empata com o limite", () => {
    const points = toBudgetBurndown(
      [{ dueDate: "2026-08-05", amountCents: LIMITE }],
      LIMITE,
      2026,
      8,
    );

    expect(crossingDay(points)).toBeNull();
  });
});
