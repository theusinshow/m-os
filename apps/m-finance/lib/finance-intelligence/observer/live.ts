import type { PackInsight } from "@/lib/finance-intelligence/kernel/context-pack";
import { runDetectors } from "@/lib/finance-intelligence/observer/detectors";
import type { FinanceSnapshot } from "@/lib/finance-intelligence/types";

/**
 * Os insights para o contexto do Hermes: detectados AGORA, em memória, sem
 * gravar nada. O Hermes nunca vê insight velho, e uma leitura continua sendo
 * só leitura.
 */
export function liveInsightsFor(snapshot: FinanceSnapshot, previousSafeToSpendCents: number | null): PackInsight[] {
  return runDetectors({ snapshot, previousSafeToSpendCents }).map((observation) => ({
    type: observation.detector,
    severity: observation.severity,
    title: observation.title,
    summary: observation.summary,
    origin: "live" as const,
  }));
}
