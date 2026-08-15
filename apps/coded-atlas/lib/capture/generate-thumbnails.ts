import path from "node:path";
import sharp from "sharp";
import { AtlasError } from "../errors";
import { thumbnailDir, publicPath as makePublicPath } from "../storage/paths";
import type { DeviceCaptureResult } from "./capture-device";

export interface ThumbnailResult {
  main: string;   // caminho público → catalog.json e UI
  mobile: string; // caminho público → catalog.json e UI
}

export async function generateThumbnails(
  slug: string,
  desktop: DeviceCaptureResult,
  mobile: DeviceCaptureResult
): Promise<ThumbnailResult> {
  const thumbDir = thumbnailDir(slug);
  const mainAbsPath = path.join(thumbDir, "thumb-main.webp");
  const mobileAbsPath = path.join(thumbDir, "thumb-mobile.webp");

  try {
    await sharp(desktop.screenshotAbsPath)
      .resize(640, 400, { fit: "cover", position: "top" })
      .webp({ quality: 82 })
      .toFile(mainAbsPath);
  } catch (err) {
    throw new AtlasError(
      "STORAGE_FAILED",
      "Não foi possível salvar os arquivos gerados.",
      `Failed to generate main thumbnail from "${desktop.screenshotAbsPath}": ${err}`
    );
  }

  try {
    await sharp(mobile.screenshotAbsPath)
      .resize(320, 640, { fit: "cover", position: "top" })
      .webp({ quality: 82 })
      .toFile(mobileAbsPath);
  } catch (err) {
    throw new AtlasError(
      "STORAGE_FAILED",
      "Não foi possível salvar os arquivos gerados.",
      `Failed to generate mobile thumbnail from "${mobile.screenshotAbsPath}": ${err}`
    );
  }

  return {
    main: makePublicPath(slug, "thumbnails", "thumb-main.webp"),
    mobile: makePublicPath(slug, "thumbnails", "thumb-mobile.webp"),
  };
}
