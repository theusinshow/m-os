import { z } from "zod";

export const settingsSchema = z.object({
  alertDaysBefore: z.coerce
    .number()
    .int("Informe um número inteiro de dias.")
    .min(0, "O aviso não pode ser negativo.")
    .max(30, "O aviso máximo é de 30 dias."),
});

export const categorySchema = z.object({
  name: z.string().trim().min(1, "Informe o nome da categoria."),
});
