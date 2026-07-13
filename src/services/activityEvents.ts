/**
 * Servico de eventos de atividade (reconstrucao do dia — secao 14).
 */

import type { ActivityEvent } from "@/types/domain";
import { invokeCommand } from "./tauri";

/** Lista eventos no intervalo [from, to) (ISO UTC). */
export function listActivityEvents(
  from: string,
  to: string,
): Promise<ActivityEvent[]> {
  return invokeCommand<ActivityEvent[]>("list_activity_events", { from, to });
}
