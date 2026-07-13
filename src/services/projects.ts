/**
 * Servico de projetos: wrappers tipados sobre os comandos Tauri.
 */

import type { Project, ProjectStatus, ProjectTotal } from "@/types/domain";
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

export function listProjectTotals(): Promise<ProjectTotal[]> {
  return invokeCommand<ProjectTotal[]>("list_project_totals");
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
