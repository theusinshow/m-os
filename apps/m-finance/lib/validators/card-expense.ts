import { z } from "zod";

export const cardExpenseSchema = z.object({
  description: z.string().trim().min(1, "Informe a descrição da compra."),
  amountCents: z.number().int().positive("Informe um valor maior que zero."),
  purchaseDate: z.string().optional(),
});
