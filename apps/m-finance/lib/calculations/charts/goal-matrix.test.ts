import { describe, expect, it } from "vitest";
import { toGoalMatrix } from "@/lib/calculations/charts/goal-matrix";
import type { GoalWithProgress } from "@/lib/goals";

const HOJE = new Date(2026, 7, 24); // 24 de agosto de 2026

function goal(
  overrides: Partial<GoalWithProgress> & Pick<GoalWithProgress, "id">,
): GoalWithProgress {
  return {
    name: `Meta ${overrides.id}`,
    targetAmountCents: 100_000,
    currentAmountCents: 25_000,
    deadline: "2026-09-23",
    priority: "medium",
    status: "active",
    notes: null,
    progressPercent: 25,
    remainingCents: 75_000,
    ...overrides,
  };
}

describe("toGoalMatrix", () => {
  it("mede os dias que faltam ate o prazo", () => {
    const { points } = toGoalMatrix([goal({ id: "a", deadline: "2026-09-23" })], HOJE);
    expect(points[0].daysLeft).toBe(30);
  });

  it("devolve dias negativos quando o prazo ja passou", () => {
    const { points } = toGoalMatrix([goal({ id: "a", deadline: "2026-08-14" })], HOJE);
    expect(points[0].daysLeft).toBe(-10);
  });

  it("separa as metas sem prazo em vez de inventar um eixo para elas", () => {
    const { points, withoutDeadline } = toGoalMatrix(
      [goal({ id: "com" }), goal({ id: "sem", deadline: null })],
      HOJE,
    );

    expect(points.map((point) => point.id)).toEqual(["com"]);
    expect(withoutDeadline.map((item) => item.id)).toEqual(["sem"]);
  });

  it("usa o quanto falta, nao o quanto ja foi", () => {
    const { points } = toGoalMatrix(
      [goal({ id: "a", progressPercent: 25, remainingCents: 75_000 })],
      HOJE,
    );

    expect(points[0].remainingPercent).toBe(75);
    expect(points[0].remainingCents).toBe(75_000);
  });

  it("deixa de fora meta concluida e arquivada", () => {
    const { points, withoutDeadline } = toGoalMatrix(
      [
        goal({ id: "ativa" }),
        goal({ id: "pausada", status: "paused" }),
        goal({ id: "concluida", status: "completed" }),
        goal({ id: "arquivada", status: "archived" }),
      ],
      HOJE,
    );

    expect(points.map((point) => point.id)).toEqual(["ativa", "pausada"]);
    expect(withoutDeadline).toEqual([]);
  });

  it("nao quebra sem nenhuma meta", () => {
    expect(toGoalMatrix([], HOJE)).toEqual({ points: [], withoutDeadline: [] });
  });
});
