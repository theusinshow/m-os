import { AlertTriangle } from "lucide-react";
import type { SuspicionReason } from "@/lib/suspiciousEntry";
import { SUSPICION_REASON_LABELS } from "@/lib/labels";

interface SuspicionBadgeProps {
  reasons: SuspicionReason[];
}

/**
 * Selo discreto em sessoes com duracao implausivel. So chama atencao — nao
 * bloqueia nada e nao altera o tempo gravado.
 */
export function SuspicionBadge({ reasons }: SuspicionBadgeProps) {
  if (reasons.length === 0) return null;
  const motivos = reasons.map((r) => SUSPICION_REASON_LABELS[r]).join(" · ");
  return (
    <span
      className="inline-flex items-center gap-1 rounded border border-warning/40 px-1.5 py-0.5 text-2xs font-medium text-warning"
      title={motivos}
    >
      <AlertTriangle size={11} strokeWidth={2} aria-hidden />
      Conferir?
    </span>
  );
}
