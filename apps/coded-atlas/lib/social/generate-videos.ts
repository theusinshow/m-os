import path from "node:path";
import { promises as fs } from "node:fs";
import { config } from "../config";
import { socialVideoDir, publicPath as makePublicPath } from "../storage/paths";
import { ffmpegAvailable, runFfmpeg } from "./ffmpeg";
import type { DeviceCaptureResult } from "../capture/capture-device";
import type { SocialVideoAsset } from "../types";

/** Cor de fundo da marca no formato hex que o filtro `pad` do ffmpeg aceita (0xRRGGBB). */
function padColor(): string {
  return "0x" + config.social.background.replace(/^#/, "");
}

/** Arredonda para o par mais próximo (yuv420p exige dimensões pares). */
const even = (n: number) => Math.max(2, Math.round(n / 2) * 2);

/**
 * Gera os vídeos do kit de redes (v2.0): a gravação de scroll enquadrada sobre
 * o fundo da marca, nos formatos de vídeo do config (Instagram: 9:16, 1:1),
 * como MP4 H.264 pronto para postar.
 *
 * Requer ffmpeg. Sem ele, retorna [] silenciosamente — o kit de imagem continua
 * valendo e a geração do catálogo não é derrubada. Falha por formato é isolada.
 */
export async function generateSocialVideos(
  slug: string,
  desktop: DeviceCaptureResult,
  mobile: DeviceCaptureResult
): Promise<SocialVideoAsset[]> {
  if (!(await ffmpegAvailable())) {
    console.warn(`[atlas:${slug}] ffmpeg indisponível — kit de vídeo pulado. Defina ATLAS_FFMPEG_PATH ou instale o ffmpeg.`);
    return [];
  }

  const dir = socialVideoDir(slug);
  await fs.mkdir(dir, { recursive: true });

  const s = config.social;
  const results: SocialVideoAsset[] = [];

  for (const fmt of s.videos) {
    const srcVideo = fmt.source === "mobile" ? mobile.videoAbsPath : desktop.videoAbsPath;
    if (!srcVideo) {
      // A gravação de scroll dessa origem não existe (vídeo desabilitado na geração).
      continue;
    }

    const W = even(fmt.width);
    const H = even(fmt.height);
    const pad = Math.round(Math.min(W, H) * s.paddingRatio);
    const boxW = even(W - 2 * pad);
    const boxH = even(H - 2 * pad);

    const filename = `${fmt.name}.mp4`;
    const absPath = path.join(dir, filename);
    const posterName = `${fmt.name}.jpg`;
    const posterAbs = path.join(dir, posterName);

    // Encaixa o vídeo na área útil preservando proporção; preenche o resto com o fundo da marca.
    const vf =
      `scale=${boxW}:${boxH}:force_original_aspect_ratio=decrease,` +
      `pad=${W}:${H}:(ow-iw)/2:(oh-ih)/2:${padColor()}`;

    try {
      await runFfmpeg([
        "-y",
        "-i", srcVideo,
        "-t", String(s.videoMaxDurationS),
        "-vf", vf,
        "-r", String(s.videoFps),
        "-an",
        "-c:v", "libx264",
        "-preset", "medium",
        "-crf", String(s.videoCrf),
        "-pix_fmt", "yuv420p",
        "-movflags", "+faststart",
        absPath,
      ]);

      // Poster: um frame perto do início, para o <video> exibir antes do play.
      let poster: string | undefined;
      try {
        await runFfmpeg(["-y", "-ss", "0.3", "-i", absPath, "-frames:v", "1", "-q:v", "3", posterAbs]);
        poster = makePublicPath(slug, "social", "videos", posterName);
      } catch (err) {
        console.warn(`[atlas:${slug}] poster de "${fmt.name}" falhou: ${err}`);
      }

      results.push({
        name: fmt.name,
        label: fmt.label,
        platform: fmt.platform,
        width: fmt.width,
        height: fmt.height,
        video: makePublicPath(slug, "social", "videos", filename),
        ...(poster ? { poster } : {}),
      });
    } catch (err) {
      console.warn(`[atlas:${slug}] vídeo de rede "${fmt.name}" falhou: ${err}`);
    }
  }

  return results;
}
