import type { FinancialInsightSeverity, FinancialInsightStatus } from "@/db/schema";
import { severityRank, type FinanceObservation } from "@/lib/finance-intelligence/observer/detectors";

/** Um insight reconhecido volta a pedir atenção depois disto, se continuar material. */
export const COOLDOWN_DAYS = 7;
/** Mudança de materialidade que conta como "o valor mudou". */
export const MATERIAL_CHANGE_POINTS = 10;

export type LiveInsight = {
  id: string;
  dedupeKey: string;
  status: FinancialInsightStatus;
  severity: FinancialInsightSeverity;
  materialityScore: number;
  acknowledgedAt: Date | null;
  lastSeenAt: Date;
  facts: unknown;
};

export type InsightDecision =
  /** Não havia: cria aberto. */
  | { action: "create"; notify: boolean }
  /** Continua igual: só `last_seen_at` e os fatos. */
  | { action: "touch" }
  /** Aberto e piorou: atualiza e avisa de novo. */
  | { action: "escalate"; notify: boolean }
  /** Reconhecido, mas piorou ou o cooldown venceu: volta a aberto. */
  | { action: "reopen"; notify: boolean };

/**
 * O que fazer com uma observação diante do insight vivo que já existe.
 *
 * A regra anti-spam inteira mora aqui, e é código: o mesmo evento em dez
 * ciclos seguidos produz um insight e nove `touch`. Ele só volta a chamar
 * atenção quando a severidade sobe, quando o valor muda de verdade, ou quando
 * o dono o reconheceu há mais de uma semana e ele continua lá.
 *
 * `notify` diz se vale push — e só vale para o que é crítico.
 */
export function decideInsightUpdate(
  existing: LiveInsight | null,
  observation: FinanceObservation,
  now: Date,
): InsightDecision {
  const critical = observation.severity === "critical";
  if (!existing) return { action: "create", notify: critical };

  const escalated = severityRank(observation.severity) > severityRank(existing.severity);
  const changed = Math.abs(observation.materialityScore - existing.materialityScore) >= MATERIAL_CHANGE_POINTS;

  if (existing.status === "open") {
    return escalated || changed ? { action: "escalate", notify: critical && escalated } : { action: "touch" };
  }

  // Reconhecido.
  if (escalated || changed) return { action: "reopen", notify: critical && escalated };
  const since = existing.acknowledgedAt ?? existing.lastSeenAt;
  const cooledDown = now.getTime() - since.getTime() >= COOLDOWN_DAYS * 86_400_000;
  return cooledDown ? { action: "reopen", notify: false } : { action: "touch" };
}

/**
 * Os vivos que esta rodada não viu. `bill_due_soon` que sumiu não foi
 * "resolvido" — o vencimento passou (virou vencido ou foi pago) — e fica como
 * expirado; o resto se resolveu.
 */
export function staleInsights(live: (LiveInsight & { type: string })[], observedKeys: Set<string>) {
  return live
    .filter((insight) => !observedKeys.has(insight.dedupeKey))
    .map((insight) => ({
      id: insight.id,
      status: (insight.type === "bill_due_soon" ? "expired" : "resolved") as FinancialInsightStatus,
    }));
}
