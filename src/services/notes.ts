/**
 * Servico de anotacoes e pendencias: wrappers tipados sobre os comandos Tauri.
 */

import type { Project, ProjectTodo } from "@/types/domain";
import { invokeCommand } from "./tauri";

export function updateProjectNotes(
  projectId: string,
  notes: string | null,
): Promise<Project> {
  return invokeCommand<Project>("update_project_notes", { projectId, notes });
}

export function listTodos(): Promise<ProjectTodo[]> {
  return invokeCommand<ProjectTodo[]>("list_todos");
}

export function createTodo(
  projectId: string,
  text: string,
): Promise<ProjectTodo> {
  return invokeCommand<ProjectTodo>("create_todo", { projectId, text });
}

export function setTodoDone(id: string, done: boolean): Promise<ProjectTodo> {
  return invokeCommand<ProjectTodo>("set_todo_done", { id, done });
}

export function updateTodoText(id: string, text: string): Promise<ProjectTodo> {
  return invokeCommand<ProjectTodo>("update_todo_text", { id, text });
}

export function deleteTodo(id: string): Promise<void> {
  return invokeCommand<void>("delete_todo", { id });
}
