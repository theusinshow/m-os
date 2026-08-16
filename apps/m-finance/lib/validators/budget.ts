import { z } from "zod";

export const budgetSchema = z.object({
  budgetType: z.enum(["total", "category", "card"]),
  limitCents: z.number().int().positive("Informe um valor maior que zero."),
  categoryId: z.string().uuid().optional(),
  cardId: z.string().uuid().optional(),
}).superRefine((data, context) => {
  if (data.budgetType === "category" && !data.categoryId) {
    context.addIssue({
      code: "custom",
      message: "Selecione uma categoria.",
      path: ["categoryId"],
    });
  }
  if (data.budgetType === "card" && !data.cardId) {
    context.addIssue({
      code: "custom",
      message: "Selecione um cartão.",
      path: ["cardId"],
    });
  }
  if (data.budgetType === "total" && (data.categoryId || data.cardId)) {
    context.addIssue({
      code: "custom",
      message: "Orçamento total não deve ter categoria ou cartão.",
      path: ["budgetType"],
    });
  }
});
