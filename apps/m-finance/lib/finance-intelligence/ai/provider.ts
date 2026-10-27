import OpenAI from "openai";
import { z } from "zod";
import { env, isFinanceAiConfigured } from "@/lib/env";
import { FINANCIAL_SYSTEM_CONTRACT, renderEvidence } from "@/lib/finance-intelligence/ai/prompts";

export type AnalysisTier = "standard" | "heavy";
export const ANALYSIS_TASKS = [
  "monthly_review",
  "plan",
  "scenario_comparison",
  "tradeoffs",
  "insight_narrative",
] as const;
export type AnalysisTask = (typeof ANALYSIS_TASKS)[number];

export type FinancialAnalysisRequest = {
  tier: AnalysisTier;
  task: AnalysisTask;
  /** A pergunta do dono, quando houver. É dado, não instrução. */
  question?: string;
  /** O pacote de evidência determinístico, por nome de ferramenta. */
  evidence: Record<string, unknown>;
};

export type FinancialAnalysisResult = {
  answer: string;
  claims: { claim: string; source: string }[];
  assumptions: string[];
  nextSteps: string[];
  /** Claims descartados por citarem fonte que não estava na evidência. */
  droppedClaims: number;
  model: string;
  tier: AnalysisTier;
};

/**
 * A fronteira com qualquer LLM financeira. Domínio nenhum importa SDK de
 * provedor — importa isto.
 */
export interface FinancialAiProvider {
  readonly name: string;
  analyze(request: FinancialAnalysisRequest): Promise<FinancialAnalysisResult>;
}

export class FinanceAiNotConfiguredError extends Error {
  constructor() {
    super("IA financeira não configurada.");
  }
}

const outputSchema = z.object({
  answer: z.string().trim().min(1).max(4000),
  claims: z
    .array(z.object({ claim: z.string().trim().min(1).max(400), source: z.string().trim().min(1).max(80) }))
    .max(20)
    .default([]),
  assumptions: z.array(z.string().trim().min(1).max(300)).max(10).default([]),
  nextSteps: z.array(z.string().trim().min(1).max(300)).max(6).default([]),
});

/**
 * Valida a saída do modelo contra a evidência que ELE recebeu.
 *
 * Um claim com fonte que não estava no pacote é um número sem lastro — foi o
 * modelo lembrando de outra conversa, ou inventando. Ele cai, e a contagem do
 * que caiu volta junto para quem audita.
 */
export function validateAnalysisOutput(raw: string, sources: string[]) {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  const result = outputSchema.safeParse(parsed);
  if (!result.success) return null;
  const allowed = new Set(sources);
  const claims = result.data.claims.filter((claim) => allowed.has(claim.source));
  return {
    answer: result.data.answer,
    claims,
    assumptions: result.data.assumptions,
    nextSteps: result.data.nextSteps,
    droppedClaims: result.data.claims.length - claims.length,
  };
}

/** Qualquer endpoint compatível com a API da OpenAI (DeepSeek, OpenAI, OpenRouter…). */
export class OpenAiCompatibleProvider implements FinancialAiProvider {
  readonly name = "openai-compatible";
  private client: OpenAI;

  constructor(
    private readonly config: {
      baseUrl: string;
      apiKey: string;
      standardModel: string;
      heavyModel: string;
      timeoutMs: number;
    },
  ) {
    this.client = new OpenAI({ apiKey: config.apiKey, baseURL: config.baseUrl, timeout: config.timeoutMs, maxRetries: 1 });
  }

  modelFor(tier: AnalysisTier) {
    return tier === "heavy"
      ? this.config.heavyModel || this.config.standardModel
      : this.config.standardModel || this.config.heavyModel;
  }

  async analyze(request: FinancialAnalysisRequest): Promise<FinancialAnalysisResult> {
    const model = this.modelFor(request.tier);
    const sources = Object.keys(request.evidence);
    const completion = await this.client.chat.completions.create({
      model,
      temperature: 0.2,
      response_format: { type: "json_object" },
      messages: [
        { role: "system", content: FINANCIAL_SYSTEM_CONTRACT },
        { role: "user", content: renderEvidence(request) },
      ],
    });
    const raw = completion.choices[0]?.message?.content ?? "";
    const output = validateAnalysisOutput(raw, sources);
    if (!output) {
      throw new Error("A IA financeira devolveu uma resposta fora do formato.");
    }
    return { ...output, model, tier: request.tier };
  }
}

let cached: FinancialAiProvider | null | undefined;

/** O provider configurado, ou `null`. Nunca lança por falta de configuração. */
export function getFinancialAiProvider(): FinancialAiProvider | null {
  if (cached !== undefined) return cached;
  cached = isFinanceAiConfigured()
    ? new OpenAiCompatibleProvider({
        baseUrl: env.financeAiBaseUrl,
        apiKey: env.financeAiApiKey,
        standardModel: env.financeAiModelStandard,
        heavyModel: env.financeAiModelHeavy,
        timeoutMs: env.financeAiTimeoutMs,
      })
    : null;
  return cached;
}

/** Só para testes: troca o provider do processo. */
export function setFinancialAiProviderForTests(provider: FinancialAiProvider | null | undefined) {
  cached = provider;
}
