import { describe, expect, it } from "vitest";
import { baseSnapshot, withPolicies } from "@/lib/finance-intelligence/__fixtures__/snapshot";
import { materialityOf, runDetectors, type FinanceObservation } from "@/lib/finance-intelligence/observer/detectors";
import {
  COOLDOWN_DAYS,
  decideInsightUpdate,
  staleInsights,
  type LiveInsight,
} from "@/lib/finance-intelligence/observer/lifecycle";

const byDetector = (observations: FinanceObservation[], detector: string) =>
  observations.filter((row) => row.detector === detector);

describe("detectores", () => {
  const observations = runDetectors({ snapshot: baseSnapshot(), previousSafeToSpendCents: null });

  it("conta vencida há 10 dias é crítica", () => {
    const [overdue] = byDetector(observations, "overdue_commitment");
    expect(overdue).toMatchObject({
      dedupeKey: "overdue_commitment:bill:bill-light-sep",
      severity: "critical",
      facts: expect.objectContaining({ daysLate: 10 }),
    });
  });

  it("fatura que vence hoje é aviso", () => {
    const due = byDetector(observations, "bill_due_soon");
    expect(due.map((row) => row.dedupeKey)).toEqual(["bill_due_soon:invoice:inv-nu-sep:2026-09-15"]);
    expect(due[0].severity).toBe("warning");
  });

  it("fatura 25% acima da média dispara o pico", () => {
    const [spike] = byDetector(observations, "card_spending_spike");
    expect(spike.facts).toMatchObject({ amountCents: 150000, averageCents: 120000, deltaCents: 30000 });
  });

  it("teste grátis que cobra em 3 dias", () => {
    const load = byDetector(observations, "subscription_load");
    expect(load.map((row) => row.dedupeKey)).toEqual(["subscription_load:trial:sub-trial:2026-09-18"]);
  });

  it("o urgente vem primeiro", () => {
    expect(observations[0].severity).toBe("critical");
  });

  it("sem margem configurada não há pressão futura; com margem, outubro aparece", () => {
    expect(byDetector(observations, "future_month_pressure")).toHaveLength(0);
    const withBuffer = runDetectors({
      snapshot: baseSnapshot({ policies: withPolicies({ minimumMonthEndBufferCents: 200000 }) }),
      previousSafeToSpendCents: null,
    });
    expect(byDetector(withBuffer, "future_month_pressure")[0]).toMatchObject({
      dedupeKey: "future_month_pressure:2026-10",
      severity: "warning",
    });
  });

  it("parcelas acima do teto configurado", () => {
    const result = runDetectors({
      snapshot: baseSnapshot({ policies: withPolicies({ maxInstallmentCommitmentCents: 10000 }) }),
      previousSafeToSpendCents: null,
    });
    expect(byDetector(result, "installment_pressure")[0]).toMatchObject({
      severity: "warning",
      facts: expect.objectContaining({ installmentsCents: 20000, reliefMonth: "2026-12" }),
    });
  });

  it("NF não lançada depois do dia 10", () => {
    const snapshot = baseSnapshot();
    snapshot.incomes = snapshot.incomes.filter((row) => row.id !== "inc-nf-sep");
    const result = runDetectors({ snapshot, previousSafeToSpendCents: null });
    expect(byDetector(result, "income_missing")[0]?.dedupeKey).toBe("income_missing:nf:2026-09");
  });

  it("receita com data esperada vencida e não recebida", () => {
    const snapshot = baseSnapshot();
    snapshot.incomes[3] = { ...snapshot.incomes[3], expectedDate: "2026-09-05" };
    const result = runDetectors({ snapshot, previousSafeToSpendCents: null });
    expect(byDetector(result, "income_missing")[0]).toMatchObject({
      dedupeKey: "income_missing:inc-nf-sep",
      severity: "warning",
    });
  });

  it("queda material do Safe-to-Spend", () => {
    const result = runDetectors({ snapshot: baseSnapshot(), previousSafeToSpendCents: 300000 });
    expect(byDetector(result, "safe_to_spend_drop")[0].facts).toMatchObject({
      previousCents: 300000,
      currentCents: 190000,
      dropCents: 110000,
    });
    // Oscilação pequena não é insight.
    const small = runDetectors({ snapshot: baseSnapshot(), previousSafeToSpendCents: 200000 });
    expect(byDetector(small, "safe_to_spend_drop")).toHaveLength(0);
  });

  it("sensibilidade baixa cala o pico no limite", () => {
    const result = runDetectors({
      snapshot: baseSnapshot({ policies: withPolicies({ observerSensitivity: "low" }) }),
      previousSafeToSpendCents: null,
    });
    expect(byDetector(result, "card_spending_spike")).toHaveLength(0);
  });

  it("materialidade cresce com o valor, em escala", () => {
    expect(materialityOf(5000)).toBeLessThan(materialityOf(50000));
    expect(materialityOf(50)).toBe(0);
    expect(materialityOf(100_000_000)).toBe(100);
  });
});

