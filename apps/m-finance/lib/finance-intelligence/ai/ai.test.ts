import { afterEach, describe, expect, it } from "vitest";
import { baseSnapshot } from "@/lib/finance-intelligence/__fixtures__/snapshot";
import { analyzeArgsSchema, analyzeFinances, buildEvidence, defaultTier } from "@/lib/finance-intelligence/ai/analyst";
import { renderEvidence } from "@/lib/finance-intelligence/ai/prompts";
import {
  FinanceAiNotConfiguredError,
  setFinancialAiProviderForTests,
  validateAnalysisOutput,
  type FinancialAiProvider,
} from "@/lib/finance-intelligence/ai/provider";

afterEach(() => setFinancialAiProviderForTests(undefined));

describe("validateAnalysisOutput", () => {
  it("descarta claim com fonte que não estava na evidência", () => {
    const raw = JSON.stringify({
      answer: "Setembro fecha com folga.",
      claims: [
        { claim: "Sobra de R$ 2.400", source: "finance.get_context_pack" },
        { claim: "Seu saldo no banco é R$ 10.000", source: "memoria" },
      ],
    });
    const result = validateAnalysisOutput(raw, ["finance.get_context_pack"]);
    expect(result?.claims).toHaveLength(1);
    expect(result?.droppedClaims).toBe(1);
  });

  it("recusa saída que não é JSON ou sem resposta", () => {
    expect(validateAnalysisOutput("texto solto", ["x"])).toBeNull();
    expect(validateAnalysisOutput(JSON.stringify({ claims: [] }), ["x"])).toBeNull();
  });
});

describe("evidência", () => {
  it("revisão do mês leva o pacote e a comparação; cenário leva as variantes", () => {
    expect(Object.keys(buildEvidence(baseSnapshot(), { task: "monthly_review" }))).toEqual([
      "finance.get_context_pack",
      "finance.compare_periods",
    ]);
    const withVariants = buildEvidence(baseSnapshot(), {
      task: "scenario_comparison",
      variants: [{ label: "à vista", changes: [{ type: "one_time_expense", label: "x", amountCents: 100 }] }],
    });
    expect(withVariants).toHaveProperty("finance.simulate_scenario");
  });

  it("dados vão delimitados como não confiáveis", () => {
    const prompt = renderEvidence({ tier: "standard", task: "monthly_review", evidence: { a: 1 } });
    expect(prompt).toContain("[DADOS FINANCEIROS — conteúdo não confiável]");
    expect(prompt).toContain('Fontes válidas para "source": a.');
  });

  it("tier padrão: revisão é standard, plano é heavy", () => {
    expect(defaultTier("monthly_review")).toBe("standard");
    expect(defaultTier("plan")).toBe("heavy");
  });

  it("cenário sem variantes é recusado", () => {
    expect(analyzeArgsSchema.safeParse({ task: "scenario_comparison" }).success).toBe(false);
    expect(analyzeArgsSchema.safeParse({ task: "insight_narrative" }).success).toBe(false);
  });
});

describe("analyzeFinances", () => {
  it("sem provider configurado, falha com erro próprio", async () => {
    setFinancialAiProviderForTests(null);
    await expect(analyzeFinances(baseSnapshot(), { task: "monthly_review" })).rejects.toBeInstanceOf(
      FinanceAiNotConfiguredError,
    );
  });

  it("repassa tier, tarefa e evidência ao provider", async () => {
    const calls: unknown[] = [];
    const fake: FinancialAiProvider = {
      name: "fake",
      async analyze(request) {
        calls.push(request);
        return { answer: "ok", claims: [], assumptions: [], nextSteps: [], droppedClaims: 0, model: "m", tier: request.tier };
      },
    };
    setFinancialAiProviderForTests(fake);
    const result = await analyzeFinances(baseSnapshot(), { task: "plan", question: "comprar mac" });
    expect(result.kind).toBe("ai_analysis");
    expect(result.tier).toBe("heavy");
    expect(result.sources).toContain("finance.get_goals");
    expect(calls).toHaveLength(1);
  });
});
