import { z } from "zod";

export const billSchema = z.object({
  name: z.string().min(1, "Informe o nome da conta."),
  amountCents: z.number().int().positive("Informe um valor maior que zero."),
  categoryId: z.string().uuid().optional(),
  dueDay: z.number().int().min(1).max(31).optional(),
  isRecurring: z.boolean().default(false),
  notes: z.string().optional(),
});
