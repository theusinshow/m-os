import { z } from "zod";
import { analyzeArgsSchema, analyzeFinances } from "@/lib/finance-intelligence/ai/analyst";
import { addMonths } from "@/lib/finance-intelligence/dates";
import { cardExposure } from "@/lib/finance-intelligence/kernel/cards";
import { futureCommitments, upcomingCommitments } from "@/lib/finance-intelligence/kernel/commitments";
import { comparePeriods } from "@/lib/finance-intelligence/kernel/compare";
import { contextPack, type PackInsight } from "@/lib/finance-intelligence/kernel/context-pack";
import { ENTITY_KINDS, findEntities } from "@/lib/finance-intelligence/kernel/entities";
import { goalSummary } from "@/lib/finance-intelligence/kernel/goals";
import { monthOverview } from "@/lib/finance-intelligence/kernel/overview";
import { safeToSpend } from "@/lib/finance-intelligence/kernel/safe-to-spend";
import { subscriptionSummary } from "@/lib/finance-intelligence/kernel/subscriptions";
import { POLICY_KEYS, POLICY_LABELS, policyValueOf } from "@/lib/finance-intelligence/policies";
import { compareScenarios, simulateScenario } from "@/lib/finance-intelligence/scenarios/engine";
import { scenarioRequestSchema } from "@/lib/finance-intelligence/scenarios/types";
import type { FinanceSnapshot, MonthParts } from "@/lib/finance-intelligence/types";

/**
 * O catálogo de LEITURA do Intelligence Gateway.
 *
 * Um id por ferramenta, um schema `strict` por ferramenta, e nenhum caminho
 * que aceite SQL, nome de tabela ou operação arbitrária. Nenhuma ferramenta
 * recebe o banco: elas recebem o snapshot já carregado. Escrever daqui não é
 * proibido por convenção — é impossível por construção.
 */
export type ToolContext = {
  snapshot: () => Promise<FinanceSnapshot>;
  /** Insights para o pacote: detectados agora, em memória. */
  liveInsights: () => Promise<PackInsight[]>;
  /** Insights gravados pelo Observer (abertos e reconhecidos). */
  storedInsights: (limit: number) => Promise<unknown[]>;
};

const monthParts = z.object({ month: z.number().int().min(1).max(12), year: z.number().int().min(2020).max(2100) }).strict();
const optionalMonth = z
  .object({ month: z.number().int().min(1).max(12).optional(), year: z.number().int().min(2020).max(2100).optional() })
  .strict()
  .refine((value) => (value.month === undefined) === (value.year === undefined), {
    message: "Informe month e year juntos.",
  });

function monthOrCurrent(args: { month?: number; year?: number }, snapshot: FinanceSnapshot): MonthParts {
  return args.month && args.year ? { month: args.month, year: args.year } : snapshot.current;
}

type ToolDefinition<S extends z.ZodType> = {
  description: string;
  schema: S;
  run: (args: z.infer<S>, context: ToolContext) => Promise<unknown>;
};

function tool<S extends z.ZodType>(definition: ToolDefinition<S>) {
  return definition;
}

const empty = z.object({}).strict();

