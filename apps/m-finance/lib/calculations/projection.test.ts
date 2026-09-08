import { describe, expect, it } from "vitest";
import { buildMonthProjection } from "@/lib/calculations/projection";

describe("buildMonthProjection", () => {
  const totals = [
    { month: 9, year: 2026, incomeCents: 0, billsCents: 2_275_00, invoicesCents: 4_527_00 },
    { month: 10, year: 2026, incomeCents: 5_700_00, billsCents: 2_275_00, invoicesCents: 0 },
    { month: 11, year: 2026, incomeCents: 0, billsCents: 700_00, invoicesCents: 0 },
  ];

  it("mostra o mês atual e os seguintes, com a sobra de cada um", () => {
    const rows = buildMonthProjection(totals, { month: 9, year: 2026 }, 3);

    expect(rows).toEqual([
      {
        month: 9,
        year: 2026,
        incomeCents: 0,
        committedCents: 6_802_00,
        remainingCents: -6_802_00,
        hasIncome: false,
        isCurrent: true,
      },
      {
        month: 10,
        year: 2026,
        incomeCents: 5_700_00,
        committedCents: 2_275_00,
        remainingCents: 3_425_00,
        hasIncome: true,
        isCurrent: false,
      },
      {
        month: 11,
        year: 2026,
        incomeCents: 0,
        committedCents: 700_00,
        remainingCents: -700_00,
        hasIncome: false,
        isCurrent: false,
      },
    ]);
  });

  it("não olha para trás", () => {
    const rows = buildMonthProjection(
      [{ month: 8, year: 2026, incomeCents: 100, billsCents: 0, invoicesCents: 0 }, ...totals],
      { month: 9, year: 2026 },
      3,
    );

    expect(rows.every((row) => row.year > 2026 || row.month >= 9)).toBe(true);
  });

  it("para na quantidade pedida", () => {
    expect(buildMonthProjection(totals, { month: 9, year: 2026 }, 2)).toHaveLength(2);
  });

  it("mês sem movimento nenhum não entra na projeção", () => {
    const rows = buildMonthProjection(
      [{ month: 12, year: 2026, incomeCents: 0, billsCents: 0, invoicesCents: 0 }],
      { month: 9, year: 2026 },
      6,
    );

    expect(rows).toEqual([]);
  });
});
