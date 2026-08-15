/**
 * Script de verificação da Fase 6 — buildCatalog + writeJson.
 * Requer screenshots e thumbnails em public/generated/[slug]/ (Fases 4 e 5).
 * Execute com: npx tsx scripts/test-phase6.ts [slug]
 */
import { promises as fs } from "node:fs";
import path from "node:path";
import { buildCatalog } from "../lib/capture/build-catalog";
import { writeJson } from "../lib/storage/write-json";
import {
  screenshotDir,
  thumbnailDir,
  catalogPath,
  publicPath as makePublicPath,
} from "../lib/storage/paths";
import { config } from "../lib/config";
import type { Catalog } from "../lib/types";
import type { DeviceCaptureResult } from "../lib/capture/capture-device";
import type { ThumbnailResult } from "../lib/capture/generate-thumbnails";

const slug = process.argv[2] ?? "example-com";

let passed = 0;
let failed = 0;

function ok(name: string, fn: () => void): void {
  try {
    fn();
    console.log(`  ✓  ${name}`);
    passed++;
  } catch (err) {
    console.error(`  ✗  ${name} — ${err instanceof Error ? err.message : err}`);
    failed++;
  }
}

async function main() {
  console.log(`\n[atlas:${slug}] Catálogo — Fase 6\n`);

  const shotDir = screenshotDir(slug);
  const thumbDir = thumbnailDir(slug);

  const desktop: DeviceCaptureResult = {
    viewport: "1440x900",
    screenshot: makePublicPath(slug, "screenshots", "desktop-1440x900.png"),
    fullpage: makePublicPath(slug, "screenshots", "desktop-fullpage.png"),
    screenshotAbsPath: path.join(shotDir, "desktop-1440x900.png"),
    fullpageAbsPath: path.join(shotDir, "desktop-fullpage.png"),
    sections: [],
  };

  const mobile: DeviceCaptureResult = {
    viewport: "390x844",
    screenshot: makePublicPath(slug, "screenshots", "mobile-390x844.png"),
    fullpage: makePublicPath(slug, "screenshots", "mobile-fullpage.png"),
    screenshotAbsPath: path.join(shotDir, "mobile-390x844.png"),
    fullpageAbsPath: path.join(shotDir, "mobile-fullpage.png"),
    sections: [],
  };

  const thumbnails: ThumbnailResult = {
    main: makePublicPath(slug, "thumbnails", "thumb-main.webp"),
    mobile: makePublicPath(slug, "thumbnails", "thumb-mobile.webp"),
  };

  const input = {
    url: "https://example.com",
    name: "Example",
    slug,
    category: "Site Institucional",
    client: "Example Inc.",
    description: "Página de teste para verificação do Fase 6.",
  };

  const startedAt = Date.now() - 9100; // simula 9.1s de captura

  // ── buildCatalog ───────────────────────────────────────────────────────────
  console.log("── buildCatalog ─────────────────────────────────────");

  const catalog = buildCatalog(input, { desktop, mobile }, thumbnails, undefined, {}, startedAt);

  ok("version = atlasVersion do config", () => {
    if (catalog.version !== config.atlasVersion)
      throw new Error(`"${catalog.version}" ≠ "${config.atlasVersion}"`);
  });

  ok("project bate com input", () => {
    if (catalog.project.url !== input.url) throw new Error("url diverge");
    if (catalog.project.slug !== slug) throw new Error("slug diverge");
    if (catalog.project.name !== input.name) throw new Error("name diverge");
  });

  ok("captures.desktop.viewport = '1440x900'", () => {
    if (catalog.captures.desktop.viewport !== "1440x900")
      throw new Error(`"${catalog.captures.desktop.viewport}"`);
  });

  ok("captures.mobile.viewport = '390x844'", () => {
    if (catalog.captures.mobile.viewport !== "390x844")
      throw new Error(`"${catalog.captures.mobile.viewport}"`);
  });

  ok("sections é array vazio", () => {
    if (!Array.isArray(catalog.sections) || catalog.sections.length !== 0)
      throw new Error(`sections = ${JSON.stringify(catalog.sections)}`);
  });

  ok("videos ausente (não é campo do MVP)", () => {
    if ("videos" in catalog && catalog.videos !== undefined)
      throw new Error("videos presente no catalog (deve estar ausente no MVP)");
  });

  ok("meta.captureDelayMs = config.captureDelayMs", () => {
    if (catalog.meta.captureDelayMs !== config.captureDelayMs)
      throw new Error(`${catalog.meta.captureDelayMs} ≠ ${config.captureDelayMs}`);
  });

  ok("meta.durationMs > 0", () => {
    if (catalog.meta.durationMs <= 0)
      throw new Error(`durationMs = ${catalog.meta.durationMs}`);
  });

  ok("createdAt é ISO 8601 válido", () => {
    const d = new Date(catalog.createdAt);
    if (isNaN(d.getTime())) throw new Error(`"${catalog.createdAt}" não é data válida`);
  });

  // ── Guard rail: sem caminhos absolutos no JSON ─────────────────────────────
  console.log("\n── guard rails ──────────────────────────────────────");

  const jsonStr = JSON.stringify(catalog);

  ok("Nenhum caminho absoluto do OS no catalog.json", () => {
    if (jsonStr.includes(config.outputDir))
      throw new Error(`outputDir encontrado no JSON: "${config.outputDir}"`);
    // Windows drive letters (C:\, D:\, etc.)
    if (/[A-Za-z]:\\/.test(jsonStr))
      throw new Error("Caminho Windows (X:\\) encontrado no JSON");
  });

  ok("Todos os paths começam com /generated/", () => {
    const paths = [
      catalog.captures.desktop.screenshot,
      catalog.captures.desktop.fullpage,
      catalog.captures.mobile.screenshot,
      catalog.captures.mobile.fullpage,
      catalog.thumbnails.main,
      catalog.thumbnails.mobile,
    ];
    for (const p of paths) {
      if (!p.startsWith("/generated/"))
        throw new Error(`Path não começa com /generated/: "${p}"`);
    }
  });

  ok("absPath ausente no catalog (screenshotAbsPath/fullpageAbsPath)", () => {
    if (jsonStr.includes("screenshotAbsPath") || jsonStr.includes("fullpageAbsPath"))
      throw new Error("Campos absolutos vazaram para o catalog.json");
  });

  // ── writeJson → leitura → validação ───────────────────────────────────────
  console.log("\n── writeJson + leitura ──────────────────────────────");

  await writeJson(catalogPath(slug), catalog);
  console.log(`  ✓ catalog.json escrito em ${catalogPath(slug)}`);

  const raw = await fs.readFile(catalogPath(slug), "utf-8");
  const parsed = JSON.parse(raw) as Catalog;

  ok("version sobrevive ao JSON round-trip", () => {
    if (parsed.version !== catalog.version) throw new Error("version diverge");
  });

  ok("project.slug sobrevive ao round-trip", () => {
    if (parsed.project.slug !== slug) throw new Error("slug diverge");
  });

  ok("captures.desktop intacto após round-trip", () => {
    if (parsed.captures.desktop.viewport !== "1440x900") throw new Error("viewport diverge");
    if (parsed.captures.desktop.screenshot !== catalog.captures.desktop.screenshot)
      throw new Error("screenshot diverge");
  });

  ok("thumbnails intactos após round-trip", () => {
    if (parsed.thumbnails.main !== catalog.thumbnails.main)
      throw new Error("thumbnails.main diverge");
  });

  ok("JSON indentado com 2 espaços", () => {
    if (!raw.includes('  "version"'))
      throw new Error("JSON não está indentado — espaços de 2 esperados");
  });

  // ── Resumo ─────────────────────────────────────────────────────────────────
  console.log(`\n${"─".repeat(52)}`);
  console.log(`  ${passed} passou  |  ${failed} falhou\n`);
  if (failed > 0) process.exit(1);
}

main().catch((err) => {
  console.error("Erro inesperado:", err);
  process.exit(1);
});
