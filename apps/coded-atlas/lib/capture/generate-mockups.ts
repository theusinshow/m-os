import path from "node:path";
import { promises as fs } from "node:fs";
import sharp from "sharp";
import { config } from "../config";
import { mockupDir, publicPath as makePublicPath } from "../storage/paths";
import type { DeviceCaptureResult } from "./capture-device";

export interface MockupResult {
  name: string;
  label: string;
  image: string; // caminho público
}

const TRANSPARENT = { r: 0, g: 0, b: 0, alpha: 0 } as const;

/**
 * Gera mockups (v1.4): a captura dentro de uma moldura desenhada via SVG —
 * janela de navegador (desktop) e corpo de celular (mobile). Fundo transparente
 * para reuso em qualquer layout. Falhas por mockup são silenciosas (enhancement).
 */
export async function generateMockups(
  slug: string,
  desktop: DeviceCaptureResult,
  mobile: DeviceCaptureResult
): Promise<MockupResult[]> {
  const dir = mockupDir(slug);
  await fs.mkdir(dir, { recursive: true });

  const results: MockupResult[] = [];

  const jobs: { name: string; label: string; run: () => Promise<void> }[] = [
    {
      name: "browser",
      label: "Navegador",
      run: () => makeBrowserMockup(desktop.screenshotAbsPath, path.join(dir, "browser.png")),
    },
    {
      name: "phone",
      label: "Celular",
      run: () => makePhoneMockup(mobile.screenshotAbsPath, path.join(dir, "phone.png")),
    },
  ];

  for (const job of jobs) {
    try {
      await job.run();
      results.push({ name: job.name, label: job.label, image: makePublicPath(slug, "mockups", `${job.name}.png`) });
    } catch (err) {
      console.warn(`[atlas:${slug}] mockup "${job.name}" falhou: ${err}`);
    }
  }

  return results;
}

/** Coloca o "device" (já pronto, com alpha) num canvas transparente com sombra. */
async function placeWithShadow(
  device: Buffer,
  deviceW: number,
  deviceH: number,
  radius: number
): Promise<sharp.Sharp> {
  const m = config.mockups;
  const finalW = deviceW + 2 * m.pad;
  const finalH = deviceH + 2 * m.pad;

  const shadowSvg = Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${finalW}" height="${finalH}">` +
      `<rect x="${m.pad}" y="${m.pad + m.shadowOffsetY}" width="${deviceW}" height="${deviceH}" ` +
      `rx="${radius}" ry="${radius}" fill="#000" fill-opacity="${m.shadowOpacity}"/></svg>`
  );
  const shadow = await sharp(shadowSvg).blur(m.shadowSigma).png().toBuffer();

  return sharp({ create: { width: finalW, height: finalH, channels: 4, background: TRANSPARENT } }).composite([
    { input: shadow, top: 0, left: 0 },
    { input: device, top: m.pad, left: m.pad },
  ]);
}

