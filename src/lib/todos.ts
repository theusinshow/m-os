/**
 * Regra de exibicao das pendencias no Painel (funcao pura, testavel isolada).
 *
 * Um lembrete so serve se encontra o usuario: o Painel mostra as pendencias
 * **abertas** de todos os projetos, e o projeto do cronometro ativo vem primeiro,
 * porque e o contexto de quem esta trabalhando agora.
 */

import type { Project, ProjectTodo } from "@/types/domain";

export interface TodoGroup {
  project: Project;
  todos: ProjectTodo[];
}

export function openTodosByProject(
  todos: ProjectTodo[],
  projects: Project[],
  activeProjectId: string | null,
): TodoGroup[] {
  const open = todos.filter((t) => !t.done);

  const groups: TodoGroup[] = [];
  for (const project of projects) {
    const projectTodos = open.filter((t) => t.projectId === project.id);
    // Pendencias de projetos fora da lista (ex.: arquivados) ficam de fora.
    if (projectTodos.length > 0) groups.push({ project, todos: projectTodos });
  }

  return groups.sort((a, b) => {
    if (a.project.id === activeProjectId) return -1;
    if (b.project.id === activeProjectId) return 1;
    return a.project.name.localeCompare(b.project.name, "pt-BR");
  });
}
