import { promises as fs } from "node:fs";
import sharp from "sharp";
import { coverPath, thumbnailDir, publicPath as makePublicPath } from "../storage/paths";
import { config } from "../config";

export interface CoverResult {
  cover: string;                       // caminho público
  source: "og-image" | "smart-crop";
}

/**
 * Gera a capa do catálogo em duas estratégias (prioridade nessa ordem):
 *
 * 1. OG image — a imagem que o próprio site define como preview
 *    (og:image / twitter:image). Curada pelo dono do site, sempre a melhor
 *    opção quando disponível. Baixada, redimensionada e salva como WebP.
 *
 * 2. Smart crop — Sharp com `position: "attention"`: analisa o screenshot
 *    desktop inteiro e recorta a região de maior saliência visual (frequência
 *    luminosa, saturação e presença de tons de pele). Ótimo fallback para
 *    capturar o hero da página.
 *
 * Saída: 1200 × 630 px WebP (padrão OG, configurável via env).
 */
export async function generateCover(
  slug: string,
  desktopScreenshotAbsPath: string,
  baseUrl: string,
  ogImageUrl?: string
): Promise<CoverResult> {
  await fs.mkdir(thumbnailDir(slug), { recursive: true });

  const { coverWidth: W, coverHeight: H } = config;
  const destPath = coverPath(slug);

  // ── Estratégia 1: OG image ────────────────────────────────────────────────
  if (ogImageUrl) {
    try {
      const buf = await fetchImage(ogImageUrl, baseUrl);
      if (buf) {
        await sharp(buf)
          .resize(W, H, { fit: "cover", position: "attention" })
          .webp({ quality: 88 })
          .toFile(destPath);

        return {
          cover: makePublicPath(slug, "thumbnails", "cover.webp"),
          source: "og-image",
        };
      }
    } catch {
      // cai no fallback
    }
  }

  // ── Estratégia 2: smart crop do screenshot desktop ────────────────────────
  await sharp(desktopScreenshotAbsPath)
    .resize(W, H, { fit: "cover", position: "attention" })
    .webp({ quality: 88 })
    .toFile(destPath);

  return {
    cover: makePublicPath(slug, "thumbnails", "cover.webp"),
    source: "smart-crop",
  };
}

/** Baixa uma URL de imagem (absoluta ou relativa à baseUrl). */
async function fetchImage(url: string, baseUrl: string): Promise<Buffer | null> {
  const resolved = new URL(url, baseUrl).href;
  const res = await fetch(resolved, {
    headers: { "User-Agent": config.userAgent },
    signal: AbortSignal.timeout(8000),
  });
  if (!res.ok) return null;
  const buf = await res.arrayBuffer();
  if (buf.byteLength < 1024) return null; // descarta respostas vazias
  return Buffer.from(buf);
}
