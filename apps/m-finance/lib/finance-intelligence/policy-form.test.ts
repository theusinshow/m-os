import { describe, expect, it } from "vitest";
import { policiesFromForm } from "@/lib/finance-intelligence/policy-form";

function form(values: Record<string, string>) {
  return { get: (name: string) => values[name] ?? null };
}

describe("policiesFromForm", () => {
  it("tudo vazio volta ao padrão", () => {
    const result = policiesFromForm(form({}));
    expect(result.set).toEqual([]);
    expect(result.reset).toHaveLength(6);
  });

  it("converte reais e percentuais para o formato do schema", () => {
    const result = policiesFromForm(
      form({
        minimumBuffer: "800,00",
        reliableMain: "100",
        reliableFreelance: "0",
        reliableExtra: "",
        horizonMonths: "6",
        sensitivity: "high",
        lookaheadMonths: "3",
        protectGoals: "on",
      }),
    );
    expect(result.errors).toEqual({});
    expect(result.set).toEqual([
      { key: "minimum_month_end_buffer", value: { amountCents: 80000 } },
      { key: "reliable_income_rules", value: { main: 1, freelance: 0, extra: 0 } },
      { key: "forecast_horizon_months", value: { months: 6 } },
      { key: "observer_sensitivity", value: { level: "high" } },
      { key: "safe_to_spend_policy", value: { lookaheadMonths: 3, protectGoals: true } },
    ]);
    expect(result.reset).toEqual(["max_installment_commitment"]);
  });

  it("valor fora do schema vira erro no campo", () => {
    const result = policiesFromForm(form({ reliableFreelance: "150", horizonMonths: "40", sensitivity: "max" }));
    expect(Object.keys(result.errors).sort()).toEqual(["horizonMonths", "reliableFreelance", "sensitivity"]);
  });
});