export const FINANCE_TOOLS = {
  "finance.get_context_pack": tool({
    description: "Resumo compacto: mês atual, Safe-to-Spend, 30 dias, próximos meses, cartões, assinaturas, metas, políticas, insights.",
    schema: empty,
    run: async (_args, context) => contextPack(await context.snapshot(), await context.liveInsights()),
  }),
  "finance.get_month_overview": tool({
    description: "Totais de um mês: receita (total, confiável, estimada), contas, faturas, pago, pendente, vencido, sobra, saúde.",
    schema: optionalMonth,
    run: async (args, context) => {
      const snapshot = await context.snapshot();
      return monthOverview(snapshot, monthOrCurrent(args, snapshot));
    },
  }),
  "finance.get_upcoming_commitments": tool({
    description: "O que vence nos próximos N dias (1–90) e o que já venceu sem pagar.",
    schema: z.object({ days: z.number().int().min(1).max(90).default(30) }).strict(),
    run: async (args, context) => upcomingCommitments(await context.snapshot(), args.days),
  }),
  "finance.get_future_commitments": tool({
    description: "Comprometimento mês a mês (1–24 meses), com renda confiável, parcelas e o que termina.",
    schema: z.object({ months: z.number().int().min(1).max(24).optional() }).strict(),
    run: async (args, context) => {
      const snapshot = await context.snapshot();
      return futureCommitments(snapshot, args.months ?? snapshot.policies.forecastHorizonMonths);
    },
  }),
  "finance.get_card_exposure": tool({
    description: "Por cartão: histórico, faturas futuras (reais/estimadas), tendência e parcelamentos vivos.",
    schema: z
      .object({ cardId: z.string().uuid().nullable().optional(), months: z.number().int().min(1).max(12).default(3) })
      .strict(),
    run: async (args, context) =>
      cardExposure(await context.snapshot(), { cardId: args.cardId ?? null, months: args.months }),
  }),
  "finance.compare_periods": tool({
    description: "Dois meses lado a lado: deltas absolutos e percentuais, por cartão e por categoria.",
    schema: z.object({ from: monthParts.optional(), to: monthParts.optional() }).strict(),
    run: async (args, context) => {
      const snapshot = await context.snapshot();
      return comparePeriods(snapshot, args.from ?? addMonths(snapshot.current, -1), args.to ?? snapshot.current);
    },
  }),
  "finance.get_subscriptions": tool({
    description: "Assinaturas e testes grátis, com custo mensal equivalente.",
    schema: empty,
    run: async (_args, context) => subscriptionSummary(await context.snapshot()),
  }),
  "finance.get_goals": tool({
    description: "Metas: quanto falta, prazo e ritmo mensal necessário.",
    schema: empty,
    run: async (_args, context) => goalSummary(await context.snapshot()),
  }),
  "finance.get_safe_to_spend": tool({
    description: "Safe-to-Spend com a decomposição completa (sobra contábil, deduções, premissas).",
    schema: optionalMonth,
    run: async (args, context) => {
      const snapshot = await context.snapshot();
      return safeToSpend(snapshot, monthOrCurrent(args, snapshot));
    },
  }),
  "finance.simulate_scenario": tool({
    description: "Simula mudanças temporárias (compra, parcelas, renda, assinaturas, política) por 1–24 meses. Nunca grava.",
    schema: scenarioRequestSchema,
    run: async (args, context) => {
      const snapshot = await context.snapshot();
      return args.variants
        ? compareScenarios(snapshot, args.variants, args.horizonMonths)
        : simulateScenario(snapshot, args.changes ?? [], args.horizonMonths);
    },
  }),
  "finance.get_recent_insights": tool({
    description: "Insights do Observer ainda vivos, mais os detectados agora.",
    schema: z.object({ limit: z.number().int().min(1).max(20).default(10) }).strict(),
    run: async (args, context) => ({
      live: (await context.liveInsights()).slice(0, args.limit),
      stored: await context.storedInsights(args.limit),
    }),
  }),
  "finance.get_policies": tool({
    description: "Políticas financeiras em vigor, com o valor atual de cada chave.",
    schema: empty,
    run: async (_args, context) => {
      const { policies } = await context.snapshot();
      return {
        configured: policies.configured,
        policies: POLICY_KEYS.map((key) => ({
          key,
          label: POLICY_LABELS[key],
          value: policyValueOf(policies, key),
          isDefault: !policies.configured.includes(key),
        })),
      };
    },
  }),
  "finance.find_entities": tool({
    description: "Acha contas, faturas, cartões, assinaturas, metas e receitas pelo nome, com id — para propor ação.",
    schema: z
      .object({ query: z.string().trim().min(1).max(80), kinds: z.array(z.enum(ENTITY_KINDS)).min(1).max(6).optional() })
      .strict(),
    run: async (args, context) => findEntities(await context.snapshot(), args.query, args.kinds),
  }),
  "finance.analyze": tool({
    description: "Análise por IA pesada sobre evidência determinística (revisão, plano, comparação de cenários, trade-offs).",
    schema: analyzeArgsSchema,
    run: async (args, context) => analyzeFinances(await context.snapshot(), args, await context.liveInsights()),
  }),
} as const;

export type FinanceToolId = keyof typeof FINANCE_TOOLS;
export const FINANCE_TOOL_IDS = Object.keys(FINANCE_TOOLS) as FinanceToolId[];

export function isFinanceToolId(value: unknown): value is FinanceToolId {
  return typeof value === "string" && Object.hasOwn(FINANCE_TOOLS, value);
}
