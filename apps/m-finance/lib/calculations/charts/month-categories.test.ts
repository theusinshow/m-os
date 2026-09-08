import { describe, expect, it } from "vitest";
import { toMonthCategoryData } from "@/lib/calculations/charts/month-categories";

describe("toMonthCategoryData", () => {
  it("soma contas e faturas — a fatura é a maior despesa do mês e ficava de fora", () => {
    const data = toMonthCategoryData(
      [
        { amountCents: 700_00, categoryName: "Moto" },
        { amountCents: 300_00, categoryName: "Moradia" },
      ],
      [
        { amountCents: 1_800_00, name: "Nubank Pessoal" },
        { amountCents: 1_300_00, name: "Nubank PJ" },
      ],
    );

    expect(data).toEqual([
      { name: "Cartões", value: 3_100_00 },
      { name: "Moto", value: 700_00 },
      { name: "Moradia", value: 300_00 },
    ]);
  });

  it("conta sem categoria vira 'Sem categoria' em vez de sumir", () => {
    const data = toMonthCategoryData([{ amountCents: 90_00, categoryName: null }], []);

    expect(data).toEqual([{ name: "Sem categoria", value: 90_00 }]);
  });

  it("ordena da maior fatia para a menor", () => {
    const data = toMonthCategoryData(
      [
        { amountCents: 100_00, categoryName: "Lazer" },
        { amountCents: 900_00, categoryName: "Moradia" },
      ],
      [{ amountCents: 400_00, name: "Itaú" }],
    );

    expect(data.map((slice) => slice.name)).toEqual(["Moradia", "Cartões", "Lazer"]);
  });

  it("mês sem nada devolve lista vazia", () => {
    expect(toMonthCategoryData([], [])).toEqual([]);
  });
});
