import { monthIndex, monthOfDate } from "@/lib/finance-intelligence/dates";
import { sanitizeLabel } from "@/lib/finance-intelligence/sanitize";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";

export type GoalView = {
  id: string;
  name: string;
  priority: "low" | "medium" | "high";
  status: "active" | "paused" | "completed" | "archived";
  targetCents: number;
  currentCents: number;
  remainingCents: number;
  progressPercent: number;
  deadline: string | null;
  /** Meses de hoje até o prazo, contando o mês atual. Null sem prazo. */
  monthsLeft: number | null;
  /** Quanto separar por mês para chegar no prazo. Null sem prazo ou já cumprida. */
  requiredMonthlyCents: number | null;
  deadlinePassed: boolean;
};

const PRIORITY_ORDER = { high: 0, medium: 1, low: 2 } as const;

/**
 * As metas com o ritmo que cada uma pede.
 *
 * O ritmo é teto (arredonda para cima): separar R$ 333,33 por três meses deixa
 * um centavo de fora, e uma meta que falha por um centavo parece bug.
 */
export function goalSummary(snapshot: FinanceSnapshot) {
  const currentIndex = monthIndex(snapshot.current);

  const goals: GoalView[] = snapshot.goals
    .filter((goal) => goal.status !== "archived")
    .map((goal) => {
      const remainingCents = Math.max(0, goal.targetAmountCents - goal.currentAmountCents);
      const progressPercent =
        goal.targetAmountCents > 0
          ? Math.min(100, Math.round((goal.currentAmountCents / goal.targetAmountCents) * 100))
          : 0;
      const deadlinePassed = goal.deadline !== null && goal.deadline < snapshot.today;
      const monthsLeft =
        goal.deadline === null
          ? null
          : Math.max(0, monthIndex(monthOfDate(goal.deadline)) - currentIndex + 1);
      const requiredMonthlyCents =
        monthsLeft === null || remainingCents === 0 || goal.status === "completed"
          ? null
          : monthsLeft === 0
            ? remainingCents
            : Math.ceil(remainingCents / monthsLeft);

      return {
        id: goal.id,
        name: sanitizeLabel(goal.name),
        priority: goal.priority,
        status: goal.status,
        targetCents: goal.targetAmountCents,
        currentCents: goal.currentAmountCents,
        remainingCents,
        progressPercent,
        deadline: goal.deadline,
        monthsLeft,
        requiredMonthlyCents,
        deadlinePassed,
      };
    })
    .sort(
      (a, b) =>
        (a.status === "active" ? 0 : 1) - (b.status === "active" ? 0 : 1) ||
        PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority] ||
        (a.deadline ?? "9999").localeCompare(b.deadline ?? "9999"),
    );

  const active = goals.filter((goal) => goal.status === "active");
  return {
    goals,
    activeCount: active.length,
    totalRemainingCents: active.reduce((sum, goal) => sum + goal.remainingCents, 0),
    totalRequiredMonthlyCents: active.reduce((sum, goal) => sum + (goal.requiredMonthlyCents ?? 0), 0),
  };
}
