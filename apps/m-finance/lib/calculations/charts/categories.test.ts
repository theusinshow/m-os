import { describe, expect, it } from "vitest";
import { categoryLabelReserve, toCategorySlices } from "@/lib/calculations/charts/categories";

describe("toCategorySlices", () => {
  it("ordena da maior para a menor e descarta valor zero ou negativo", () => {
    const slices = toCategorySlices([
      { name: "Lazer", value: 1_000 },
      { name: "Casa", value: 5_000 },
      { name: "Vazia", value: 0 },
      { name: "Mercado", value: 4_000 },
    ]);

    expect(slices.map((slice) => slice.name)).toEqual(["Casa", "Mercado", "Lazer"]);
  });

  it("agrupa da oitava categoria em diante como Outras", () => {
    const data = Array.from({ length: 12 }, (_, index) => ({
      name: `Cat ${index}`,
      value: (12 - index) * 1_000,
    }));

    const slices = toCategorySlices(data);

    expect(slices).toHaveLength(8);
    expect(slices[7].name).toBe("Outras");
    // As cinco menores: 5000 + 4000 + 3000 + 2000 + 1000.
    expect(slices[7].value).toBe(15_000);
  });

  it("nao cria Outras quando cabe tudo", () => {
    const slices = toCategorySlices([
      { name: "Casa", value: 2_000 },
      { name: "Lazer", value: 1_000 },
    ]);

    expect(slices.map((slice) => slice.name)).toEqual(["Casa", "Lazer"]);
  });

  it("distribui os percentuais de modo que somem exatamente 100", () => {
    const slices = toCategorySlices([
      { name: "A", value: 1 },
      { name: "B", value: 1 },
      { name: "C", value: 1 },
    ]);

    expect(slices.reduce((total, slice) => total + slice.percent, 0)).toBe(100);
    expect(slices.map((slice) => slice.percent).sort()).toEqual([33, 33, 34]);
  });

  it("devolve lista vazia quando nao ha valor positivo", () => {
    expect(toCategorySlices([])).toEqual([]);
    expect(toCategorySlices([{ name: "Zerada", value: 0 }])).toEqual([]);
  });

  it("respeita um teto customizado", () => {
    const data = Array.from({ length: 5 }, (_, index) => ({
      name: `Cat ${index}`,
      value: 1_000,
    }));

    const slices = toCategorySlices(data, 3);

    expect(slices).toHaveLength(3);
    expect(slices[2].name).toBe("Outras");
    expect(slices[2].value).toBe(3_000);
  });
});

describe("categoryLabelReserve", () => {
  it("reserva mais espaco para o rotulo mais largo da serie", () => {
    const curto = categoryLabelReserve(["R$ 89,90 · 1%"]);
    const largo = categoryLabelReserve(["R$ 89,90 · 1%", "R$ 12.450,00 · 100%"]);

    expect(largo).toBeGreaterThan(curto);
  });

  it("cabe o rotulo que a margem fixa de 96px cortava a 375px", () => {
    // `R$ 2.450,00 · 36%` mede ~102px com a folga, e a margem antiga deixava 88.
    expect(categoryLabelReserve(["R$ 2.450,00 · 36%"])).toBeGreaterThanOrEqual(102);
  });

  it("nao reserva nada sem rotulo", () => {
    expect(categoryLabelReserve([])).toBe(0);
  });
});
