import { z } from "zod";

/**
 * O vocabulário de "e se". Cada mudança é temporária e explícita — o engine
 * nunca recebe o banco, então não tem como gravar nada.
 *
 * Meses são `yyyy-mm`. Omitido, vale o mês atual.
 */

const monthKey = z.string().regex(/^\d{4}-(0[1-9]|1[0-2])$/, "Use yyyy-mm.");
const positiveCents = z.number().int().positive().max(100_000_000);
const label = z.string().trim().min(1).max(80);
const incomeType = z.enum(["main", "freelance", "extra"]);

export const scenarioChangeSchema = z.discriminatedUnion("type", [
  z
    .object({
      type: z.literal("one_time_expense"),
      label,
      amountCents: positiveCents,
      month: monthKey.optional(),
    })
    .strict(),
  z
    .object({
      type: z.literal("installment_purchase"),
      label,
      totalCents: positiveCents,
      installments: z.number().int().min(1).max(60),
      startMonth: monthKey.optional(),
      downPaymentCents: z.number().int().min(0).max(100_000_000).optional(),
    })
    .strict()
    .refine((value) => (value.downPaymentCents ?? 0) < value.totalCents, {
      message: "A entrada precisa ser menor que o total.",
    }),
  z
    .object({
      type: z.literal("recurring_expense"),
      label,
      amountCents: positiveCents,
      fromMonth: monthKey.optional(),
      untilMonth: monthKey.optional(),
    })
    .strict(),
  z
    .object({
      type: z.literal("remove_expense"),
      target: z.discriminatedUnion("kind", [
        z.object({ kind: z.literal("subscription"), id: z.string().min(1).max(64) }).strict(),
        z.object({ kind: z.literal("bill"), name: label }).strict(),
        z.object({ kind: z.literal("amount"), label, amountCents: positiveCents }).strict(),
      ]),
      fromMonth: monthKey.optional(),
    })
    .strict(),
  z
    .object({
      type: z.literal("change_income"),
      incomeType: incomeType.or(z.literal("all")),
      deltaCents: z.number().int().min(-100_000_000).max(100_000_000).optional(),
      percent: z.number().min(-100).max(500).optional(),
      fromMonth: monthKey.optional(),
      untilMonth: monthKey.optional(),
    })
    .strict()
    .refine((value) => (value.deltaCents === undefined) !== (value.percent === undefined), {
      message: "Informe deltaCents OU percent.",
    })
    .refine((value) => value.incomeType !== "all" || value.percent !== undefined, {
      message: "Para todas as receitas, use percent.",
    }),
  z
    .object({
      type: z.literal("remove_income"),
      incomeType: incomeType.or(z.literal("all")),
      fromMonth: monthKey.optional(),
    })
    .strict(),
  z
    .object({
      type: z.literal("add_income"),
      label,
      amountCents: positiveCents,
      incomeType: incomeType.default("extra"),
      recurring: z.boolean().default(true),
      fromMonth: monthKey.optional(),
      untilMonth: monthKey.optional(),
    })
    .strict(),
  z
    .object({
      type: z.literal("pay_off_installments"),
      installmentId: z.string().min(1).max(64),
      month: monthKey.optional(),
    })
    .strict(),
  z
    .object({
      type: z.literal("set_policy_temporary"),
      minimumMonthEndBufferCents: z.number().int().min(0).max(100_000_000).optional(),
      maxInstallmentCommitmentCents: z.number().int().min(0).max(100_000_000).optional(),
      reliableIncomeRules: z
        .object({
          main: z.number().min(0).max(1),
          freelance: z.number().min(0).max(1),
          extra: z.number().min(0).max(1),
        })
        .strict()
        .optional(),
    })
    .strict(),
]);

export type ScenarioChange = z.infer<typeof scenarioChangeSchema>;

export const MAX_SCENARIO_CHANGES = 12;
export const MAX_SCENARIO_VARIANTS = 5;
export const MAX_HORIZON_MONTHS = 24;

export const scenarioRequestSchema = z
  .object({
    horizonMonths: z.number().int().min(1).max(MAX_HORIZON_MONTHS).optional(),
    changes: z.array(scenarioChangeSchema).max(MAX_SCENARIO_CHANGES).optional(),
    variants: z
      .array(
        z
          .object({
            label,
            changes: z.array(scenarioChangeSchema).max(MAX_SCENARIO_CHANGES),
          })
          .strict(),
      )
      .min(1)
      .max(MAX_SCENARIO_VARIANTS)
      .optional(),
  })
  .strict()
  .refine((value) => (value.changes === undefined) !== (value.variants === undefined), {
    message: "Informe changes OU variants.",
  });

export type ScenarioRequest = z.infer<typeof scenarioRequestSchema>;
