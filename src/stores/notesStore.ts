/**
 * Store das pendencias por projeto.
 *
 * O banco e a fonte da verdade: cada acao chama um comando e o estado local
 * reflete o retorno. Guarda **todas** as pendencias (abertas e concluidas); quem
 * filtra e ordena para exibicao e `src/lib/todos.ts`.
 */

import { create } from "zustand";
import type { ProjectTodo } from "@/types/domain";
import {
  createTodo as apiCreateTodo,
  deleteTodo as apiDeleteTodo,
  listTodos,
  setTodoDone as apiSetTodoDone,
  updateTodoText as apiUpdateTodoText,
} from "@/services/notes";

interface NotesState {
  todos: ProjectTodo[];
  loaded: boolean;
  error: string | null;

  load: () => Promise<void>;
  createTodo: (projectId: string, text: string) => Promise<void>;
  setTodoDone: (id: string, done: boolean) => Promise<void>;
  updateTodoText: (id: string, text: string) => Promise<void>;
  deleteTodo: (id: string) => Promise<void>;
}

function messageOf(err: unknown): string {
  return typeof err === "string"
    ? err
    : err instanceof Error
      ? err.message
      : String(err);
}

export const useNotesStore = create<NotesState>((set, get) => ({
  todos: [],
  loaded: false,
  error: null,

  load: async () => {
    try {
      const todos = await listTodos();
      set({ todos, loaded: true, error: null });
    } catch (err) {
      set({ loaded: true, error: messageOf(err) });
    }
  },

  createTodo: async (projectId, text) => {
    const todo = await apiCreateTodo(projectId, text);
    set({ todos: [...get().todos, todo] });
  },

  setTodoDone: async (id, done) => {
    const updated = await apiSetTodoDone(id, done);
    set({ todos: get().todos.map((t) => (t.id === id ? updated : t)) });
  },

  updateTodoText: async (id, text) => {
    const updated = await apiUpdateTodoText(id, text);
    set({ todos: get().todos.map((t) => (t.id === id ? updated : t)) });
  },

  deleteTodo: async (id) => {
    await apiDeleteTodo(id);
    set({ todos: get().todos.filter((t) => t.id !== id) });
  },
}));
