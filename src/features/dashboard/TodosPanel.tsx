import { useEffect } from "react";
import { Link } from "react-router-dom";
import { StickyNote } from "lucide-react";
import { useNotesStore } from "@/stores/notesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { useTimerStore } from "@/stores/timerStore";
import { openTodosByProject } from "@/lib/todos";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Checkbox } from "@/components/ui/Checkbox";
import { EmptyState } from "@/components/ui/EmptyState";
import { ROUTES } from "@/app/routes";

/**
 * Pendencias abertas de todos os projetos. Um lembrete so serve se encontra o
 * usuario — por isso ele mora no Painel, e nao escondido dentro do projeto.
 * O projeto do cronometro ativo vem primeiro e destacado.
 */
export function TodosPanel() {
  const todos = useNotesStore((s) => s.todos);
  const loaded = useNotesStore((s) => s.loaded);
  const load = useNotesStore((s) => s.load);
  const setTodoDone = useNotesStore((s) => s.setTodoDone);
  const projects = useCatalogStore((s) => s.projects);
  const activeTimer = useTimerStore((s) => s.activeTimer);

  useEffect(() => {
    if (!loaded) void load();
  }, [loaded, load]);

  const activeProjectId = activeTimer?.projectId ?? null;
  const groups = openTodosByProject(todos, projects, activeProjectId);

  return (
    <Panel>
      <PanelHeader title="Pendencias" />
      {groups.length === 0 ? (
        <div className="p-4">
          <EmptyState
            title="Nenhuma pendencia"
            description="Anote lembretes de cada projeto na tela de Projetos."
            action={
              <Link
                to={ROUTES.projects}
                className="text-sm text-accent hover:underline"
              >
                Ir para Projetos
              </Link>
            }
          />
        </div>
      ) : (
        <div className="divide-y divide-border">
          {groups.map(({ project, todos: items }) => {
            const isActive = project.id === activeProjectId;
            return (
              <div
                key={project.id}
                className={
                  isActive
                    ? "border-l-2 border-l-accent px-4 py-3"
                    : "px-4 py-3"
                }
              >
                <div className="flex items-center gap-1.5">
                  <p
                    className={
                      isActive
                        ? "text-sm font-medium text-text"
                        : "text-sm text-text-muted"
                    }
                  >
                    {project.code
                      ? `${project.code} · ${project.name}`
                      : project.name}
                  </p>
                  {project.notes && (
                    <StickyNote
                      size={13}
                      strokeWidth={1.75}
                      className="shrink-0 text-text-subtle"
                      aria-label="Este projeto tem anotacoes"
                    />
                  )}
                </div>
                <ul className="mt-1.5 space-y-1.5">
                  {items.map((todo) => (
                    <li key={todo.id}>
                      <Checkbox
                        label={todo.text}
                        checked={false}
                        onChange={() => void setTodoDone(todo.id, true)}
                      />
                    </li>
                  ))}
                </ul>
              </div>
            );
          })}
        </div>
      )}
    </Panel>
  );
}
