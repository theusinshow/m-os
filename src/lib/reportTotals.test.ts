import { describe, expect, it } from "vitest";
import { groupBy } from "./reportTotals";

interface Row {
  projectId: string;
  roundedBillable: number;
  amount: number;
}

const rows: Row[] = [
  { projectId: "a", roundedBillable: 3_600, amount: 6_000 },
  { projectId: "b", roundedBillable: 7_200, amount: 20_000 },
  { projectId: "a", roundedBillable: 1_800, amount: 3_000 },
  { projectId: "c", roundedBillable: 900, amount: 1_000 },
];

describe("groupBy", () => {
  it("soma horas e valores da mesma chave", () => {
    const groups = groupBy(rows, (r) => r.projectId);
    const a = groups.find((g) => g.key === "a");
    expect(a).toEqual({ key: "a", seconds: 5_400, amount: 9_000 });
  });

  it("ordena do maior valor para o menor", () => {
    expect(groupBy(rows, (r) => r.projectId).map((g) => g.key)).toEqual([
      "b",
      "a",
      "c",
    ]);
  });

  it("a soma dos grupos e igual ao total das linhas", () => {
    const groups = groupBy(rows, (r) => r.projectId);
    const totalRows = rows.reduce((s, r) => s + r.amount, 0);
    const totalGroups = groups.reduce((s, g) => s + g.amount, 0);
    expect(totalGroups).toBe(totalRows);
  });

  it("lista vazia gera nenhum grupo", () => {
    expect(groupBy([], (r: Row) => r.projectId)).toEqual([]);
  });

  it("sessao nao-faturavel entra com zero sem sumir do grupo", () => {
    const groups = groupBy(
      [{ projectId: "a", roundedBillable: 0, amount: 0 }],
      (r) => r.projectId,
    );
    expect(groups).toEqual([{ key: "a", seconds: 0, amount: 0 }]);
  });
});
