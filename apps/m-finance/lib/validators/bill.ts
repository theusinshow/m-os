import { z } from "zod";

export const billSchema = z.object({
  name: z.string().min(1, "Informe o nome da conta."),
  amountCents: z.number().int().positive("Informe um valor maior que zero."),
  categoryId: z.string().uuid().optional(),
  dueDay: z.number().int().min(1).max(31).optional(),
  isRecurring: z.boolean().default(false),
  notes: z.string().optional(),
});

export const createBillSchema = billSchema
  .extend({
    scheduleType: z.enum(["once", "fixed", "ongoing"]).default("once"),
    repeatMonths: z.number().int().min(2).max(60).optional(),
  })
  .superRefine((data, context) => {
    if (data.scheduleType === "fixed" && !data.repeatMonths) {
      context.addIssue({
        code: "custom",
        message: "Informe por quantos meses a conta deve se repetir.",
        path: ["repeatMonths"],
      });
    }
  });
