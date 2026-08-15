import { z } from "zod";

export const subscriptionSchema = z.object({
  name: z.string().trim().min(1, "Informe o nome do serviço."),
  amountCents: z.number().int().positive("Informe um valor maior que zero."),
  nextChargeDate: z.string().min(1, "Informe a data da cobrança."),
  cycle: z.enum(["once", "monthly", "yearly"]).default("monthly"),
  isTrial: z.boolean().default(false),
  reminderDaysBefore: z.number().int().min(0).max(30).default(1),
});

export type SubscriptionInput = z.infer<typeof subscriptionSchema>;
