"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { purchaseSimulations } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId, getCurrentMonthForUser } from "@/lib/months";
import { parseCurrencyToCents } from "@/lib/money";
import { simulationSchema } from "@/lib/validators/simulation";
import { computeSimulation } from "@/lib/calculations/simulator";
import { getSimulationBaseline } from "@/lib/simulations";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

export async function createSimulation(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const currentMonth = await getCurrentMonthForUser(appUser.id);

  if (!currentMonth) {
    return errorState("Crie o mês atual antes de simular uma compra.");
  }

  const paymentType = String(formData.get("paymentType") ?? "");
  const installmentsRaw = String(formData.get("installments") ?? "").trim();
  const parsed = simulationSchema.safeParse({
    name: formData.get("name"),
    totalAmountCents: parseCurrencyToCents(formData.get("amount")),
    paymentType,
    installments: installmentsRaw ? Number(installmentsRaw) : undefined,
    startMonth: formData.get("startMonth"),
    startYear: formData.get("startYear"),
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { totalAmountCents: "amount" }),
    );
  }

  const payload = parsed.data;
  const baseline = await getSimulationBaseline(appUser.id, currentMonth.id);

  const result = computeSimulation({
    totalAmountCents: payload.totalAmountCents,
    paymentType: payload.paymentType,
    installments: payload.installments ?? 1,
    startMonth: payload.startMonth,
    startYear: payload.startYear,
    baselineRemainingCents: baseline.baselineRemainingCents,
  });

  await db.insert(purchaseSimulations).values({
    userId: appUser.id,
    name: payload.name,
    totalAmountCents: payload.totalAmountCents,
    paymentType: payload.paymentType,
    installments: payload.paymentType === "installment" ? (payload.installments ?? null) : null,
    startMonth: payload.startMonth,
    startYear: payload.startYear,
    monthlyImpactCents: result.monthlyImpactCents,
    riskLevel: result.riskLevel,
    recommendation: result.recommendation,
    resultPayload: result,
  });

  revalidatePath("/app/simulator");
  return successState("Simulação salva.");
}

export async function deleteSimulation(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const simulationId = String(formData.get("simulationId") ?? "");

  if (!db || !appUser || !simulationId) {
    throw new Error("Não foi possível excluir a simulação.");
  }

  await db
    .delete(purchaseSimulations)
    .where(
      and(
        eq(purchaseSimulations.id, simulationId),
        eq(purchaseSimulations.userId, appUser.id),
      ),
    );

  revalidatePath("/app/simulator");
}
