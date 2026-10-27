import { describe, expect, it } from "vitest";
import {
  averageOfRecent,
  forecastCardMonth,
  forecastMainIncome,
  type ForecastCard,
  type ForecastInvoice,
} from "@/lib/calculations/forecast";

const setembro = { month: 9, year: 2026 };
const outubro = { month: 10, year: 2026 };
const novembro = { month: 11, year: 2026 };

const nubank: ForecastCard = {
  id: "nu",
  name: "Nubank",
  cardType: "personal",
  dueDay: 10,
  isActive: true,
};

const fatura = (
  cardId: string,
  parts: { month: number; year: number },
  amountCents: number,
  extra: Partial<ForecastInvoice> = {},
): ForecastInvoice => ({
  id: `${cardId}-${parts.year}-${parts.month}`,
  cardId,
  ...parts,
  amountCents,
  dueDate: `${parts.year}-${String(parts.month).padStart(2, "0")}-10`,
  status: "pending",
  ...extra,
});

describe("averageOfRecent", () => {
  it("faz a média só dos meses anteriores ao alvo, até a janela", () => {
    const history = [
      { month: 6, year: 2026, amountCents: 100_00 },
      { month: 7, year: 2026, amountCents: 200_00 },
      { month: 8, year: 2026, amountCents: 300_00 },
      { month: 9, year: 2026, amountCents: 400_00 },
      { month: 10, year: 2026, amountCents: 9_999_00 },
    ];

    expect(averageOfRecent(history, outubro, 3)).toEqual({ averageCents: 300_00, count: 3 });
  });

  it("sem histórico não inventa número", () => {
    expect(averageOfRecent([], outubro)).toEqual({ averageCents: 0, count: 0 });
  });

  it("vira o ano sem se perder", () => {
    const history = [{ month: 12, year: 2026, amountCents: 500_00 }];
    expect(averageOfRecent(history, { month: 1, year: 2027 })).toEqual({
      averageCents: 500_00,
      count: 1,
    });
  });
});

describe("forecastCardMonth", () => {
  it("outubro sem fatura lançada mostra a estimada, não o vazio", () => {
    const [line] = forecastCardMonth(
      { cards: [nubank], invoices: [fatura("nu", setembro, 1_800_00, { status: "paid" })], installments: [] },
      outubro,
      setembro,
    );

    expect(line).toMatchObject({
      source: "estimated",
      amountCents: 1_800_00,
      basisCount: 1,
      invoice: null,
      dueDate: "2026-10-10",
    });
  });

  it("fatura lançada manda, mesmo menor que a média", () => {
    const [line] = forecastCardMonth(
      {
        cards: [nubank],
        invoices: [fatura("nu", setembro, 1_800_00), fatura("nu", outubro, 300_00)],
        installments: [],
      },
      outubro,
      setembro,
    );

    expect(line.source).toBe("actual");
    expect(line.amountCents).toBe(300_00);
    expect(line.invoice?.id).toBe("nu-2026-10");
  });

  it("parcelas já lançadas são o piso da estimativa", () => {
    const [line] = forecastCardMonth(
      {
        cards: [nubank],
        invoices: [fatura("nu", setembro, 400_00)],
        installments: [{ cardId: "nu", ...novembro, amountCents: 650_00 }],
      },
      novembro,
      setembro,
    );

    expect(line).toMatchObject({ source: "estimated", amountCents: 650_00, installmentsCents: 650_00 });
  });

  it("mês passado sem fatura não ganha estimativa: não houve fatura", () => {
    const [line] = forecastCardMonth(
      { cards: [nubank], invoices: [fatura("nu", setembro, 1_800_00)], installments: [] },
      { month: 8, year: 2026 },
      setembro,
    );

    expect(line).toMatchObject({ source: "none", amountCents: 0 });
  });

  it("cartão sem histórico nem parcelas fica sem resposta", () => {
    const [line] = forecastCardMonth({ cards: [nubank], invoices: [], installments: [] }, outubro, setembro);
    expect(line).toMatchObject({ source: "none", amountCents: 0 });
  });

  it("cartão inativo só aparece se tem fatura de verdade no mês", () => {
    const inativo = { ...nubank, id: "velho", isActive: false };
    const semFatura = forecastCardMonth(
      { cards: [inativo], invoices: [fatura("velho", setembro, 100_00)], installments: [] },
      outubro,
      setembro,
    );
    const comFatura = forecastCardMonth(
      { cards: [inativo], invoices: [fatura("velho", outubro, 100_00)], installments: [] },
      outubro,
      setembro,
    );

    expect(semFatura).toEqual([]);
    expect(comFatura).toHaveLength(1);
  });
});

describe("forecastMainIncome", () => {
  const nf = (parts: { month: number; year: number }, amountCents: number, incomeType = "main") => ({
    ...parts,
    amountCents,
    incomeType: incomeType as "main" | "extra" | "freelance",
  });

  it("NF lançada no mês é a resposta", () => {
    expect(forecastMainIncome([nf(outubro, 5_700_00)], outubro, setembro)).toEqual({
      source: "actual",
      amountCents: 5_700_00,
      basisCount: 0,
    });
  });

  it("mês à frente sem NF usa a média das NFs anteriores", () => {
    const incomes = [nf(setembro, 5_000_00), nf(outubro, 6_000_00)];
    expect(forecastMainIncome(incomes, novembro, setembro)).toEqual({
      source: "estimated",
      amountCents: 5_500_00,
      basisCount: 2,
    });
  });

  it("freelance não entra na média da NF", () => {
    const incomes = [nf(setembro, 5_000_00), nf(setembro, 2_000_00, "freelance")];
    expect(forecastMainIncome(incomes, outubro, setembro).amountCents).toBe(5_000_00);
  });

  it("duas NFs no mesmo mês somam antes da média", () => {
    const incomes = [nf(setembro, 3_000_00), nf(setembro, 2_000_00)];
    expect(forecastMainIncome(incomes, outubro, setembro).amountCents).toBe(5_000_00);
  });

  it("mês passado sem NF fica sem resposta", () => {
    expect(forecastMainIncome([nf(outubro, 5_700_00)], { month: 8, year: 2026 }, setembro).source).toBe(
      "none",
    );
  });
});
