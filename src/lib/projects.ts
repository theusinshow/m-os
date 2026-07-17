/**
 * Ajudas de dominio sobre projetos, derivadas do historico de sessoes.
 */

import type { Project, TimeEntry } from "@/types/domain";

/** Quantos projetos recentes retornar por padrao. */
const DEFAULT_RECENT_LIMIT = 3;

/**
 * IDs dos projetos usados mais recentemente, do mais recente para o mais
 * antigo, sem repetir e apenas projetos que ainda existem em `projects`.
 * Usado para pre-selecionar projeto em atalhos (inicio em 1 clique, tempo
 * esquecido).
 */
export function recentProjectIds(
  entries: TimeEntry[],
  projects: Project[],
  limit: number = DEFAULT_RECENT_LIMIT,
): string[] {
  const known = new Set(projects.map((p) => p.id));
  const seen = new Set<string>();
  const result: string[] = [];
  for (const e of entries) {
    if (seen.has(e.projectId) || !known.has(e.projectId)) continue;
    seen.add(e.projectId);
    result.push(e.projectId);
    if (result.length >= limit) break;
  }
  return result;
}
