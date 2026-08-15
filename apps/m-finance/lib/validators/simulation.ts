import { z } from "zod";

export const simulationSchema = z
  .object({
    name: z.string().trim().min(1, "Informe o nome da compra."),
    totalAmountCents: z.number().int().positive("Informe um valor maior que zero."),
    paymentType: z.enum(["cash", "installment"]),
    installments: z.number().int().min(1).max(60).optional(),
    startMonth: z.coerce.number().int().min(1).max(12),
    startYear: z.coerce.number().int().min(2020).max(2100),
  })
  .refine((data) => data.paymentType === "cash" || (data.installments ?? 0) >= 2, {
    message: "Parcelado exige ao menos 2 parcelas.",
    path: ["installments"],
  });