/** Janela de navegador: chrome (3 pontos + barra) no topo, captura no corpo. */
async function makeBrowserMockup(srcAbsPath: string, destAbsPath: string): Promise<void> {
  const b = config.mockups.browser;
  const W = b.width;

  // Captura redimensionada à largura da janela.
  const shot = await sharp(srcAbsPath)
    .resize(W, null, { withoutEnlargement: false })
    .toBuffer({ resolveWithObject: true });
  const bodyH = shot.info.height;
  const r = b.radius;

  // Cantos inferiores arredondados na captura (os de cima encostam no chrome).
  const bottomMask = Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${bodyH}">` +
      `<path d="M0 0 H ${W} V ${bodyH - r} Q ${W} ${bodyH} ${W - r} ${bodyH} H ${r} Q 0 ${bodyH} 0 ${bodyH - r} Z" fill="#fff"/></svg>`
  );
  const body = await sharp(shot.data).composite([{ input: bottomMask, blend: "dest-in" }]).png().toBuffer();

  // Barra do navegador (cantos superiores arredondados).
  const ch = b.chromeHeight;
  const cy = ch / 2;
  const pillX = Math.round(W * 0.3);
  const pillW = Math.round(W * 0.4);
  const chromeSvg = Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${ch}">` +
      `<path d="M ${r} 0 H ${W - r} Q ${W} 0 ${W} ${r} V ${ch} H 0 V ${r} Q 0 0 ${r} 0 Z" fill="${b.chrome}"/>` +
      `<circle cx="24" cy="${cy}" r="6" fill="${b.dots[0]}"/>` +
      `<circle cx="46" cy="${cy}" r="6" fill="${b.dots[1]}"/>` +
      `<circle cx="68" cy="${cy}" r="6" fill="${b.dots[2]}"/>` +
      `<rect x="${pillX}" y="${Math.round(ch * 0.28)}" width="${pillW}" height="${Math.round(ch * 0.44)}" rx="${Math.round(ch * 0.22)}" fill="${b.addressBar}"/></svg>`
  );

  const winW = W;
  const winH = ch + bodyH;

  // Borda fina ao redor da janela inteira.
  const borderSvg = Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${winW}" height="${winH}">` +
      `<rect x="0.5" y="0.5" width="${winW - 1}" height="${winH - 1}" rx="${r}" ry="${r}" fill="none" stroke="${b.border}" stroke-width="1"/></svg>`
  );

  const windowBuf = await sharp({ create: { width: winW, height: winH, channels: 4, background: TRANSPARENT } })
    .composite([
      { input: chromeSvg, top: 0, left: 0 },
      { input: body, top: ch, left: 0 },
      { input: borderSvg, top: 0, left: 0 },
    ])
    .png()
    .toBuffer();

  const out = await placeWithShadow(windowBuf, winW, winH, r);
  await out.png().toFile(destAbsPath);
}

/** Corpo de celular: bezel, cantos bem arredondados, notch e a captura na tela. */
async function makePhoneMockup(srcAbsPath: string, destAbsPath: string): Promise<void> {
  const p = config.mockups.phone;
  const screenW = p.screenWidth;

  const shot = await sharp(srcAbsPath)
    .resize(screenW, null, { withoutEnlargement: false })
    .toBuffer({ resolveWithObject: true });
  const screenH = shot.info.height;

  const bezel = Math.round(screenW * p.bezelRatio);
  const bodyW = screenW + 2 * bezel;
  const bodyH = screenH + 2 * bezel;
  const bodyRadius = Math.round(bodyW * p.bodyRadiusRatio);
  const screenRadius = Math.max(bodyRadius - bezel, 8);

  // Tela com cantos arredondados.
  const screenMask = Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${screenW}" height="${screenH}">` +
      `<rect width="${screenW}" height="${screenH}" rx="${screenRadius}" ry="${screenRadius}" fill="#fff"/></svg>`
  );
  const screen = await sharp(shot.data).composite([{ input: screenMask, blend: "dest-in" }]).png().toBuffer();

  // Corpo do device + borda.
  const bodySvg = Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${bodyW}" height="${bodyH}">` +
      `<rect x="0.5" y="0.5" width="${bodyW - 1}" height="${bodyH - 1}" rx="${bodyRadius}" ry="${bodyRadius}" fill="${p.body}" stroke="${p.border}" stroke-width="1"/></svg>`
  );

  // Notch: pílula preta no topo central, sobre a tela.
  const notchW = Math.round(bodyW * 0.34);
  const notchH = Math.round(bezel * 1.5);
  const notchSvg = Buffer.from(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${bodyW}" height="${bodyH}">` +
      `<rect x="${Math.round((bodyW - notchW) / 2)}" y="${bezel + Math.round(notchH * 0.25)}" width="${notchW}" height="${notchH}" rx="${Math.round(notchH / 2)}" fill="#000"/></svg>`
  );

  const deviceBuf = await sharp({ create: { width: bodyW, height: bodyH, channels: 4, background: TRANSPARENT } })
    .composite([
      { input: bodySvg, top: 0, left: 0 },
      { input: screen, top: bezel, left: bezel },
      { input: notchSvg, top: 0, left: 0 },
    ])
    .png()
    .toBuffer();

  const out = await placeWithShadow(deviceBuf, bodyW, bodyH, bodyRadius);
  await out.png().toFile(destAbsPath);
}
