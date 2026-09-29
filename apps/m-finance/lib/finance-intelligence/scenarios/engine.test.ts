import { describe, expect, it } from "vitest";
import { baseSnapshot } from "@/lib/finance-intelligence/__fixtures__/snapshot";
import { compareScenarios, simulateScenario } from "@/lib/finance-intelligence/scenarios/engine";
import { scenarioChangeSchema, scenarioRequestSchema } from "@/lib/finance-intelligence/scenarios/types";

describe("simulateScenario — base", () => {
  it("sem mudança, cenário e base são iguais", () => {
    const result = simulateScenario(baseSnapshot(), [], 3);

    expect(result.kind).toBe("projection");
    expect(result.months.map((row) => row.key)).toEqual(["2026-09", "2026-10", "2026-11"]);
    for (const row of result.months) expect(row.deltaSafeToSpendCents).toBe(0);
    expect(result.totalAddedCommitmentCents).toBe(0);
    // Setembro: 5.500 confiável − 3.600 comprometido.
    expect(result.months[0].safeToSpendCents).toBe(190000);
  });

  it("não toca o snapshot", () => {
    const snapshot = baseSnapshot();
    const before = JSON.stringify(snapshot);
    simulateScenario(snapshot, [
      { type: "one_time_expense", label: "x", amountCents: 1000 },
      { type: "remove_income", incomeType: "all" },
    ]);
    expect(JSON.stringify(snapshot)).toBe(before);
  });
});

describe("compra", () => {
  it("R$ 6.000 em 10x: R$ 600 por mês por 10 meses", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [{ type: "installment_purchase", label: "Notebook", totalCents: 600000, installments: 10 }],
      12,
    );

    expect(result.months.slice(0, 10).every((row) => row.deltaSafeToSpendCents === -60000)).toBe(true);
    expect(result.months[10].deltaSafeToSpendCents).toBe(0);
    expect(result.totalAddedCommitmentCents).toBe(600000);
    expect(result.months[0].installmentsCents).toBe(20000 + 60000);
  });

  it("à vista pesa tudo no mês escolhido", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [{ type: "one_time_expense", label: "Notebook", amountCents: 600000, month: "2026-10" }],
      3,
    );
    expect(result.months.map((row) => row.deltaSafeToSpendCents)).toEqual([0, -600000, 0]);
    expect(result.worstMonth?.key).toBe("2026-10");
    expect(result.warnings.join(" ")).toContain("2026-10: o Safe-to-Spend do mês fica negativo");
  });

  it("entrada + parcelas, começando depois", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [
        {
          type: "installment_purchase",
          label: "Moto",
          totalCents: 1000000,
          installments: 4,
          downPaymentCents: 200000,
          startMonth: "2026-10",
        },
      ],
      6,
    );
    expect(result.months.map((row) => row.deltaSafeToSpendCents)).toEqual([
      0, -400000, -200000, -200000, -200000, 0,
    ]);
  });

  it("parcela além do horizonte vira aviso", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [{ type: "installment_purchase", label: "TV", totalCents: 120000, installments: 12 }],
      3,
    );
    expect(result.warnings.join(" ")).toContain("9 parcela(s)");
  });
});

describe("renda", () => {
  it("cenário composto: sem salário em dezembro, freelance de 1.500 e sem Claude Max", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [
        { type: "remove_income", incomeType: "main", fromMonth: "2026-12" },
        {
          type: "add_income",
          label: "Freelance",
          amountCents: 150000,
          incomeType: "freelance",
          recurring: true,
          fromMonth: "2026-12",
        },
        { type: "remove_expense", target: { kind: "subscription", id: "sub-claude" }, fromMonth: "2026-12" },
      ],
      6,
    );

    const dec = result.months.find((row) => row.key === "2026-12")!;
    // Freelance a receber conta 50% pela política padrão.
    expect(dec.reliableIncomeCents).toBe(75000);
    expect(dec.base.reliableIncomeCents).toBe(506667);
    const nov = result.months.find((row) => row.key === "2026-11")!;
    expect(nov.deltaSafeToSpendCents).toBe(0);
    expect(result.assumptions).toHaveLength(3);
  });

  it("percentual em todas as receitas", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [{ type: "change_income", incomeType: "all", percent: -10, fromMonth: "2026-10" }],
      2,
    );
    expect(result.months[1].incomeCents).toBe(Math.round(506667 * 0.9));
  });

  it("delta num tipo", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [{ type: "change_income", incomeType: "main", deltaCents: 50000, fromMonth: "2026-10", untilMonth: "2026-10" }],
      3,
    );
    expect(result.months[1].incomeCents).toBe(556667);
    expect(result.months[2].incomeCents).toBe(506667);
  });

  it("política temporária muda o peso da renda sem gravar nada", () => {
    const snapshot = baseSnapshot();
    const result = simulateScenario(
      snapshot,
      [{ type: "set_policy_temporary", reliableIncomeRules: { main: 1, freelance: 1, extra: 0 } }],
      1,
    );
    expect(result.months[0].reliableIncomeCents).toBe(600000);
    expect(snapshot.policies.reliableIncomeRules.freelance).toBe(0.5);
  });
});

