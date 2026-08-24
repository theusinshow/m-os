import type { GoalWithProgress } from "@/lib/goals";

export type GoalMatrixPoint = {
  id: string;
  name: string;
  /** Negativo quando o prazo já passou. */
  daysLeft: number;
  /** 0 a 100. Quanto ainda falta, não quanto já foi. */
  remainingPercent: number;
  remainingCents: number;
};

/**
 * As metas em duas dimensões: tempo que resta e distância do objetivo.
 *
 * O eixo Y é o que **falta**, e não o progresso, porque o gráfico existe para
 * mostrar risco: quem falta muito fica em cima, onde o olho vai primeiro. O
 * canto superior esquerdo — falta muito, sobra pouco tempo — é a meta em apuro.
 *
 * Meta sem prazo não entra. Inventar um `x` para ela seria mentir sobre o dado,
 * e ela sai numa lista à parte para não sumir da tela.
 */
export function toGoalMatrix(
  goals: GoalWithProgress[],
  today: Date,
): { points: GoalMatrixPoint[]; withoutDeadline: GoalWithProgress[] } {
  const tracked = goals.filter((goal) => goal.status === "active" || goal.status === "paused");

  const points: GoalMatrixPoint[] = [];
  const withoutDeadline: GoalWithProgress[] = [];

  for (const goal of tracked) {
    if (!goal.deadline) {
      withoutDeadline.push(goal);
      continue;
    }

    points.push({
      id: goal.id,
      name: goal.name,
      daysLeft: daysBetween(today, goal.deadline),
      remainingPercent: Math.max(0, 100 - goal.progressPercent),
      remainingCents: goal.remainingCents,
    });
  }

  return { points, withoutDeadline };
}

/**
 * Dias inteiros entre hoje e uma data `YYYY-MM-DD`.
 *
 * Os dois lados viram `Date.UTC` antes da subtração: em milissegundos locais,
 * um mês que atravessa mudança de horário devolve 29,96 dias e arredonda para
 * o dia errado.
 */
function daysBetween(today: Date, isoDate: string) {
  const [year, month, day] = isoDate.split("-").map(Number);
  const target = Date.UTC(year, month - 1, day);
  const from = Date.UTC(today.getFullYear(), today.getMonth(), today.getDate());
  return Math.round((target - from) / 86_400_000);
}
