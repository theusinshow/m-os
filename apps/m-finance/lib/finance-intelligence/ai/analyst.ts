import { z } from "zod";
import { env } from "@/lib/env";
import {
  ANALYSIS_TASKS,
  FinanceAiNotConfiguredError,
  getFinancialAiProvider,
  type AnalysisTask,
  type AnalysisTier,
} from "@/lib/finance-intelligence/ai/provider";
import { addMonths } from "@/lib/finance-intelligence/dates";
import { futureCommitments } from "@/lib/finance-intelligence/kernel/commitments";
import { comparePeriods } from "@/lib/finance-intelligence/kernel/compare";
import { contextPack, type PackInsight } from "@/lib/finance-intelligence/kernel/context-pack";
import { goalSummary } from "@/lib/finance-intelligence/kernel/goals";
import type { FinanceObservation } from "@/lib/finance-intelligence/observer/detectors";
import { compareScenarios } from "@/lib/finance-intelligence/scenarios/engine";
import { scenarioChangeSchema } from "@/lib/finance-intelligence/scenarios/types";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";

export const analyzeArgsSchema = z
  .object({
    task: z.enum(ANALYSIS_TASKS).exclude(["insight_narrative"]),
    question: z.string().trim().max(500).optional(),
    tier: z.enum(["standard", "heavy"]).optional(),
    horizonMonths: z.number().int().min(1).max(24).optional(),
    variants: z
      .array(z.object({ label: z.string().trim().min(1).max(80), changes: z.array(scenarioChangeSchema).max(12) }).strict())
      .min(1)
      .max(5)
      .optional(),
  })
  .strict()
  .refine((value) => value.task !== "scenario_comparison" || value.variants !== undefined, {
    message: "scenario_comparison exige variants.",
  });

export type AnalyzeArgs = z.infer<typeof analyzeArgsSchema>;

/** Revisão do mês é leitura; plano, cenário e trade-off pedem o tier pesado. */
export function defaultTier(task: AnalysisTask): AnalysisTier {
  return task === "monthly_review" || task === "insight_narrative" ? "standard" : "heavy";
}

/**
 * O pacote de evidência de cada tarefa, montado pelo kernel. É isto — e só
 * isto — que a LLM enxerga; os nomes das chaves são as fontes que ela pode
 * citar.
 */
export function buildEvidence(snapshot: FinanceSnapshot, args: AnalyzeArgs, insights: PackInsight[] = []) {
  const evidence: Record<string, unknown> = {
    "finance.get_context_pack": contextPack(snapshot, insights),
  };
  const horizon = args.horizonMonths ?? snapshot.policies.forecastHorizonMonths;

  if (args.task === "monthly_review") {
    evidence["finance.compare_periods"] = comparePeriods(snapshot, addMonths(snapshot.current, -1), snapshot.current);
  }
  if (args.task === "plan" || args.task === "tradeoffs") {
    evidence["finance.get_future_commitments"] = futureCommitments(snapshot, horizon);
    evidence["finance.get_goals"] = goalSummary(snapshot);
  }
  if (args.variants) {
    evidence["finance.simulate_scenario"] = compareScenarios(snapshot, args.variants, horizon);
  }
  return evidence;
}

export async function analyzeFinances(snapshot: FinanceSnapshot, args: AnalyzeArgs, insights: PackInsight[] = []) {
  const provider = getFinancialAiProvider();
  if (!provider) throw new FinanceAiNotConfiguredError();
  const tier = args.tier ?? defaultTier(args.task);
  const evidence = buildEvidence(snapshot, args, insights);
  const result = await provider.analyze({ tier, task: args.task, question: args.question, evidence });
  return {
    kind: "ai_analysis" as const,
    // O que o modelo diz é interpretação da evidência, não fato novo.
    disclaimer: "Interpretação gerada por IA sobre os dados determinísticos listados em sources.",
    sources: Object.keys(evidence),
    ...result,
  };
}

/**
 * Narrativa opcional de um insight que o detector JÁ achou material. Sem
 * provider, sem flag, ou com erro, devolve `null` e o template vale.
 */
export async function narrateObservation(observation: FinanceObservation): Promise<string | null> {
  if (!env.financeAiNarrateInsights) return null;
  const provider = getFinancialAiProvider();
  if (!provider) return null;
  try {
    const result = await provider.analyze({
      tier: "standard",
      task: "insight_narrative",
      evidence: {
        observation: { detector: observation.detector, title: observation.title, summary: observation.summary, facts: observation.facts },
      },
    });
    return result.answer.slice(0, 600);
  } catch {
    return null;
  }
}
