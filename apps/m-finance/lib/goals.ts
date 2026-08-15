import { and, asc, desc, eq, sql } from "drizzle-orm";
import { db } from "@/db/client";
import { goalContributions, goals } from "@/db/schema";
import type { GoalPriority, GoalStatus } from "@/db/schema";

export type GoalWithProgress = {
  id: string;
  name: string;
  targetAmountCents: number;
  currentAmountCents: number;
  deadline: string | null;
  priority: GoalPriority;
  status: GoalStatus;
  notes: string | null;
  progressPercent: number;
  remainingCents: number;
};

export async function getGoals(userId: string): Promise<GoalWithProgress[]> {
  if (!db) {
    return [];
  }

  // Active work first, then paused, completed, and finally archived.
  const statusOrder = sql`case ${goals.status}
    when 'active' then 0
    when 'paused' then 1
    when 'completed' then 2
    else 3
  end`;

  const rows = await db
    .select()
    .from(goals)
    .where(eq(goals.userId, userId))
    .orderBy(statusOrder, desc(goals.createdAt));

  return rows.map((goal) => {
    const progressPercent =
      goal.targetAmountCents > 0
        ? Math.min(100, Math.round((goal.currentAmountCents / goal.targetAmountCents) * 100))
        : 0;
    return {
      id: goal.id,
      name: goal.name,
      targetAmountCents: goal.targetAmountCents,
      currentAmountCents: goal.currentAmountCents,
      deadline: goal.deadline,
      priority: goal.priority,
      status: goal.status,
      notes: goal.notes,
      progressPercent,
      remainingCents: Math.max(0, goal.targetAmountCents - goal.currentAmountCents),
    };
  });
}

export async function getGoalContributions(userId: string, goalId: string) {
  if (!db) {
    return [];
  }

  return db
    .select()
    .from(goalContributions)
    .where(and(eq(goalContributions.userId, userId), eq(goalContributions.goalId, goalId)))
    .orderBy(asc(goalContributions.contributionDate));
}
