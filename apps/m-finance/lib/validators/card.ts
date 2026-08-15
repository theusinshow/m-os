import { z } from "zod";

export const cardSchema = z.object({
  name: z.string().trim().min(1, "Informe o nome do cartão."),
  cardType: z.enum(["personal", "business"]),
  dueDay: z.coerce
    .number()
    .int("Informe um dia válido.")
    .min(1, "O dia deve ser entre 1 e 31.")
    .max(31, "O dia deve ser entre 1 e 31."),
});
