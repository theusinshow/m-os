import { z } from "zod";

export const incomeSchema = z.object({
  name: z.string().min(1, "Informe o nome da receita."),
  amountCents: z.number().int().positive("Informe um valor maior que zero."),
  incomeType: z.enum(["main", "extra", "freelance"]),
  expectedDate: z.string().optional(),
  received: z.boolean().default(false),
  notes: z.string().optional(),
});

export const createIncomeSchema = incomeSchema.extend({
  /** Mês em que a receita entra, no formato `yyyy-mm`. Vazio = mês da tela. */
  targetMonth: z
    .string()
    .regex(/^\d{4}-\d{2}$/, "Escolha o mês em que a receita entra.")
    .optional(),
});
