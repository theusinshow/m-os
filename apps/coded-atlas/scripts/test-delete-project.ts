import { promises as fs } from "node:fs";
import path from "node:path";
import { deleteProject } from "../lib/storage/delete-project";
import { projectDir } from "../lib/storage/paths";
import { config } from "../lib/config";

let pass = 0, fail = 0;
function ok(label: string, cond: boolean) {
  if (cond) { console.log(`  ✓ ${label}`); pass++; }
  else { console.log(`  ✗ ${label}`); fail++; }
}
async function exists(p: string) {
  try { await fs.access(p); return true; } catch { return false; }
}
async function threw(fn: () => Promise<unknown>) {
  try { await fn(); return false; } catch { return true; }
}

async function main() {
  const SLUG = "zz-test-delete";

  // ── Remove um projeto válido ──
  await fs.mkdir(projectDir(SLUG), { recursive: true });
  await fs.writeFile(path.join(projectDir(SLUG), "catalog.json"), "{}");
  ok("pasta criada", await exists(projectDir(SLUG)));
  await deleteProject(SLUG);
  ok("deleteProject remove a pasta", !(await exists(projectDir(SLUG))));

  // ── Sentinela fora do outputDir que NÃO pode ser tocada ──
  const sentinel = path.join(path.resolve(config.outputDir), "..", "__sentinela__.txt");
  await fs.writeFile(sentinel, "não me apague");

  // ── Slugs maliciosos / inválidos são rejeitados ──
  for (const bad of ["..", "../..", "../__sentinela__", "a/b", "Bad Slug", "foo/../../bar", ""]) {
    ok(`rejeita slug inválido: ${JSON.stringify(bad)}`, await threw(() => deleteProject(bad)));
  }

  ok("sentinela fora do outputDir intacta", await exists(sentinel));
  await fs.rm(sentinel, { force: true });

  console.log(`\n${pass}/${pass + fail} passaram.`);
  process.exit(fail === 0 ? 0 : 1);
}

main();
