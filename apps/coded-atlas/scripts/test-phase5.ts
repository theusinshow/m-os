/**
 * Script de verificação da Fase 5 — thumbnails WebP via Sharp.
 * Requer screenshots em public/generated/[slug]/screenshots/ (gerados na Fase 4).
 * Execute com: npx tsx scripts/test-phase5.ts [slug]
 * Padrão: slug = example-com
 */
import { promises as fs } from "node:fs";
import path from "node:path";
import sharp from "sharp";
import { generateThumbnails } from "../lib/capture/generate-thumbnails";
import {
  screenshotDir,
  thumbnailDir,
  publicPath as makePublicPath,
} from "../lib/storage/paths";
import type { DeviceCaptureResult } from "../lib/capture/capture-device";

const slug = process.argv[2] ?? "example-com";

async function main() {
  console.log(`\n[atlas:${slug}] Thumbnails — Fase 5\n`);

  const shotDir = screenshotDir(slug);

  // ── Constrói DeviceCaptureResult apontando para screenshots da Fase 4 ──────
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

  // ── Verifica pré-requisitos ────────────────────────────────────────────────
  for (const [label, p] of [
    ["desktop screenshot", desktop.screenshotAbsPath],
    ["mobile screenshot", mobile.screenshotAbsPath],
  ] as [string, string][]) {
    try {
      await fs.access(p);
      console.log(`  ✓ ${label} encontrado`);
    } catch {
      throw new Error(
        `${label} não encontrado: ${p}\n  Execute "npx tsx scripts/test-phase4.ts" primeiro.`
      );
    }
  }

  // ── Gera thumbnails ────────────────────────────────────────────────────────
  console.log("\n  Gerando thumbnails com Sharp...");
  const thumbnails = await generateThumbnails(slug, desktop, mobile);
  console.log("  Pronto.\n");

  // ── Verifica arquivos gerados ──────────────────────────────────────────────
  const thumbDir = thumbnailDir(slug);

  for (const [label, absPath, pubPath, expW, expH] of [
    ["thumb-main.webp",   path.join(thumbDir, "thumb-main.webp"),   thumbnails.main,   640, 400],
    ["thumb-mobile.webp", path.join(thumbDir, "thumb-mobile.webp"), thumbnails.mobile, 320, 640],
  ] as [string, string, string, number, number][]) {
    const stat = await fs.stat(absPath);
    const kb = (stat.size / 1024).toFixed(1);

    if (stat.size < 1024) throw new Error(`${label} muito pequeno (${kb} KB) — geração falhou`);

    const meta = await sharp(absPath).metadata();
    if (meta.format !== "webp")
      throw new Error(`${label}: formato esperado webp, recebeu ${meta.format}`);
    if (meta.width !== expW || meta.height !== expH)
      throw new Error(
        `${label}: dimensões ${meta.width}×${meta.height}, esperava ${expW}×${expH}`
      );
    if (!pubPath.startsWith("/generated/"))
      throw new Error(`${label}: publicPath inválido: "${pubPath}"`);

    console.log(
      `  ✓ ${label.padEnd(20)} ${String(expW).padStart(3)}×${expH}  ${kb.padStart(6)} KB  →  ${pubPath}`
    );
  }

  console.log(`\n✓ Fase 5 — thumbnails WebP gerados com dimensões e formato corretos.\n`);
}

main().catch((err) => {
  console.error("\n✗ Fase 5 falhou:", err instanceof Error ? err.message : err);
  process.exit(1);
});
