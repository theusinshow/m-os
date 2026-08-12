/**
 * Rotulos em pt-BR centralizados (secao 18).
 * Mantidos aqui para facilitar internacionalizacao futura sem espalhar texto.
 */

import type {
  ActivityType,
  ProjectStatus,
  TimerStatus,
} from "@/types/domain";
import {
  LONG_SESSION_HOURS,
  type SuspicionReason,
} from "@/lib/suspiciousEntry";

export const ACTIVITY_TYPE_LABELS: Record<ActivityType, string> = {
  drawing: "Desenho",
  detailing: "Detalhamento",
  revision: "Revisao",
  meeting: "Reuniao",
  study: "Estudo",
  other: "Outro",
};

export const PROJECT_STATUS_LABELS: Record<ProjectStatus, string> = {
  active: "Ativo",
  paused: "Pausado",
  completed: "Concluido",
  archived: "Arquivado",
};

export const TIMER_STATUS_LABELS: Record<TimerStatus, string> = {
  running: "Em execucao",
  paused: "Pausado",
};

export const ACTIVITY_TYPE_OPTIONS: { value: ActivityType; label: string }[] =
  Object.entries(ACTIVITY_TYPE_LABELS).map(([value, label]) => ({
    value: value as ActivityType,
    label,
  }));

export const SUSPICION_REASON_LABELS: Record<SuspicionReason, string> = {
  "muito-longa": `Mais de ${LONG_SESSION_HOURS}h em uma unica sessao`,
  madrugada: "Atravessa a madrugada",
};
