import { describe, expect, it } from "vitest";
import { baseSnapshot, withPolicies } from "@/lib/finance-intelligence/__fixtures__/snapshot";
import { futurePressureReserve, safeToSpend } from "@/lib/finance-intelligence/kernel/safe-to-spend";

function identityHolds(result: ReturnType<typeof safeToSpend>) {
  const deductions = result.deductions.reduce((sum, item) => sum + item.amountCents, 0);
  return result.accountingRemainingCents - deductions === result.safeToSpendCents;
}

describe("futurePressureReserve", () => {
  const month = (key: string, reliableRemainingCents: number, hasIncome = true) => ({
    key,
    reliableRemainingCents,
    hasIncome,
  });

  it("saldo corrido: um mês folgado cobre o apertado seguinte", () => {
    const result = futurePressureReserve([month("a", 50000), month("b", -30000)], 0);
    expect(result.reserveCents).toBe(0);
    expect(result.shortfalls).toEqual([{ key: "b", shortfallCents: 30000 }]);
  });

  it("a reserva é o ponto mais baixo do saldo", () => {
    const result = futurePressureReserve(
      [month("a", 10000), month("b", -30000), month("c", 10000), month("d", -20000)],
      0,
    );
    // corrido: 100, -200, -100, -300
    expect(result.reserveCents).toBe(30000);
  });

  it("a margem mínima vale em cada mês", () => {
    const result = futurePressureReserve([month("a", 50000)], 80000);
    expect(result.reserveCents).toBe(30000);
  });

  it("mês sem renda conhecida fica fora da conta", () => {
    const result = futurePressureReserve([month("a", -500000, false)], 0);
    expect(result.reserveCents).toBe(0);
  });
});

describe("safeToSpend", () => {
  it("padrão: só a confiabilidade da receita sai da sobra", () => {
    const result = safeToSpend(baseSnapshot());

    expect(result.accountingRemainingCents).toBe(240000);
    // freelance de 1.000 conta 50%.
    expect(result.deductions).toEqual([
      expect.objectContaining({ reason: "income_reliability", amountCents: 50000 }),
    ]);
    expect(result.safeToSpendCents).toBe(190000);
    expect(identityHolds(result)).toBe(true);
    expect(result.assumptions.join(" ")).toContain("Nenhuma margem mínima configurada");
  });

  it("margem mínima e pressão de outubro saem, e a identidade se mantém", () => {
    const snapshot = baseSnapshot({
      policies: withPolicies({ minimumMonthEndBufferCents: 200000 }),
    });
    const result = safeToSpend(snapshot);

    // Outubro: 5.066,67 − 3.286,67 = 1.780 de sobra confiável; menos 2.000 de
    // margem = −220. Novembro cobre depois, mas o ponto mais baixo é −220.
    expect(result.deductions.map((item) => item.reason)).toEqual([
      "income_reliability",
      "minimum_buffer",
      "future_pressure",
    ]);
    expect(result.deductions[2].amountCents).toBe(22000);
    expect(result.safeToSpendCents).toBe(240000 - 50000 - 200000 - 22000);
    expect(result.status).toBe("negative");
    expect(identityHolds(result)).toBe(true);
  });

  it("metas protegidas entram só quando a política pede", () => {
    const off = safeToSpend(baseSnapshot());
    expect(off.deductions.some((item) => item.reason === "goal_reserve")).toBe(false);

    const on = safeToSpend(
      baseSnapshot({
        policies: withPolicies({ safeToSpend: { lookaheadMonths: 2, protectGoals: true } }),
      }),
    );
    expect(on.deductions.find((item) => item.reason === "goal_reserve")?.amountCents).toBe(150000);
    expect(identityHolds(on)).toBe(true);
  });

  it("lookahead zero não olha para frente e diz isso", () => {
    const result = safeToSpend(
      baseSnapshot({
        policies: withPolicies({
          minimumMonthEndBufferCents: 200000,
          safeToSpend: { lookaheadMonths: 0, protectGoals: false },
        }),
      }),
    );
    expect(result.deductions.some((item) => item.reason === "future_pressure")).toBe(false);
    expect(result.assumptions.join(" ")).toContain("lookahead 0");
  });

  it("sem receita nenhuma o status é desconhecido, não 'negativo'", () => {
    const result = safeToSpend(baseSnapshot({ incomes: [] }));
    expect(result.status).toBe("unknown");
    expect(identityHolds(result)).toBe(true);
  });

  it("mês à frente sem renda vira premissa, não dedução", () => {
    const snapshot = baseSnapshot();
    // Sem histórico de NF, outubro e novembro não têm estimativa.
    snapshot.incomes = snapshot.incomes.filter((row) => row.month === 9);
    snapshot.incomes = snapshot.incomes.map((row) => ({ ...row, incomeType: "extra" as const }));
    const result = safeToSpend(snapshot);
    expect(result.deductions.some((item) => item.reason === "future_pressure")).toBe(false);
    expect(result.assumptions.join(" ")).toContain("Sem receita conhecida em 2026-10, 2026-11");
  });

  it("é determinístico", () => {
    expect(safeToSpend(baseSnapshot())).toEqual(safeToSpend(baseSnapshot()));
  });
});
