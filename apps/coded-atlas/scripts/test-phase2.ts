/**
 * Script de verificação da Fase 2.
 * Execute com: npx tsx scripts/test-phase2.ts
 */
import { promises as fs } from "node:fs";
import path from "node:path";
import { ensureProjectFolder } from "../lib/storage/ensure-project-folder";
import { writeJson } from "../lib/storage/write-json";
import {
  projectDir,
  screenshotDir,
  thumbnailDir,
  catalogPath,
  publicPath,
} from "../lib/storage/paths";
import { config } from "../lib/config";
import { AtlasError } from "../lib/errors";

const TEST_SLUG = "test-phase2";

let passed = 0;
let failed = 0;

async function ok(name: string, fn: () => Promise<void> | void): Promise<void> {
  try {
    await fn();
    console.log(`  ✓  ${name}`);
    passed++;
  } catch (err) {
    console.error(`  ✗  ${name} — ${err}`);
    failed++;
  }
}

async function throws(
  name: string,
  fn: () => Promise<void>,
  expectedCode?: string
): Promise<void> {
  try {
    await fn();
    console.error(`  ✗  ${name} — esperava erro, mas não lançou`);
    failed++;
  } catch (err) {
    if (!(err instanceof AtlasError)) {
      console.error(`  ✗  ${name} — lançou erro não-AtlasError: ${err}`);
      failed++;
      return;
    }
    if (expectedCode && err.code !== expectedCode) {
      console.error(
        `  ✗  ${name} — código esperado "${expectedCode}", recebeu "${err.code}"`
      );
      failed++;
      return;
    }
    console.log(`  ✓  ${name}  →  AtlasError(${err.code})`);
    passed++;
  }
}

async function main() {
  // ── paths ──────────────────────────────────────────────────────────────────
  console.log("\n── paths ────────────────────────────────────────────");

  await ok("projectDir = outputDir/slug", () => {
    const result = projectDir(TEST_SLUG);
    const expected = path.join(config.outputDir, TEST_SLUG);
    if (result !== expected) throw new Error(`"${result}" ≠ "${expected}"`);
  });

  await ok("screenshotDir = projectDir/screenshots", () => {
    const result = screenshotDir(TEST_SLUG);
    const expected = path.join(projectDir(TEST_SLUG), "screenshots");
    if (result !== expected) throw new Error(`"${result}" ≠ "${expected}"`);
  });

  await ok("publicPath gera /generated/slug/...", () => {
    const p = publicPath(TEST_SLUG, "screenshots", "desktop.png");
    const expected = "/generated/test-phase2/screenshots/desktop.png";
    if (p !== expected) throw new Error(`"${p}" ≠ "${expected}"`);
  });

  await ok("publicPath não contém caminho absoluto do OS", () => {
    const p = publicPath(TEST_SLUG, "catalog.json");
    if (p.includes(config.outputDir))
      throw new Error(`Contém caminho absoluto: "${p}"`);
    if (!p.startsWith("/generated/"))
      throw new Error(`Não começa com /generated/: "${p}"`);
  });

  // ── ensureProjectFolder ────────────────────────────────────────────────────
  console.log("\n── ensureProjectFolder ──────────────────────────────");

  await ok("Cria screenshots/ e thumbnails/", async () => {
    await ensureProjectFolder(TEST_SLUG);
    await fs.access(screenshotDir(TEST_SLUG));
    await fs.access(thumbnailDir(TEST_SLUG));
  });

  await ok("Overwrite não lança — recria a pasta limpa", async () => {
    // Deixa um arquivo "órfão" para provar que overwrite limpa
    const orphan = path.join(screenshotDir(TEST_SLUG), "orphan.txt");
    await fs.writeFile(orphan, "stale");
    await ensureProjectFolder(TEST_SLUG);
    // Após overwrite, o órfão não deve mais existir
    try {
      await fs.access(orphan);
      throw new Error("Arquivo órfão sobreviveu ao overwrite");
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
    }
  });

  // ── writeJson ──────────────────────────────────────────────────────────────
  console.log("\n── writeJson ────────────────────────────────────────");

  const testData = { version: "0.1.0", slug: TEST_SLUG, nested: { ok: true } };

  await ok("Escreve JSON no disco", async () => {
    await writeJson(catalogPath(TEST_SLUG), testData);
    await fs.access(catalogPath(TEST_SLUG));
  });

  await ok("JSON lido bate com o escrito", async () => {
    const raw = await fs.readFile(catalogPath(TEST_SLUG), "utf-8");
    const parsed = JSON.parse(raw) as typeof testData;
    if (parsed.slug !== TEST_SLUG) throw new Error("slug diverge");
    if (parsed.version !== "0.1.0") throw new Error("version diverge");
    if (!parsed.nested?.ok) throw new Error("nested diverge");
  });

  await ok("JSON formatado com 2 espaços de indentação", async () => {
    const raw = await fs.readFile(catalogPath(TEST_SLUG), "utf-8");
    if (!raw.includes('  "version"')) throw new Error("JSON não está indentado com 2 espaços");
  });

  await throws(
    "writeJson lança STORAGE_FAILED em caminho inválido",
    async () =>
      writeJson(
        path.join(projectDir(TEST_SLUG), "nao-existe", "sub", "file.json"),
        {}
      ),
    "STORAGE_FAILED"
  );

  // ── limpeza ────────────────────────────────────────────────────────────────
  console.log("\n── limpeza ──────────────────────────────────────────");

  await ok("Remove pasta de teste", async () => {
    await fs.rm(projectDir(TEST_SLUG), { recursive: true, force: true });
    try {
      await fs.access(projectDir(TEST_SLUG));
      throw new Error("Pasta ainda existe após remoção");
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
    }
  });

  // ── resumo ─────────────────────────────────────────────────────────────────
  console.log(`\n${"─".repeat(52)}`);
  console.log(`  ${passed} passou  |  ${failed} falhou\n`);
  if (failed > 0) process.exit(1);
}

main().catch((err) => {
  console.error("Erro inesperado:", err);
  process.exit(1);
});
