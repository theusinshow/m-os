import { promises as fs } from "node:fs";
import { config } from "../config";
import { catalogPath } from "./paths";
import type { Catalog } from "../types";

export interface ProjectSummary {
  slug: string;
  name: string;
  category: string;
  client?: string;
  url: string;
  thumbnail: string;
  createdAt: string;
}

/**
 * Lista todos os projetos gerados lendo as subpastas de public/generated/.
 * Ignora pastas que não tiverem catalog.json válido.
 * Retorna ordenado por data de criação decrescente (mais recente primeiro).
 */
export async function listProjects(): Promise<ProjectSummary[]> {
  let entries: string[];
  try {
    entries = await fs.readdir(config.outputDir);
  } catch {
    return [];
  }

  const results = await Promise.all(
    entries.map(async (entry): Promise<ProjectSummary | null> => {
      try {
        const raw = await fs.readFile(catalogPath(entry), "utf-8");
        const catalog = JSON.parse(raw) as Catalog;
        return {
          slug: entry,
          name: catalog.project.name,
          category: catalog.project.category,
          client: catalog.project.client,
          url: catalog.project.url,
          thumbnail: catalog.thumbnails.main,
          createdAt: catalog.createdAt,
        };
      } catch {
        return null;
      }
    })
  );

  return results
    .filter((p): p is ProjectSummary => p !== null)
    .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
}
