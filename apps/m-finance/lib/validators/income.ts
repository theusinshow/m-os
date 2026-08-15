import { z } from "zod";

export const incomeSchema = z.object({
  name: z.string().min(1, "Informe o nome da receita."),
  amountCents: z.number().int().positive("Informe um valor maior que zero."),
  incomeType: z.enum(["main", "extra", "freelance"]),
  expectedDate: z.string().optional(),
  received: z.boolean().default(false),
  notes: z.string().optional(),
});