describe("despesas", () => {
  it("remove conta pelo nome e avisa quando não acha", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [
        { type: "remove_expense", target: { kind: "bill", name: "internet" }, fromMonth: "2026-10" },
        { type: "remove_expense", target: { kind: "bill", name: "academia" } },
      ],
      2,
    );
    expect(result.months[1].deltaSafeToSpendCents).toBe(12000);
    expect(result.warnings.join(" ")).toContain('Nenhuma conta "academia"');
  });

  it("recorrente com fim", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [{ type: "recurring_expense", label: "Academia", amountCents: 10000, fromMonth: "2026-10", untilMonth: "2026-11" }],
      4,
    );
    expect(result.months.map((row) => row.deltaSafeToSpendCents)).toEqual([0, -10000, -10000, 0]);
  });

  it("antecipar parcelas move o que falta para o mês escolhido", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [{ type: "pay_off_installments", installmentId: "inst-tv" }],
      3,
    );
    expect(result.months.map((row) => row.deltaSafeToSpendCents)).toEqual([-40000, 20000, 20000]);
    expect(result.totalAddedCommitmentCents).toBe(0);
  });

  it("teto de parcelas vira aviso", () => {
    const result = simulateScenario(
      baseSnapshot(),
      [
        { type: "set_policy_temporary", maxInstallmentCommitmentCents: 50000 },
        { type: "installment_purchase", label: "Sofá", totalCents: 300000, installments: 6 },
      ],
      2,
    );
    expect(result.warnings.join(" ")).toContain("acima do teto");
  });
});

describe("compareScenarios", () => {
  it("à vista × 6x × esperar: ranking pelo pior mês", () => {
    const result = compareScenarios(
      baseSnapshot(),
      [
        { label: "À vista", changes: [{ type: "one_time_expense", label: "Mac", amountCents: 900000 }] },
        { label: "6x", changes: [{ type: "installment_purchase", label: "Mac", totalCents: 900000, installments: 6 }] },
      ],
      6,
    );
    expect(result.variants).toHaveLength(2);
    expect(result.rankingByWorstMonth[0]).toBe("6x");
  });
});

describe("schemas", () => {
  it("recusa campo desconhecido e tipo desconhecido", () => {
    expect(scenarioChangeSchema.safeParse({ type: "one_time_expense", label: "x", amountCents: 1, sql: "drop" }).success).toBe(false);
    expect(scenarioChangeSchema.safeParse({ type: "execute", label: "x" }).success).toBe(false);
  });

  it("exige changes OU variants, e horizonte até 24", () => {
    expect(scenarioRequestSchema.safeParse({}).success).toBe(false);
    expect(scenarioRequestSchema.safeParse({ changes: [], horizonMonths: 25 }).success).toBe(false);
    expect(scenarioRequestSchema.safeParse({ changes: [], horizonMonths: 24 }).success).toBe(true);
  });

  it("delta OU percentual; 'all' só com percentual", () => {
    expect(scenarioChangeSchema.safeParse({ type: "change_income", incomeType: "main" }).success).toBe(false);
    expect(scenarioChangeSchema.safeParse({ type: "change_income", incomeType: "all", deltaCents: 10 }).success).toBe(false);
  });
});
