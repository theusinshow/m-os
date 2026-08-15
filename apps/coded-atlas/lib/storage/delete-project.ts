import { promises as fs } from "node:fs";
import path from "node:path";
import { config } from "../config";
import { projectDir } from "./paths";
import { AtlasError } from "../errors";

/**
 * Remove por completo a pasta de um projeto gerado.
 * Valida o slug e confina a operação dentro de outputDir — nunca apaga
 * nada fora de public/generated, mesmo com slug malicioso (path traversal).
 */
export async function deleteProject(slug: string): Promise<void> {
  if (!/^[a-z0-9-]+$/.test(slug)) {
    throw new AtlasError("STORAGE_FAILED", "Slug inválido.", `Bad slug: ${slug}`);
  }

  const root = path.resolve(config.outputDir);
  const dir = path.resolve(projectDir(slug));

  if (dir === root || !dir.startsWith(root + path.sep)) {
    throw new AtlasError(
      "STORAGE_FAILED",
      "Caminho inválido.",
      `Refusing to delete outside output dir: ${dir}`
    );
  }

  await fs.rm(dir, { recursive: true, force: true });
}