describe("decideInsightUpdate — sem spam", () => {
  const now = new Date("2026-09-15T12:00:00Z");
  const observation: FinanceObservation = {
    detector: "card_spending_spike",
    dedupeKey: "k",
    severity: "warning",
    materialityScore: 50,
    title: "t",
    summary: "s",
    facts: {},
    entityRefs: [],
  };

  it("o mesmo evento em 10 ciclos: um insight, nove toques", () => {
    let live: LiveInsight | null = null;
    const actions: string[] = [];
    for (let cycle = 0; cycle < 10; cycle += 1) {
      const decision = decideInsightUpdate(live, observation, now);
      actions.push(decision.action);
      if (decision.action === "create") {
        live = {
          id: "i",
          dedupeKey: "k",
          status: "open",
          severity: observation.severity,
          materialityScore: observation.materialityScore,
          acknowledgedAt: null,
          lastSeenAt: now,
          facts: {},
        };
      }
    }
    expect(actions.filter((action) => action === "create")).toHaveLength(1);
    expect(actions.filter((action) => action === "touch")).toHaveLength(9);
  });

  const acknowledged = (daysAgo: number): LiveInsight => ({
    id: "i",
    dedupeKey: "k",
    status: "acknowledged",
    severity: "warning",
    materialityScore: 50,
    acknowledgedAt: new Date(now.getTime() - daysAgo * 86_400_000),
    lastSeenAt: now,
    facts: {},
  });

  it("reconhecido fica quieto dentro do cooldown", () => {
    expect(decideInsightUpdate(acknowledged(2), observation, now)).toEqual({ action: "touch" });
  });

  it("reconhecido volta depois do cooldown, sem push", () => {
    expect(decideInsightUpdate(acknowledged(COOLDOWN_DAYS), observation, now)).toEqual({
      action: "reopen",
      notify: false,
    });
  });

  it("severidade subindo reabre e, se crítico, avisa", () => {
    expect(decideInsightUpdate(acknowledged(1), { ...observation, severity: "critical" }, now)).toEqual({
      action: "reopen",
      notify: true,
    });
  });

  it("valor mudando além do limiar escala o aberto", () => {
    const open = { ...acknowledged(0), status: "open" as const };
    expect(decideInsightUpdate(open, { ...observation, materialityScore: 65 }, now).action).toBe("escalate");
    expect(decideInsightUpdate(open, { ...observation, materialityScore: 55 }, now).action).toBe("touch");
  });

  it("insight novo só pede push quando é crítico", () => {
    expect(decideInsightUpdate(null, observation, now)).toEqual({ action: "create", notify: false });
    expect(decideInsightUpdate(null, { ...observation, severity: "critical" }, now)).toEqual({
      action: "create",
      notify: true,
    });
  });

  it("o que parou de disparar resolve; vencimento que passou expira", () => {
    const base = acknowledged(0);
    const stale = staleInsights(
      [
        { ...base, id: "a", dedupeKey: "a", type: "card_spending_spike" },
        { ...base, id: "b", dedupeKey: "b", type: "bill_due_soon" },
        { ...base, id: "c", dedupeKey: "c", type: "overdue_commitment" },
      ],
      new Set(["c"]),
    );
    expect(stale).toEqual([
      { id: "a", status: "resolved" },
      { id: "b", status: "expired" },
    ]);
  });
});
