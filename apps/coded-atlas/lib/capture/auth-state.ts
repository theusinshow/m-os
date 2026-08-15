import { promises as fs } from "node:fs";
import { authStatePath } from "../storage/paths";

/**
 * Opções de contexto para captura autenticada.
 *
 * Se existir uma sessão salva para o slug (gerada por `scripts/login.mjs`),
 * devolve `{ storageState }` — o Playwright entra já logado (cookies + localStorage).
 * Caso não exista, devolve `{}`: a captura segue como uma visita anônima normal.
 *
 * Espalhe o retorno em cada `browser.newContext({ ... })` que navega no site
 * do usuário.
 */
export async function authContextOptions(
  slug: string,
): Promise<{ storageState?: string }> {
  const p = authStatePath(slug);
  try {
    await fs.access(p);
    return { storageState: p };
  } catch {
    return {};
  }
}
