import path from "node:path";
import { promises as fs } from "node:fs";
import { config } from "../config";
import { socialImageDir, publicPath as makePublicPath } from "../storage/paths";
import { composeFramed, type FrameStyle } from "./compose";
import type { DeviceCaptureResult } from "../capture/capture-device";
import type { SocialImageAsset } from "../types";

/** Extrai a estética da moldura do config (compartilhada por imagens e vídeos). */
export function socialFrameStyle(): FrameStyle {
  const s = config.social;
  return {
    background: s.background,
    paddingRatio: s.paddingRatio,
    cornerRadius: s.cornerRadius,
    shadowSigma: s.shadowSigma,
    shadowOpacity: s.shadowOpacity,
    shadowOffsetY: s.shadowOffsetY,
  };
}

/**
 * Gera as imagens do kit de redes (v2.0): a captura emoldurada sobre o fundo
 * da marca, nos formatos de imagem definidos no config (Instagram: 1:1, 4:5, 9:16).
 * Puramente Sharp — sem nova captura. Falhas por formato são silenciosas
 * (o kit é enhancement, não pode derrubar a geração do catálogo).
 */
export async function generateSocialImages(
  slug: string,
  desktop: DeviceCaptureResult,
  mobile: DeviceCaptureResult
): Promise<SocialImageAsset[]> {
  const dir = socialImageDir(slug);
  await fs.mkdir(dir, { recursive: true });

  const style = socialFrameStyle();
  const results: SocialImageAsset[] = [];

  for (const fmt of config.social.images) {
    const src = fmt.source === "mobile" ? mobile.screenshotAbsPath : desktop.screenshotAbsPath;
    const filename = `${fmt.name}.png`;
    const absPath = path.join(dir, filename);
    try {
      await composeFramed(src, fmt.width, fmt.height, style, absPath);
      results.push({
        name: fmt.name,
        label: fmt.label,
        platform: fmt.platform,
        width: fmt.width,
        height: fmt.height,
        image: makePublicPath(slug, "social", "images", filename),
      });
    } catch (err) {
      console.warn(`[atlas:${slug}] imagem de rede "${fmt.name}" falhou: ${err}`);
    }
  }

  return results;
}
