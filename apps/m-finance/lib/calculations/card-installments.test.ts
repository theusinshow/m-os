import { describe, expect, it } from "vitest";
import { summarizeCardInstallments } from "@/lib/calculations/card-installments";

const parcela = (n: number, month: number, extra = {}) => ({
  description: "Notebook",
  amountCents: 400_00,
  installmentId: "note",
  installmentNumber: n,
  installmentTotal: 10,
  month,
  year: 2026,
  ...extra,
});

describe("summarizeCardInstallments", () => {
  it("mostra em que parcela está e quanto ainda falta do parcelamento", () => {
    const series = summarizeCardInstallments(
      [parcela(1, 9), parcela(2, 10), parcela(3, 11)],
      { month: 9, year: 2026 },
    );

    expect(series).toEqual([
      {
        installmentId: "note",
        description: "Notebook",
        installmentCents: 400_00,
        currentNumber: 1,
        installmentTotal: 10,
        remainingCount: 9,
        remainingCents: 3_600_00,
        lastMonth: { month: 6, year: 2027 },
      },
    ]);
  });

  it("parcelamento que já passou do mês ativo não aparece", () => {
    const series = summarizeCardInstallments(
      [parcela(1, 6), parcela(2, 7)],
      { month: 9, year: 2026 },
    );

    expect(series).toEqual([]);
  });

  it("última parcela no mês ativo não tem nada a vencer depois", () => {
    const series = summarizeCardInstallments(
      [parcela(10, 9, { installmentNumber: 10 })],
      { month: 9, year: 2026 },
    );

    expect(series[0].remainingCount).toBe(0);
    expect(series[0].remainingCents).toBe(0);
  });

  it("compra à vista não é parcelamento", () => {
    expect(
      summarizeCardInstallments(
        [
          {
            description: "Mercado",
            amountCents: 90_00,
            installmentId: null,
            installmentNumber: null,
            installmentTotal: null,
            month: 9,
            year: 2026,
          },
        ],
        { month: 9, year: 2026 },
      ),
    ).toEqual([]);
  });

  it("ordena pelo que ainda pesa mais", () => {
    const series = summarizeCardInstallments(
      [
        parcela(1, 9, { installmentId: "a", description: "Leve", amountCents: 50_00 }),
        parcela(1, 9, { installmentId: "b", description: "Pesado", amountCents: 900_00 }),
      ],
      { month: 9, year: 2026 },
    );

    expect(series.map((item) => item.description)).toEqual(["Pesado", "Leve"]);
  });
});
