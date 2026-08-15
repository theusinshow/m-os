import { promises as fs } from "node:fs";
import path from "node:path";
import { ensureProjectFolder } from "../lib/storage/ensure-project-folder";
import { projectDir, caseDraftPath, screenshotDir, thumbnailDir } from "../lib/storage/paths";

const SLUG = "__test_reprocess__";

let pass = 0, fail = 0;
function ok(label: string, cond: boolean) {
  if (cond) { console.log(`  ✓ ${label}`); pass++; }
  else { console.log(`  ✗ ${label}`); fail++; }
}

async function exists(p: string): Promise<boolean> {
  try { await fs.access(p); return true; } catch { return false; }
}

async function main() {
  await fs.rm(projectDir(SLUG), { recursive: true, force: true });

  // ── 1. Primeira geração — pasta nova ──
  await ensureProjectFolder(SLUG);
  ok("cria screenshots/", await exists(screenshotDir(SLUG)));
  ok("cria thumbnails/", await exists(thumbnailDir(SLUG)));

  // Artefatos: um screenshot e um case-draft autoral
  await fs.writeFile(path.join(screenshotDir(SLUG), "desktop.png"), "fake-png");
  await fs.writeFile(caseDraftPath(SLUG), "# Case autoral\nconteúdo importante");

  // ── 2. Reprocessamento (overwrite) ──
  await ensureProjectFolder(SLUG);
  ok("screenshot antigo removido (capturas regeneradas)",
    !(await exists(path.join(screenshotDir(SLUG), "desktop.png"))));
  ok("screenshots/ recriado vazio", await exists(screenshotDir(SLUG)));

  const draftSurvived = await exists(caseDraftPath(SLUG));
  ok("case-draft.mdx PRESERVADO no reprocessamento", draftSurvived);
  if (draftSurvived) {
    const content = await fs.readFile(caseDraftPath(SLUG), "utf-8");
    ok("conteúdo do case-draft intacto", content.includes("conteúdo importante"));
  }

  // ── 3. Overwrite sem case-draft não quebra ──
  await fs.rm(caseDraftPath(SLUG), { force: true });
  await ensureProjectFolder(SLUG);
  ok("overwrite sem case-draft não quebra", !(await exists(caseDraftPath(SLUG))));

  await fs.rm(projectDir(SLUG), { recursive: true, force: true });
  console.log(`\n${pass}/${pass + fail} passaram.`);
  process.exit(fail === 0 ? 0 : 1);
}

main();
