import { z } from "zod";

export const goalSchema = z.object({
  name: z.string().trim().min(1, "Informe o nome da meta."),
  targetAmountCents: z.number().int().positive("Informe um valor alvo maior que zero."),
  currentAmountCents: z.number().int().min(0, "O valor atual não pode ser negativo.").default(0),
  deadline: z.string().optional(),
  priority: z.enum(["low", "medium", "high"]),
  notes: z.string().optional(),
});

export const contributionSchema = z.object({
  amountCents: z.number().int().positive("Informe um valor maior que zero."),
  contributionDate: z.string().optional(),
});
