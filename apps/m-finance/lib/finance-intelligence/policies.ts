import { z } from "zod";

/**
 * Políticas financeiras pessoais: regras de decisão que não pertencem a uma
 * conta nem a uma fatura.
 *
 * Não é key-value livre. Cada chave tem schema, padrão e efeito documentado, e
 * uma chave que não está aqui não existe — nem no banco (enum), nem na API.
 * "Considera freelance só como bônus" vira `reliable_income_rules.freelance =
 * 0`, e não uma frase guardada que a IA reinterpreta a cada conversa.
 */

const cents = z.number().int().min(0).max(100_000_000);
const weight = z.number().min(0).max(1);

export const policySchemas = {
  /** Quanto precisa sobrar no fim de todo mês. Sai do Safe-to-Spend. */
  minimum_month_end_buffer: z.object({ amountCents: cents }).strict(),
  /**
   * Quanto de cada tipo de receita AINDA NÃO RECEBIDA conta como certo. Receita
   * já recebida é fato e conta inteira, qualquer que seja o tipo.
   */
  reliable_income_rules: z
    .object({ main: weight, freelance: weight, extra: weight })
    .strict(),
  /** Teto pessoal de parcelas por mês. Vira aviso em cenário e no Observer. */
  max_installment_commitment: z.object({ amountCents: cents }).strict(),
  /** Horizonte padrão de projeção e de cenário. */
  forecast_horizon_months: z.object({ months: z.number().int().min(1).max(24) }).strict(),
  /** Quão cedo o Observer fala. Escala os limiares de materialidade. */
  observer_sensitivity: z.object({ level: z.enum(["low", "normal", "high"]) }).strict(),
  /** Como o Safe-to-Spend olha para frente e se protege as metas. */
  safe_to_spend_policy: z
    .object({
      lookaheadMonths: z.number().int().min(0).max(12),
      protectGoals: z.boolean(),
    })
    .strict(),
} as const;

export type PolicyKey = keyof typeof policySchemas;
export const POLICY_KEYS = Object.keys(policySchemas) as PolicyKey[];

export type PolicyValue<K extends PolicyKey> = z.infer<(typeof policySchemas)[K]>;

export type ResolvedPolicies = {
  minimumMonthEndBufferCents: number;
  reliableIncomeRules: { main: number; freelance: number; extra: number };
  maxInstallmentCommitmentCents: number | null;
  forecastHorizonMonths: number;
  observerSensitivity: "low" | "normal" | "high";
  safeToSpend: { lookaheadMonths: number; protectGoals: boolean };
  /** Quais chaves vieram do dono, e não do padrão. */
  configured: PolicyKey[];
};

export const DEFAULT_POLICIES: ResolvedPolicies = {
  // Zero, e a premissa do Safe-to-Spend diz isso por extenso. Uma margem
  // padrão seria o sistema inventando uma regra que o dono não deu.
  minimumMonthEndBufferCents: 0,
  reliableIncomeRules: { main: 1, freelance: 0.5, extra: 0 },
  maxInstallmentCommitmentCents: null,
  forecastHorizonMonths: 6,
  observerSensitivity: "normal",
  safeToSpend: { lookaheadMonths: 2, protectGoals: false },
  configured: [],
};

export function isPolicyKey(value: unknown): value is PolicyKey {
  return typeof value === "string" && (POLICY_KEYS as string[]).includes(value);
}

export function parsePolicyValue(key: PolicyKey, value: unknown) {
  return policySchemas[key].safeParse(value);
}

type PolicyRow = { key: string; value: unknown; active: boolean };

/**
 * As políticas ativas por cima do padrão. Uma linha que não passa no schema é
 * ignorada (e o padrão vale) — um valor corrompido no banco não pode derrubar o
 * Safe-to-Spend nem o contexto do Hermes.
 */
export function resolvePolicies(rows: PolicyRow[]): ResolvedPolicies {
  const resolved: ResolvedPolicies = {
    ...DEFAULT_POLICIES,
    reliableIncomeRules: { ...DEFAULT_POLICIES.reliableIncomeRules },
    safeToSpend: { ...DEFAULT_POLICIES.safeToSpend },
    configured: [],
  };

  for (const row of rows) {
    if (!row.active || !isPolicyKey(row.key)) continue;
    const parsed = parsePolicyValue(row.key, row.value);
    if (!parsed.success) continue;
    applyPolicy(resolved, row.key, parsed.data);
    resolved.configured.push(row.key);
  }

  return resolved;
}

/** Sobrepõe uma política (usado também pelo cenário temporário). */
export function applyPolicy<K extends PolicyKey>(
  target: ResolvedPolicies,
  key: K,
  value: PolicyValue<K>,
) {
  switch (key) {
    case "minimum_month_end_buffer":
      target.minimumMonthEndBufferCents = (value as PolicyValue<"minimum_month_end_buffer">).amountCents;
      break;
    case "reliable_income_rules":
      target.reliableIncomeRules = { ...(value as PolicyValue<"reliable_income_rules">) };
      break;
    case "max_installment_commitment":
      target.maxInstallmentCommitmentCents = (value as PolicyValue<"max_installment_commitment">).amountCents;
      break;
    case "forecast_horizon_months":
      target.forecastHorizonMonths = (value as PolicyValue<"forecast_horizon_months">).months;
      break;
    case "observer_sensitivity":
      target.observerSensitivity = (value as PolicyValue<"observer_sensitivity">).level;
      break;
    case "safe_to_spend_policy":
      target.safeToSpend = { ...(value as PolicyValue<"safe_to_spend_policy">) };
      break;
  }
}

/** O valor atual de uma chave, no formato do schema — para preview e API. */
export function policyValueOf(policies: ResolvedPolicies, key: PolicyKey): unknown {
  switch (key) {
    case "minimum_month_end_buffer":
      return { amountCents: policies.minimumMonthEndBufferCents };
    case "reliable_income_rules":
      return { ...policies.reliableIncomeRules };
    case "max_installment_commitment":
      return policies.maxInstallmentCommitmentCents === null
        ? null
        : { amountCents: policies.maxInstallmentCommitmentCents };
    case "forecast_horizon_months":
      return { months: policies.forecastHorizonMonths };
    case "observer_sensitivity":
      return { level: policies.observerSensitivity };
    case "safe_to_spend_policy":
      return { ...policies.safeToSpend };
  }
}

export const POLICY_LABELS: Record<PolicyKey, string> = {
  minimum_month_end_buffer: "Reserva mínima ao fechar o mês",
  reliable_income_rules: "Quanto de cada receita é confiável",
  max_installment_commitment: "Máximo desejado em parcelas por mês",
  forecast_horizon_months: "Horizonte padrão de projeção",
  observer_sensitivity: "Sensibilidade dos alertas",
  safe_to_spend_policy: "Como o Safe-to-Spend olha para frente",
};

/** Multiplicador dos limiares do Observer: mais sensível, limiar menor. */
export function sensitivityFactor(level: ResolvedPolicies["observerSensitivity"]) {
  return level === "high" ? 0.6 : level === "low" ? 1.6 : 1;
}
