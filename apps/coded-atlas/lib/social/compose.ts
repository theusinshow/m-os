import sharp from "sharp";

/** Estética da moldura usada nos assets de rede (imagens e posters de vídeo). */
export interface FrameStyle {
  background: string;
  paddingRatio: number;
  cornerRadius: number;
  shadowSigma: number;
  shadowOpacity: number;
  shadowOffsetY: number;
}

/**
 * Compõe uma captura num canvas WxH, centralizada sobre o fundo da marca,
 * com cantos arredondados e sombra suave. Puramente Sharp — sem nova captura.
 *
 * A captura é encaixada com `fit: "inside"` (aparece inteira, sem corte); por
 * isso o formato vertical usa a captura mobile como origem e o quadrado/horizontal
 * a desktop — cada uma preenche melhor seu quadro.
 */
export async function composeFramed(
  srcAbsPath: string,
  W: number,
  H: number,
  style: FrameStyle,
  destAbsPath: string
): Promise<void> {
  const pad = Math.round(Math.min(W, H) * style.paddingRatio);
  const boxW = W - 2 * pad;
  const boxH = H - 2 * pad;

  // Captura redimensionada para caber na área útil (downscale nítido de origem 2×/3×).
  const resized = await sharp(srcAbsPath)
    .resize(boxW, boxH, { fit: "inside", withoutEnlargement: false })
    .toBuffer({ resolveWithObject: true });

  const w = resized.info.width;
  const h = resized.info.height;
  const r = Math.min(style.cornerRadius, Math.floor(Math.min(w, h) / 2));

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
    `<svg width="${W}" height="${H}"><rect x="${left}" y="${top + style.shadowOffsetY}" width="${w}" height="${h}" rx="${r}" ry="${r}" fill="#000" fill-opacity="${style.shadowOpacity}"/></svg>`
  );
  const shadow = await sharp(shadowSvg).blur(style.shadowSigma).png().toBuffer();

  // Canvas com o fundo da marca + sombra + captura.
  await sharp({
    create: { width: W, height: H, channels: 4, background: style.background },
  })
    .composite([
      { input: shadow, top: 0, left: 0 },
      { input: rounded, top, left },
    ])
    .png()
    .toFile(destAbsPath);
}
