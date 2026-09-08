import { describe, expect, it } from "vitest";
import { pendingRecurrences } from "@/lib/recurrence";

const bill = (name: string) => ({ id: name, name, amountCents: 1_000, dueDate: "2026-09-10" });

describe("pendingRecurrences", () => {
  it("devolve as recorrências que ainda não existem no mês seguinte", () => {
    const pending = pendingRecurrences(
      [bill("Luz"), bill("Internet"), bill("Água")],
      [bill("Água")],
    );

    expect(pending.map((item) => item.name)).toEqual(["Luz", "Internet"]);
  });

  it("compara ignorando acento, caixa e espaço em volta", () => {
    const pending = pendingRecurrences([bill("  ÁGUA ")], [bill("agua")]);

    expect(pending).toEqual([]);
  });

  it("com o mês seguinte vazio, tudo está pendente", () => {
    const pending = pendingRecurrences([bill("Luz"), bill("MEI")], []);

    expect(pending).toHaveLength(2);
  });

  it("com tudo já lançado, não sobra nada — é o que esconde o card", () => {
    const pending = pendingRecurrences([bill("Luz")], [bill("Luz"), bill("Financiamento")]);

    expect(pending).toEqual([]);
  });
});
