import { z } from "zod";

export const cardExpenseSchema = z
  .object({
    description: z.string().trim().min(1, "Informe a descrição da compra."),
    amountCents: z.number().int().positive("Informe um valor maior que zero."),
    purchaseDate: z.string().optional(),
    paymentType: z.enum(["cash", "installment"]).default("cash"),
    installments: z.number().int().min(2).max(60).optional(),
  })
  .superRefine((data, context) => {
    if (data.paymentType === "installment" && !data.installments) {
      context.addIssue({
        code: "custom",
        message: "Informe o número de parcelas.",
        path: ["installments"],
      });
    }
    if (
      data.paymentType === "installment" &&
      data.installments &&
      data.amountCents < data.installments
    ) {
      context.addIssue({
        code: "custom",
        message: "O valor total é muito baixo para essa quantidade de parcelas.",
        path: ["amountCents"],
      });
    }
  });
