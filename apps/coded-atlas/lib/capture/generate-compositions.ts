import path from "node:path";
import { promises as fs } from "node:fs";
import sharp from "sharp";
import { config } from "../config";
import { compositionDir, publicPath as makePublicPath } from "../storage/paths";
import type { DeviceCaptureResult } from "./capture-device";

export interface CompositionResult {
  name: string;
  label: string;
  width: number;
  height: number;
  image: string; // caminho público
}

type FormatDef = (typeof config.compositions.formats)[number];

/**
 * Gera as composições para redes (v1.4): a captura centralizada sobre o fundo
 * da marca, com cantos arredondados e sombra, nos formatos definidos no config.
 * Puramente Sharp — sem nova captura. Falhas por formato são silenciosas
 * (composição é enhancement, não pode derrubar a geração do catálogo).
 */
export async function generateCompositions(
  slug: string,
  desktop: DeviceCaptureResult,
  mobile: DeviceCaptureResult
): Promise<CompositionResult[]> {
  const dir = compositionDir(slug);
  await fs.mkdir(dir, { recursive: true });

  const results: CompositionResult[] = [];

  for (const fmt of config.compositions.formats) {
    const src = fmt.source === "mobile" ? mobile.screenshotAbsPath : desktop.screenshotAbsPath;
    const filename = `${fmt.name}.png`;
    const absPath = path.join(dir, filename);
    try {
      await composeFramed(src, fmt, absPath);
      results.push({
        name: fmt.name,
        label: fmt.label,
        width: fmt.width,
        height: fmt.height,
        image: makePublicPath(slug, "compositions", filename),
      });
    } catch (err) {
      console.warn(`[atlas:${slug}] composição "${fmt.name}" falhou: ${err}`);
    }
  }

  return results;
}

/** Compõe uma captura num canvas do formato, com fundo, moldura e sombra. */
async function composeFramed(srcAbsPath: string, fmt: FormatDef, destAbsPath: string): Promise<void> {
  const c = config.compositions;
  const { width: W, height: H } = fmt;

  const pad = Math.round(Math.min(W, H) * c.paddingRatio);
  const boxW = W - 2 * pad;
  const boxH = H - 2 * pad;

  // Captura redimensionada para caber na área útil (alta resolução de origem → downscale nítido).
  const resized = await sharp(srcAbsPath)
    .resize(boxW, boxH, { fit: "inside", withoutEnlargement: false })
    .toBuffer({ resolveWithObject: true });

  const w = resized.info.width;
  const h = resized.info.height;
  const r = Math.min(c.cornerRadius, Math.floor(Math.min(w, h) / 2));

  // Cantos arredondados via máscara SVG.
  const maskSvg = Buffer.from(
    `<svg width="${w}" height="${h}"><rect width="${w}" height="${h}" rx="${r}" ry="${r}" fill="#fff"/></svg>`
  );
  const rounded = await sharp(resized.data)
    .composite([{ input: maskSvg, blend: "dest-in" }])
    .png()
    .toBuffer();

  const left = Math.round((W - w) / 2);
  const top = Math.round((H - h) / 2);

  // Sombra: retângulo arredondado preto, desfocado, atrás da captura.
  const shadowSvg = Buffer.from(
    `<svg width="${W}" height="${H}"><rect x="${left}" y="${top + c.shadowOffsetY}" width="${w}" height="${h}" rx="${r}" ry="${r}" fill="#000" fill-opacity="${c.shadowOpacity}"/></svg>`
  );
  const shadow = await sharp(shadowSvg).blur(c.shadowSigma).png().toBuffer();

  // Canvas com o fundo da marca + sombra + captura.
  await sharp({
    create: { width: W, height: H, channels: 4, background: c.background },
  })
    .composite([
      { input: shadow, top: 0, left: 0 },
      { input: rounded, top, left },
    ])
    .png()
    .toFile(destAbsPath);
}
