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
        incomeEstimatedCents: 0,
        billsCents: 2_275_00,
        invoicesCents: 4_527_00,
        invoicesEstimatedCents: 0,
        committedCents: 6_802_00,
        remainingCents: -6_802_00,
        hasIncome: false,
        isEstimated: false,
        isCurrent: true,
      },
      {
        month: 10,
        year: 2026,
        incomeCents: 5_700_00,
        incomeEstimatedCents: 0,
        billsCents: 2_275_00,
        invoicesCents: 0,
        invoicesEstimatedCents: 0,
        committedCents: 2_275_00,
        remainingCents: 3_425_00,
        hasIncome: true,
        isEstimated: false,
        isCurrent: false,
      },
      {
        month: 11,
        year: 2026,
        incomeCents: 0,
        incomeEstimatedCents: 0,
        billsCents: 700_00,
        invoicesCents: 0,
        invoicesEstimatedCents: 0,
        committedCents: 700_00,
        remainingCents: -700_00,
        hasIncome: false,
        isEstimated: false,
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

  it("outubro sem fatura lançada conta a estimada, e a sobra deixa de ser folgada", () => {
    const [row] = buildMonthProjection(
      [
        {
          month: 10,
          year: 2026,
          incomeCents: 5_700_00,
          billsCents: 2_275_00,
          invoicesCents: 0,
          estimatedInvoicesCents: 4_527_00,
        },
      ],
      { month: 10, year: 2026 },
      1,
    );

    expect(row).toMatchObject({
      invoicesCents: 4_527_00,
      invoicesEstimatedCents: 4_527_00,
      committedCents: 6_802_00,
      remainingCents: -1_102_00,
      isEstimated: true,
    });
  });

  it("NF estimada dá resposta ao mês que ainda não teve nota", () => {
    const [row] = buildMonthProjection(
      [{ month: 11, year: 2026, incomeCents: 0, billsCents: 700_00, invoicesCents: 0, estimatedIncomeCents: 5_700_00 }],
      { month: 11, year: 2026 },
      1,
    );

    expect(row).toMatchObject({
      incomeCents: 5_700_00,
      incomeEstimatedCents: 5_700_00,
      remainingCents: 5_000_00,
      hasIncome: true,
      isEstimated: true,
    });
  });
});
