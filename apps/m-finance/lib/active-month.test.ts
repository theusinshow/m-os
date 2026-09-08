import { describe, expect, it } from "vitest";
import { visibleSwitcherMonths } from "@/lib/active-month";

const CURRENT = { month: 9, year: 2026 };

describe("visibleSwitcherMonths", () => {
  it("esconde mês passado sem nada dentro", () => {
    const visible = visibleSwitcherMonths(
      [
        { month: 6, year: 2026 },
        { month: 7, year: 2026 },
        { month: 9, year: 2026 },
      ],
      new Set(["2026-09"]),
      CURRENT,
    );

    expect(visible).toEqual([{ month: 9, year: 2026 }]);
  });

  it("mantém mês passado que tem lançamento", () => {
    const visible = visibleSwitcherMonths(
      [
        { month: 8, year: 2026 },
        { month: 9, year: 2026 },
      ],
      new Set(["2026-08", "2026-09"]),
      CURRENT,
    );

    expect(visible).toHaveLength(2);
  });

  it("mantém o futuro mesmo vazio — é onde o mês novo nasce", () => {
    const visible = visibleSwitcherMonths(
      [
        { month: 9, year: 2026 },
        { month: 12, year: 2026 },
        { month: 1, year: 2027 },
      ],
      new Set(["2026-09"]),
      CURRENT,
    );

    expect(visible).toHaveLength(3);
  });

  it("nunca esconde o mês atual, mesmo vazio", () => {
    expect(visibleSwitcherMonths([{ month: 9, year: 2026 }], new Set(), CURRENT)).toEqual([
      { month: 9, year: 2026 },
    ]);
  });
});
