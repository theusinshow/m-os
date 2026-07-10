/**
 * Verificação da Fase 1 (kit de redes — imagens).
 * Gera um screenshot sintético desktop e mobile, roda generateSocialImages e
 * confere que cada formato saiu com as dimensões exatas do config.
 */
import path from "node:path";
import { promises as fs } from "node:fs";
import sharp from "sharp";
import { config } from "../lib/config";
import { generateSocialImages } from "../lib/social/generate-images";
import { socialImageDir } from "../lib/storage/paths";
import type { DeviceCaptureResult } from "../lib/capture/capture-device";

const SLUG = "__test-social__";

function fakeDevice(absPath: string): DeviceCaptureResult {
  return {
    viewport: "x",
    screenshot: "/x",
    fullpage: "/x",
    screenshotAbsPath: absPath,
    fullpageAbsPath: absPath,
    sections: [],
  };
}

async function main() {
  let pass = 0;
  let fail = 0;
  const ok = (cond: boolean, msg: string) => {
    if (cond) { pass++; console.log(`  ✓ ${msg}`); }
    else { fail++; console.error(`  ✗ ${msg}`); }
  };

  const dir = path.join(config.outputDir, SLUG);
  await fs.rm(dir, { recursive: true, force: true });
  await fs.mkdir(dir, { recursive: true });

  // Screenshots sintéticos: desktop 1440×900 (largo), mobile 390×844 (alto).
  const deskSrc = path.join(dir, "src-desktop.png");
  const mobSrc = path.join(dir, "src-mobile.png");
  await sharp({ create: { width: 1440, height: 900, channels: 3, background: "#3a6df0" } }).png().toFile(deskSrc);
  await sharp({ create: { width: 390, height: 844, channels: 3, background: "#f0763a" } }).png().toFile(mobSrc);

  const results = await generateSocialImages(SLUG, fakeDevice(deskSrc), fakeDevice(mobSrc));

  ok(results.length === config.social.images.length, `gerou ${results.length}/${config.social.images.length} formatos`);

  for (const fmt of config.social.images) {
    const asset = results.find((r) => r.name === fmt.name);
    ok(!!asset, `formato "${fmt.name}" presente`);
    if (!asset) continue;
    ok(asset.image === `/generated/${SLUG}/social/images/${fmt.name}.png`, `caminho público de "${fmt.name}" correto`);
    ok(asset.image.startsWith("/generated/") && !asset.image.includes("\\") && !/^[a-zA-Z]:/.test(asset.image), `caminho de "${fmt.name}" é público (sem absoluto/backslash)`);

    const filePath = path.join(socialImageDir(SLUG), `${fmt.name}.png`);
    const meta = await sharp(filePath).metadata();
    ok(meta.width === fmt.width && meta.height === fmt.height, `"${fmt.name}" = ${meta.width}×${meta.height} (esperado ${fmt.width}×${fmt.height})`);
    ok(asset.platform === fmt.platform, `plataforma de "${fmt.name}" = ${asset.platform}`);
  }

  await fs.rm(dir, { recursive: true, force: true });

  console.log(`\n${pass} passaram, ${fail} falharam`);
  process.exit(fail ? 1 : 0);
}

main().catch((err) => { console.error(err); process.exit(1); });
