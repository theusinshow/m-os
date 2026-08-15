/**
 * Servico de projetos: wrappers tipados sobre os comandos Tauri.
 */

import type { Project, ProjectBilling, ProjectStatus } from "@/types/domain";
import { invokeCommand } from "./tauri";

export interface ProjectInput {
  clientId?: string | null;
  name: string;
  code?: string | null;
  description?: string | null;
  hourlyRateCents: number;
  budgetMinutes?: number;
  color?: string | null;
}

/**
 * Horas e valor acumulados por projeto, sobre todo o historico. O
 * arredondamento vem das Configuracoes lidas no proprio backend — nao ha
 * parametro aqui de proposito, para as duas telas nao poderem divergir.
 */
export function listProjectBilling(): Promise<ProjectBilling[]> {
  return invokeCommand<ProjectBilling[]>("list_project_billing");
}

export function listProjects(includeArchived = false): Promise<Project[]> {
  return invokeCommand<Project[]>("list_projects", { includeArchived });
}

export function getProject(id: string): Promise<Project> {
  return invokeCommand<Project>("get_project", { id });
}

export function createProject(input: ProjectInput): Promise<Project> {
  return invokeCommand<Project>("create_project", { input });
}

export function updateProject(
  id: string,
  input: ProjectInput,
): Promise<Project> {
  return invokeCommand<Project>("update_project", { id, input });
}

export function setProjectStatus(
  id: string,
  status: ProjectStatus,
): Promise<Project> {
  return invokeCommand<Project>("set_project_status", { id, status });
}
