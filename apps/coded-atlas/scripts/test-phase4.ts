/**
 * Script de verificação da Fase 4 — 4 screenshots (desktop + mobile + fullpages).
 * Execute com: npx tsx scripts/test-phase4.ts [url] [slug]
 * Exemplo:     npx tsx scripts/test-phase4.ts https://example.com example-com
 */
import { promises as fs } from "node:fs";
import { chromium } from "playwright";
import { ensureProjectFolder } from "../lib/storage/ensure-project-folder";
import { captureDevice } from "../lib/capture/capture-device";
import { screenshotDir } from "../lib/storage/paths";
import { config } from "../lib/config";
import type { ProgressEvent } from "../lib/types";

const url = process.argv[2] ?? "https://example.com";
const slug = process.argv[3] ?? "test-phase4";

function onProgress(ev: ProgressEvent) {
  console.log(`  [${String(ev.progress).padStart(3)}%] ${ev.step} — ${ev.message}`);
}

async function assertPng(absPath: string, label: string): Promise<void> {
  const stat = await fs.stat(absPath);
  const kb = (stat.size / 1024).toFixed(1);
  if (stat.size < 5 * 1024) throw new Error(`${label}: PNG muito pequeno (${kb} KB)`);
  console.log(`  ✓ ${label.padEnd(32)} ${kb} KB`);
}

async function main() {
  console.log(`\n[atlas:${slug}] Captura completa — Fase 4`);
  console.log(`  URL : ${url}\n`);

  const t0 = Date.now();
  await ensureProjectFolder(slug);

  const browser = await chromium.launch({
    headless: config.headless,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });

  try {
    // ── Desktop ──────────────────────────────────────────────────────────────
    console.log("── desktop ──────────────────────────────────────────");
    const desktop = await captureDevice(browser, { url, name: slug, slug, category: "test" }, config.viewports.desktop, onProgress);

    // ── Mobile ───────────────────────────────────────────────────────────────
    console.log("\n── mobile ───────────────────────────────────────────");
    const mobile = await captureDevice(browser, { url, name: slug, slug, category: "test" }, config.viewports.mobile, onProgress);

    // ── Verificação dos 4 arquivos ────────────────────────────────────────────
    console.log("\n── verificação ──────────────────────────────────────");
    await assertPng(desktop.screenshotAbsPath, "desktop-1440x900.png");
    await assertPng(desktop.fullpageAbsPath,   "desktop-fullpage.png");
    await assertPng(mobile.screenshotAbsPath,  "mobile-390x844.png");
    await assertPng(mobile.fullpageAbsPath,    "mobile-fullpage.png");

    // Guard rails de caminho público
    for (const [label, p] of [
      ["desktop.screenshot", desktop.screenshot],
      ["desktop.fullpage",   desktop.fullpage],
      ["mobile.screenshot",  mobile.screenshot],
      ["mobile.fullpage",    mobile.fullpage],
    ] as [string, string][]) {
      if (!p.startsWith("/generated/"))
        throw new Error(`${label}: publicPath não começa com /generated/: "${p}"`);
      if (p.includes(config.outputDir))
        throw new Error(`${label}: publicPath contém caminho absoluto do OS`);
    }
    console.log("  ✓ Guard rails de publicPath OK");

    // Verifica estrutura de pastas
    const dir = screenshotDir(slug);
    const files = await fs.readdir(dir);
    if (files.length !== 4)
      throw new Error(`Esperava 4 arquivos em screenshots/, encontrou ${files.length}: ${files.join(", ")}`);
    console.log(`  ✓ screenshots/ contém exatamente 4 arquivos`);

    const elapsed = ((Date.now() - t0) / 1000).toFixed(1);
    console.log(`\n  Duração total: ${elapsed}s`);
    console.log(`\n✓ Fase 4 — 4 screenshots reais gerados.\n`);
  } finally {
    await browser.close();
    console.log("  ✓ Browser fechado");
  }
}

main().catch((err) => {
  console.error("\n✗ Fase 4 falhou:", err instanceof Error ? err.message : err);
  process.exit(1);
});
