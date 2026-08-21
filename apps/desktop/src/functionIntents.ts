import type { FunctionDefinition } from "./types";

export type FunctionIntentTarget =
  | "home_capture"
  | "quick_capture"
  | "inbox_process"
  | "inbox_create_task"
  | "tasks_create"
  | "tasks_move"
  | "projects_create"
  | "library_create"
  | "inbox_create_resource"
  | "workspaces_create"
  | "workspaces_link_project"
  | "workspaces_link_app"
  | "home_arrange"
  | "apps_register"
  | "attention_create"
  | "daily_start"
  | "daily_view"
  | "daily_add_objective"
  | "daily_end"
  | "updates_check"
  | "function_registry";

const lowRiskTargets: Readonly<Record<string, FunctionIntentTarget>> = {
  "capture.create": "home_capture",
  "capture.quick_open": "quick_capture",
  "capture.mark_processed": "inbox_process",
  "task.create": "tasks_create",
  "task.create_from_capture": "inbox_create_task",
  "task.set_state": "tasks_move",
  "project.create": "projects_create",
  "resource.create": "library_create",
  "resource.create_from_capture": "inbox_create_resource",
  "workspace.create": "workspaces_create",
  "workspace.link_project": "workspaces_link_project",
  "workspace.link_app": "workspaces_link_app",
  "home.set_widget": "home_arrange",
  "app.register": "apps_register",
  "attention.create_reminder": "attention_create",
  /* As cinco do dia. Elas ROTEIAM para sobreposições, e nao para uma pagina:
     comecar e encerrar o dia sao gestos de trinta segundos, e tirar a pessoa de
     onde ela estava para fazer isso e exatamente a interrupcao que o §85 do
     UX-PRINCIPLES manda medir e reduzir — o mesmo motivo pelo qual o compositor
     de lembrete e sobreposicao. */
  "daily.start_day": "daily_start",
  "daily.view_today": "daily_view",
  "daily.add_objective": "daily_add_objective",
  /* Concluir, promover e resolver acontecem DENTRO da sessao: sao gestos sobre
     um objetivo que a pessoa precisa ver para escolher, e um comando que
     perguntasse "qual objetivo?" numa caixa de texto seria pior que abrir a
     lista onde ele esta. */
  "daily.set_objective_status": "daily_view",
  "daily.set_main": "daily_view",
  "daily.end_day": "daily_end",
  "system.update_check": "updates_check",
};

export function resolveFunctionTarget(definition: FunctionDefinition): FunctionIntentTarget {
  if (definition.risk !== "low") return "function_registry";
  return lowRiskTargets[definition.id] ?? "function_registry";
}
