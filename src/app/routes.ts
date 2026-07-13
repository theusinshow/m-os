/**
 * Definicao central das rotas e itens de navegacao do MVP (secao 13).
 * Mantido separado dos componentes para reuso na navegacao e no roteador.
 */

import {
  Clock,
  FolderKanban,
  History,
  CalendarClock,
  FileBarChart,
  Settings,
  type LucideIcon,
} from "lucide-react";

export interface NavItem {
  path: string;
  label: string;
  icon: LucideIcon;
}

export const ROUTES = {
  dashboard: "/",
  projects: "/projetos",
  history: "/historico",
  timeline: "/linha-do-tempo",
  reports: "/relatorios",
  settings: "/configuracoes",
} as const;

export const NAV_ITEMS: NavItem[] = [
  { path: ROUTES.dashboard, label: "Painel", icon: Clock },
  { path: ROUTES.projects, label: "Projetos", icon: FolderKanban },
  { path: ROUTES.history, label: "Historico", icon: History },
  { path: ROUTES.timeline, label: "Linha do tempo", icon: CalendarClock },
  { path: ROUTES.reports, label: "Relatorios", icon: FileBarChart },
  { path: ROUTES.settings, label: "Configuracoes", icon: Settings },
];
