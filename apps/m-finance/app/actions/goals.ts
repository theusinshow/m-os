"use server";

import { and, eq } from "drizzle-orm";
import { revalidatePath } from "next/cache";
import { goalContributions, goals } from "@/db/schema";
import type { GoalStatus } from "@/db/schema";
import { db } from "@/db/client";
import { requireUser } from "@/lib/auth/guard";
import { getAppUserBySupabaseId } from "@/lib/months";
import { parseCurrencyToCents } from "@/lib/money";
import { contributionSchema, goalSchema } from "@/lib/validators/goal";
import {
  errorState,
  fieldErrorsFromZod,
  successState,
  type FormState,
} from "@/lib/form-state";

const VALID_STATUSES: GoalStatus[] = ["active", "paused", "completed", "archived"];

function todayIso() {
  return new Date().toISOString().slice(0, 10);
}

export async function createGoal(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);

  if (!db || !appUser) {
    return errorState("Banco ou usuário interno não configurado.");
  }

  const parsed = goalSchema.safeParse({
    name: formData.get("name"),
    targetAmountCents: parseCurrencyToCents(formData.get("targetAmount")),
    currentAmountCents: parseCurrencyToCents(formData.get("currentAmount")),
    deadline: String(formData.get("deadline") ?? "") || undefined,
    priority: formData.get("priority"),
    notes: formData.get("notes") || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, {
        targetAmountCents: "targetAmount",
        currentAmountCents: "currentAmount",
      }),
    );
  }

  const payload = parsed.data;
  // A goal tracks progress toward a target, so the saved amount is capped at it
  // — overshoot isn't recorded and the progress/summary never exceed 100%.
  const reached = payload.currentAmountCents >= payload.targetAmountCents;
  const currentAmountCents = Math.min(payload.currentAmountCents, payload.targetAmountCents);

  await db.insert(goals).values({
    userId: appUser.id,
    name: payload.name,
    targetAmountCents: payload.targetAmountCents,
    currentAmountCents,
    deadline: payload.deadline ?? null,
    priority: payload.priority,
    status: reached ? "completed" : "active",
    notes: payload.notes ?? null,
  });

  revalidatePath("/app/goals");
  return successState("Meta criada.");
}

export async function updateGoal(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const goalId = String(formData.get("goalId") ?? "");

  if (!db || !appUser || !goalId) {
    return errorState("Não foi possível editar a meta.");
  }

  const parsed = goalSchema.safeParse({
    name: formData.get("name"),
    targetAmountCents: parseCurrencyToCents(formData.get("targetAmount")),
    currentAmountCents: parseCurrencyToCents(formData.get("currentAmount")),
    deadline: String(formData.get("deadline") ?? "") || undefined,
    priority: formData.get("priority"),
    notes: formData.get("notes") || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, {
        targetAmountCents: "targetAmount",
        currentAmountCents: "currentAmount",
      }),
    );
  }

  const payload = parsed.data;

  const [existing] = await db
    .select({ status: goals.status })
    .from(goals)
    .where(and(eq(goals.id, goalId), eq(goals.userId, appUser.id)))
    .limit(1);

  if (!existing) {
    return errorState("Meta não encontrada.");
  }

  const reached = payload.currentAmountCents >= payload.targetAmountCents;
  const currentAmountCents = Math.min(payload.currentAmountCents, payload.targetAmountCents);
  // Editing a goal must not silently un-pause or un-archive it: status changes
  // belong to setGoalStatus. We only keep the active<->completed pair in sync
  // with the saved amount; paused/archived are preserved as-is.
  const nextStatus =
    existing.status === "active" || existing.status === "completed"
      ? reached
        ? "completed"
        : "active"
      : existing.status;

  await db
    .update(goals)
    .set({
      name: payload.name,
      targetAmountCents: payload.targetAmountCents,
      currentAmountCents,
      deadline: payload.deadline ?? null,
      priority: payload.priority,
      status: nextStatus,
      notes: payload.notes ?? null,
      updatedAt: new Date(),
    })
    .where(and(eq(goals.id, goalId), eq(goals.userId, appUser.id)));

  revalidatePath("/app/goals");
  return successState("Meta atualizada.");
}

export async function addContribution(_prev: FormState, formData: FormData): Promise<FormState> {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const goalId = String(formData.get("goalId") ?? "");

  if (!db || !appUser || !goalId) {
    return errorState("Não foi possível registrar a contribuição.");
  }

  const parsed = contributionSchema.safeParse({
    amountCents: parseCurrencyToCents(formData.get("amount")),
    contributionDate: String(formData.get("contributionDate") ?? "") || undefined,
  });

  if (!parsed.success) {
    return errorState(
      "Revise os campos destacados.",
      fieldErrorsFromZod(parsed.error, { amountCents: "amount" }),
    );
  }

  const payload = parsed.data;

  const [goal] = await db
    .select({
      currentAmountCents: goals.currentAmountCents,
      targetAmountCents: goals.targetAmountCents,
      status: goals.status,
    })
    .from(goals)
    .where(and(eq(goals.id, goalId), eq(goals.userId, appUser.id)))
    .limit(1);

  if (!goal) {
    return errorState("Meta não encontrada.");
  }

  // Cap progress at the target (overshoot isn't tracked) and only auto-complete
  // an actively-tracked goal — a paused goal stays paused even if it reaches the
  // target, honoring the user's explicit pause.
  const reached = goal.currentAmountCents + payload.amountCents >= goal.targetAmountCents;
  const newCurrent = Math.min(goal.currentAmountCents + payload.amountCents, goal.targetAmountCents);
  const nextStatus = reached && goal.status === "active" ? "completed" : goal.status;

  await db.transaction(async (tx) => {
    await tx.insert(goalContributions).values({
      userId: appUser.id,
      goalId,
      amountCents: payload.amountCents,
      contributionDate: payload.contributionDate ?? todayIso(),
    });
    await tx
      .update(goals)
      .set({ currentAmountCents: newCurrent, status: nextStatus, updatedAt: new Date() })
      .where(and(eq(goals.id, goalId), eq(goals.userId, appUser.id)));
  });

  revalidatePath("/app/goals");
  return successState(
    nextStatus === "completed"
      ? "Contribuição registrada. Meta concluída! 🎉"
      : "Contribuição registrada.",
  );
}

export async function setGoalStatus(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const goalId = String(formData.get("goalId") ?? "");
  const status = String(formData.get("status") ?? "") as GoalStatus;

  if (!db || !appUser || !goalId || !VALID_STATUSES.includes(status)) {
    throw new Error("Não foi possível atualizar a meta.");
  }

  await db
    .update(goals)
    .set({ status, updatedAt: new Date() })
    .where(and(eq(goals.id, goalId), eq(goals.userId, appUser.id)));

  revalidatePath("/app/goals");
}

export async function deleteGoal(formData: FormData) {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  const goalId = String(formData.get("goalId") ?? "");

  if (!db || !appUser || !goalId) {
    throw new Error("Não foi possível excluir a meta.");
  }

  await db.delete(goals).where(and(eq(goals.id, goalId), eq(goals.userId, appUser.id)));

  revalidatePath("/app/goals");
}
