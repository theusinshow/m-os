import { useEffect, useState, type FormEvent } from "react";
import { Plus, Trash2 } from "lucide-react";
import type { Project } from "@/types/domain";
import { useNotesStore } from "@/stores/notesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { Input } from "@/components/ui/Field";

interface ProjectNotesModalProps {
  /** Projeto em edicao; `null` mantem o modal fechado. */
  project: Project | null;
  onClose: () => void;
}

/**
 * Anotacoes (texto livre) e pendencias (checklist) de um projeto.
 *
 * As anotacoes sao um campo do proprio projeto e salvam ao sair do campo; as
 * pendencias vivem no notesStore. Nenhuma pendencia dispara notificacao: elas
 * apenas ficam visiveis (aqui e no Painel).
 */
export function ProjectNotesModal({
  project,
  onClose,
}: ProjectNotesModalProps) {
  const todos = useNotesStore((s) => s.todos);
  const createTodo = useNotesStore((s) => s.createTodo);
  const setTodoDone = useNotesStore((s) => s.setTodoDone);
  const deleteTodo = useNotesStore((s) => s.deleteTodo);
  const updateProjectNotes = useCatalogStore((s) => s.updateProjectNotes);

  const [notes, setNotes] = useState("");
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Recarrega o rascunho ao trocar de projeto.
  useEffect(() => {
    setNotes(project?.notes ?? "");
    setText("");
    setError(null);
  }, [project]);

  if (!project) return null;

  const mine = todos.filter((t) => t.projectId === project.id);
  const open = mine.filter((t) => !t.done);
  const done = mine.filter((t) => t.done);

  async function run(action: () => Promise<unknown>) {
    setError(null);
    try {
      await action();
    } catch (err) {
      setError(typeof err === "string" ? err : "Operacao falhou.");
    }
  }

  async function saveNotes() {
    if (!project || notes === (project.notes ?? "")) return;
    const target = project;
    await run(() => updateProjectNotes(target.id, notes));
  }

  async function addTodo(e: FormEvent) {
    e.preventDefault();
    if (!project || !text.trim()) return;
    const target = project;
    await run(() => createTodo(target.id, text));
    setText("");
  }

  return (
    <Modal
      open
      title={`Anotacoes — ${project.name}`}
      onClose={onClose}
      footer={
        <Button variant="primary" onClick={onClose}>
          Fechar
        </Button>
      }
    >
      <label
        htmlFor="proj-notes"
        className="text-2xs uppercase tracking-wide text-text-subtle"
      >
        Anotacoes
      </label>
      <textarea
        id="proj-notes"
        value={notes}
        onChange={(e) => setNotes(e.target.value)}
        onBlur={() => void saveNotes()}
        rows={4}
        placeholder="Contexto, recados do cliente, o que esta pendente de terceiros…"
        className="mt-1.5 w-full rounded border border-border bg-surface-raised px-3 py-2 text-sm text-text placeholder:text-text-subtle focus:border-accent focus:outline-none"
      />

      <p className="mt-6 text-2xs uppercase tracking-wide text-text-subtle">
        Pendencias
      </p>

      <form onSubmit={(e) => void addTodo(e)} className="mt-1.5 flex gap-2">
        <Input
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="O que precisa ser feito?"
        />
        <Button
          type="submit"
          variant="secondary"
          disabled={!text.trim()}
          icon={<Plus size={16} strokeWidth={2} />}
        >
          Adicionar
        </Button>
      </form>

      {open.length === 0 && done.length === 0 ? (
        <p className="mt-4 text-sm text-text-muted">
          Nenhuma pendencia neste projeto.
        </p>
      ) : (
        <ul className="mt-3 divide-y divide-border">
          {open.map((todo) => (
            <li key={todo.id} className="flex items-center gap-3 py-2">
              <Checkbox
                label=""
                ariaLabel={`Concluir ${todo.text}`}
                checked={false}
                onChange={() => void run(() => setTodoDone(todo.id, true))}
              />
              <span className="flex-1 text-sm text-text">{todo.text}</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => void run(() => deleteTodo(todo.id))}
                aria-label={`Excluir ${todo.text}`}
                icon={<Trash2 size={15} strokeWidth={1.75} />}
              />
            </li>
          ))}
        </ul>
      )}

      {done.length > 0 && (
        <details className="mt-4">
          <summary className="cursor-pointer text-xs text-text-muted">
            Concluidas ({done.length})
          </summary>
          <ul className="mt-2 divide-y divide-border">
            {done.map((todo) => (
              <li key={todo.id} className="flex items-center gap-3 py-2">
                <Checkbox
                  label=""
                  ariaLabel={`Reabrir ${todo.text}`}
                  checked
                  onChange={() => void run(() => setTodoDone(todo.id, false))}
                />
                <span className="flex-1 text-sm text-text-subtle line-through">
                  {todo.text}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => void run(() => deleteTodo(todo.id))}
                  aria-label={`Excluir ${todo.text}`}
                  icon={<Trash2 size={15} strokeWidth={1.75} />}
                />
              </li>
            ))}
          </ul>
        </details>
      )}

      {error && <p className="mt-3 text-sm text-danger">{error}</p>}
    </Modal>
  );
}
