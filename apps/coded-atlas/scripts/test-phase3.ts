/**
 * Script de verificação da Fase 3 — captura desktop real.
 * Execute com: npx tsx scripts/test-phase3.ts [url] [slug]
 * Exemplo:     npx tsx scripts/test-phase3.ts https://example.com example-com
 */
import { promises as fs } from "node:fs";
import { chromium } from "playwright";
import { ensureProjectFolder } from "../lib/storage/ensure-project-folder";
import { captureViewport } from "../lib/capture/capture-device";
import { config } from "../lib/config";

const url = process.argv[2] ?? "https://example.com";
const slug = process.argv[3] ?? "test-phase3";

async function main() {
  console.log(`\n[atlas:${slug}] Captura desktop — Fase 3`);
  console.log(`  URL    : ${url}`);
  console.log(`  Saída  : public/generated/${slug}/screenshots/`);
  console.log(`  Delay  : ${config.captureDelayMs}ms  |  Timeout: ${config.navTimeoutMs}ms\n`);

  const t0 = Date.now();

  await ensureProjectFolder(slug);
  console.log("  ✓ Pasta criada");

  const browser = await chromium.launch({
    headless: config.headless,
    args: ["--no-sandbox", "--disable-dev-shm-usage"],
  });

  try {
    console.log("  ✓ Browser aberto");

    const result = await captureViewport(
      browser,
      { url, name: slug, slug, category: "test" },
      config.viewports.desktop
    );

    const stat = await fs.stat(result.absPath);
    const sizeKB = (stat.size / 1024).toFixed(1);

    console.log(`  ✓ Screenshot salvo`);
    console.log(`      Abs  : ${result.absPath}`);
    console.log(`      Pub  : ${result.publicPath}`);
    console.log(`      Size : ${sizeKB} KB`);

    if (stat.size < 5 * 1024) {
      throw new Error(`PNG muito pequeno (${sizeKB} KB) — provavelmente página em branco`);
    }

    if (result.publicPath.includes(config.outputDir)) {
      throw new Error("publicPath contém caminho absoluto do OS — guard rail violado");
    }

    if (!result.publicPath.startsWith("/generated/")) {
      throw new Error("publicPath não começa com /generated/");
    }

    const elapsed = ((Date.now() - t0) / 1000).toFixed(1);
    console.log(`\n  Duração total: ${elapsed}s`);
    console.log("\n✓ Fase 3 — PNG real gerado. Guard rails OK.\n");
  } finally {
    await browser.close();
    console.log("  ✓ Browser fechado");
  }
}

main().catch((err) => {
  console.error("\n✗ Fase 3 falhou:", err instanceof Error ? err.message : err);
  process.exit(1);
});
