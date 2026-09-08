import { describe, expect, it } from "vitest";
import { splitPersonalBusiness, summarizeInstallments } from "@/lib/calculations/commitments";

describe("splitPersonalBusiness", () => {
  it("separa a fatura PJ do resto do mês", () => {
    const split = splitPersonalBusiness([
      { amountCents: 1_800_00, cardType: "personal" },
      { amountCents: 1_027_00, cardType: "personal" },
      { amountCents: 400_00, cardType: "personal" },
      { amountCents: 1_300_00, cardType: "business" },
    ]);

    expect(split).toEqual({
      personalCents: 3_227_00,
      businessCents: 1_300_00,
      totalCents: 4_527_00,
      businessPercent: 29,
    });
  });

  it("sem cartão PJ, o percentual é zero e não NaN", () => {
    const split = splitPersonalBusiness([{ amountCents: 500_00, cardType: "personal" }]);

    expect(split.businessCents).toBe(0);
    expect(split.businessPercent).toBe(0);
  });

  it("mês sem fatura nenhuma não divide por zero", () => {
    expect(splitPersonalBusiness([])).toEqual({
      personalCents: 0,
      businessCents: 0,
      totalCents: 0,
      businessPercent: 0,
    });
  });
});

describe("summarizeInstallments", () => {
  const parcela = (n: number, month: string) => ({
    name: "Financiamento",
    amountCents: 700_00,
    seriesId: "moto",
    seriesNumber: n,
    seriesTotal: 22,
    dueDate: month,
    status: "pending" as const,
  });

  it("agrupa por série e soma o que ainda falta pagar", () => {
    const series = summarizeInstallments([
      { ...parcela(1, "2026-09-03"), status: "paid" },
      parcela(2, "2026-10-03"),
      parcela(3, "2026-11-03"),
    ]);

    expect(series).toEqual([
      {
        seriesId: "moto",
        name: "Financiamento",
        installmentCents: 700_00,
        paidCount: 1,
        remainingCount: 21,
        remainingCents: 14_700_00,
        seriesTotal: 22,
        lastDueDate: "2026-11-03",
      },
    ]);
  });

  it("ignora contas que não são parcelamento", () => {
    expect(
      summarizeInstallments([
        {
          name: "Luz",
          amountCents: 700_00,
          seriesId: null,
          seriesNumber: null,
          seriesTotal: null,
          dueDate: "2026-09-30",
          status: "pending",
        },
      ]),
    ).toEqual([]);
  });

  it("série quitada some da lista — não é compromisso futuro", () => {
    expect(
      summarizeInstallments([
        { ...parcela(1, "2026-09-03"), seriesTotal: 2, status: "paid" },
        { ...parcela(2, "2026-10-03"), seriesTotal: 2, status: "paid" },
      ]),
    ).toEqual([]);
  });
});
