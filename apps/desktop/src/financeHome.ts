/**
 * O que o widget FINANÇAS da Home tira do context pack do M-Finance.
 *
 * Função pura, e por isso aqui e não dentro do componente: o repo não tem
 * teste de DOM (ver `vitest.config.ts`), e o que dá para verificar tem de ser
 * função. Nenhum número é calculado — o M-Finance já calculou; aqui só se
 * escolhe o que cabe num widget.
 */

export type FinanceHomeSummary = {
  asOf: string;
  safeToSpendCents: number | null;
  /** `unknown` quando não há receita para calcular. */
  status: string;
  accountingRemainingCents: number | null;
  deductionsCents: number;
  proximo: { label: string; amountCents: number; dueDate: string; overdue: boolean; estimated: boolean } | null;
  vencidoCents: number;
  atencao: { title: string; summary: string; severity: string } | null;
};

type Json = Record<string, unknown>;

function obj(value: unknown): Json {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Json) : {};
}

function num(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function str(value: unknown): string {
  return typeof value === "string" ? value : "";
}

const SEVERIDADE: Record<string, number> = { critical: 2, warning: 1, info: 0 };

export function resumoDaHome(pack: unknown, asOf: string): FinanceHomeSummary {
  const root = obj(pack);
  const month = obj(root.currentMonth);
  const safe = obj(month.safeToSpend);
  const deductions = Array.isArray(safe.deductions) ? safe.deductions : [];
  const next = obj(root.next30Days);
  const items = (Array.isArray(next.items) ? next.items : []).map(obj);
  // O vencido vem primeiro: uma conta de agosto esquecida pesa mais que a de
  // outubro. Entre os a vencer, o primeiro da lista (que já vem por data).
  const alvo = items.find((item) => str(item.status) === "overdue") ?? items.find((item) => item.kind !== "subscription") ?? null;
  const insights = (Array.isArray(root.recentInsights) ? root.recentInsights : []).map(obj);
  const principal = [...insights].sort((a, b) => (SEVERIDADE[str(b.severity)] ?? 0) - (SEVERIDADE[str(a.severity)] ?? 0))[0];

  return {
    asOf,
    safeToSpendCents: num(safe.safeToSpendCents),
    status: str(safe.status) || "unknown",
    accountingRemainingCents: num(month.accountingRemainingCents),
    deductionsCents: deductions.reduce<number>((soma, item) => soma + (num(obj(item).amountCents) ?? 0), 0),
    proximo: alvo
      ? {
          label: str(alvo.label),
          amountCents: num(alvo.amountCents) ?? 0,
          dueDate: str(alvo.dueDate),
          overdue: str(alvo.status) === "overdue",
          estimated: alvo.estimated === true,
        }
      : null,
    vencidoCents: num(next.overdueCents) ?? 0,
    atencao: principal
      ? { title: str(principal.title), summary: str(principal.summary), severity: str(principal.severity) }
      : null,
  };
}

/** A pergunta que o "Ver análise" deixa pronta no Hermes. Nada é enviado sozinho. */
export function perguntaDoInsight(insight: { title: string }) {
  return `Sobre o alerta do M-Finance "${insight.title}": o que aconteceu, o que causou e o que você sugere?`;
}

/** `20/09` a partir de `2026-09-20`. */
export function diaCurto(isoDate: string) {
  const [, month, day] = isoDate.split("-");
  return month && day ? `${day}/${month}` : isoDate;
}
