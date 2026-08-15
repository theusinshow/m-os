import { z } from "zod";

export const invoiceSchema = z.object({
  cardId: z.string().uuid("Selecione um cartão válido."),
  amountCents: z.number().int().positive("Informe um valor maior que zero."),
  dueDay: z.number().int().min(1).max(31).optional(),
  notes: z.string().optional(),
});
