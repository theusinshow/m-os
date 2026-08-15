import sharp from "sharp";
import pixelmatch from "pixelmatch";

export interface DiffResult {
  width: number;
  height: number;
  changedPixels: number;
  totalPixels: number;
  percent: number; // 0–100
}

/**
 * Compara duas imagens (antes/depois) pixel a pixel e escreve uma imagem de
 * diferença em `diffAbs` (regiões alteradas destacadas). Usa Sharp para decodar
 * para RGBA cru e pixelmatch para o diff. Se as dimensões diferirem (a página
 * mudou de altura), a imagem "depois" é redimensionada para a base.
 */
export async function computeVisualDiff(
  beforeAbs: string,
  afterAbs: string,
  diffAbs: string
): Promise<DiffResult> {
  const a = await sharp(beforeAbs).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const { width, height } = a.info;

  let b = await sharp(afterAbs).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  if (b.info.width !== width || b.info.height !== height) {
    b = await sharp(afterAbs)
      .resize(width, height, { fit: "fill" })
      .ensureAlpha()
      .raw()
      .toBuffer({ resolveWithObject: true });
  }

  const diff = Buffer.alloc(width * height * 4);
  const changedPixels = pixelmatch(a.data, b.data, diff, width, height, {
    threshold: 0.1,
    alpha: 0.3,
    diffColor: [255, 90, 60], // destaque em vermelho-coral
  });

  await sharp(diff, { raw: { width, height, channels: 4 } }).png().toFile(diffAbs);

  const totalPixels = width * height;
  return {
    width,
    height,
    changedPixels,
    totalPixels,
    percent: (changedPixels / totalPixels) * 100,
  };
}
