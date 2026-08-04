/**
 * Store de catalogo: clientes e projetos carregados do backend.
 *
 * O banco e a fonte da verdade; este store mantem uma copia em memoria para
 * renderizacao e a mantem sincronizada apos cada operacao. Erros sao expostos
 * de forma legivel (ex.: quando rodando fora do Tauri, no navegador).
 */

import { create } from "zustand";
import type {
  Client,
  Project,
  ProjectBilling,
  ProjectStatus,
} from "@/types/domain";
import {
  archiveClient as apiArchiveClient,
  createClient as apiCreateClient,
  listClients,
  updateClient as apiUpdateClient,
  type ClientInput,
} from "@/services/clients";
import {
  createProject as apiCreateProject,
  listProjectBilling,
  listProjects,
  setProjectStatus as apiSetProjectStatus,
  updateProject as apiUpdateProject,
  type ProjectInput,
} from "@/services/projects";
import { updateProjectNotes as apiUpdateProjectNotes } from "@/services/notes";

interface CatalogState {
  clients: Client[];
  projects: Project[];
  /**
   * Horas e valor acumulados por projeto (id -> totais). Projeto sem nenhuma
   * sessao nao aparece aqui — use `EMPTY_BILLING` como padrao.
   */
  projectBilling: Record<string, ProjectBilling>;
  loading: boolean;
  loaded: boolean;
  error: string | null;

  loadAll: () => Promise<void>;
  /** Recarrega so os acumulados (apos encerrar sessao, editar ou mudar o arredondamento). */
  loadTotals: () => Promise<void>;

  createClient: (input: ClientInput) => Promise<Client>;
  updateClient: (id: string, input: ClientInput) => Promise<Client>;
  archiveClient: (id: string) => Promise<void>;

  createProject: (input: ProjectInput) => Promise<Project>;
  updateProject: (id: string, input: ProjectInput) => Promise<Project>;
  setProjectStatus: (id: string, status: ProjectStatus) => Promise<Project>;
  updateProjectNotes: (projectId: string, notes: string) => Promise<Project>;
}

function messageOf(err: unknown): string {
  return typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
}

const byName = <T extends { name: string }>(a: T, b: T) =>
  a.name.localeCompare(b.name, "pt-BR");

/** Indexa os acumulados vindos do backend pelo id do projeto. */
function byProjectId(items: ProjectBilling[]): Record<string, ProjectBilling> {
  return Object.fromEntries(items.map((item) => [item.projectId, item]));
}

/** Acumulado de um projeto que ainda nao tem nenhuma sessao registrada. */
export const EMPTY_BILLING: Omit<ProjectBilling, "projectId"> = {
  grossSeconds: 0,
  idleSeconds: 0,
  billableSeconds: 0,
  amountCents: 0,
};

export const useCatalogStore = create<CatalogState>((set, get) => ({
  clients: [],
  projects: [],
  projectBilling: {},
  loading: false,
  loaded: false,
  error: null,

  loadAll: async () => {
    set({ loading: true, error: null });
    try {
      const [clients, projects, billing] = await Promise.all([
        listClients(),
        listProjects(),
        listProjectBilling(),
      ]);
      set({
        clients,
        projects,
        projectBilling: byProjectId(billing),
        loading: false,
        loaded: true,
      });
    } catch (err) {
      set({ loading: false, loaded: true, error: messageOf(err) });
    }
  },

  loadTotals: async () => {
    try {
      set({ projectBilling: byProjectId(await listProjectBilling()) });
    } catch {
      // Mantem os totais anteriores em caso de erro pontual.
    }
  },

  createClient: async (input) => {
    const client = await apiCreateClient(input);
    set({ clients: [...get().clients, client].sort(byName) });
    return client;
  },

  updateClient: async (id, input) => {
    const updated = await apiUpdateClient(id, input);
    set({
      clients: get()
        .clients.map((c) => (c.id === id ? updated : c))
        .sort(byName),
    });
    return updated;
  },

  archiveClient: async (id) => {
    await apiArchiveClient(id);
    set({ clients: get().clients.filter((c) => c.id !== id) });
  },

  createProject: async (input) => {
    const project = await apiCreateProject(input);
    set({ projects: [...get().projects, project].sort(byName) });
    return project;
  },

  updateProject: async (id, input) => {
    const updated = await apiUpdateProject(id, input);
    set({
      projects: get()
        .projects.map((p) => (p.id === id ? updated : p))
        .sort(byName),
    });
    return updated;
  },

  setProjectStatus: async (id, status) => {
    const updated = await apiSetProjectStatus(id, status);
    // Projetos arquivados saem da lista padrao.
    const next =
      status === "archived"
        ? get().projects.filter((p) => p.id !== id)
        : get()
            .projects.map((p) => (p.id === id ? updated : p))
            .sort(byName);
    set({ projects: next });
    return updated;
  },

  updateProjectNotes: async (projectId, notes) => {
    const updated = await apiUpdateProjectNotes(projectId, notes.trim() || null);
    set({
      projects: get().projects.map((p) => (p.id === projectId ? updated : p)),
    });
    return updated;
  },
}));
