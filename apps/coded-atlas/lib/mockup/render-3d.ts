import path from "node:path";
import { promises as fs } from "node:fs";
import type { Browser } from "playwright";
import { config } from "../config";
import { mockupDir, publicPath as makePublicPath } from "../storage/paths";
import type { MockupResult } from "../capture/generate-mockups";

/**
 * Mockups 3D em perspectiva (v1.4 item 12): a captura num device inclinado,
 * renderizada via HTML + CSS 3D e fotografada pelo próprio Playwright com
 * fundo transparente. Mesma infra de render headless da captura — sem engine 3D.
 * Falha por mockup é silenciosa (enhancement).
 */
export async function generateMockups3D(
  browser: Browser,
  slug: string,
  desktopAbs: string,
  mobileAbs: string
): Promise<MockupResult[]> {
  const dir = mockupDir(slug);
  await fs.mkdir(dir, { recursive: true });

  const jobs = [
    { name: "desktop-3d", label: "Desktop 3D", build: () => desktopHtml(desktopAbs), scene: config.mockups3d.desktop },
    { name: "mobile-3d", label: "Mobile 3D", build: () => mobileHtml(mobileAbs), scene: config.mockups3d.mobile },
  ];

  const results: MockupResult[] = [];

  for (const job of jobs) {
    try {
      const html = await job.build();
      const context = await browser.newContext({
        viewport: { width: job.scene.sceneW, height: job.scene.sceneH },
        deviceScaleFactor: config.mockups3d.deviceScaleFactor,
      });
      const page = await context.newPage();
      await page.setContent(html, { waitUntil: "load" });
      await page.waitForTimeout(250); // deixa fontes/transform assentarem
      await page.screenshot({ path: path.join(dir, `${job.name}.png`), omitBackground: true });
      await context.close();

      results.push({ name: job.name, label: job.label, image: makePublicPath(slug, "mockups", `${job.name}.png`) });
    } catch (err) {
      console.warn(`[atlas:${slug}] mockup 3D "${job.name}" falhou: ${err}`);
    }
  }

  return results;
}

async function dataUrl(absPath: string): Promise<string> {
  const buf = await fs.readFile(absPath);
  return `data:image/png;base64,${buf.toString("base64")}`;
}

const SHADOW = "0 60px 120px rgba(0,0,0,.45), 0 24px 48px rgba(0,0,0,.35)";

async function desktopHtml(absPath: string): Promise<string> {
  const src = await dataUrl(absPath);
  const d = config.mockups3d.desktop;
  return `<!doctype html><html><head><meta charset="utf-8"><style>
    html,body{margin:0;padding:0;background:transparent;height:100%}
    .scene{width:100%;height:100%;display:flex;align-items:center;justify-content:center}
    .window{width:${d.windowW}px;border-radius:16px;overflow:hidden;border:1px solid #2a2d34;
      box-shadow:${SHADOW};
      transform:perspective(${d.perspective}px) rotateY(${d.rotateY}deg) rotateX(${d.rotateX}deg);
      transform-origin:center}
    .chrome{height:44px;background:#15171c;display:flex;align-items:center;gap:9px;padding:0 18px}
    .dot{width:11px;height:11px;border-radius:50%}
    img{display:block;width:100%}
  </style></head><body><div class="scene"><div class="window">
    <div class="chrome">
      <span class="dot" style="background:#ff5f57"></span>
      <span class="dot" style="background:#febc2e"></span>
      <span class="dot" style="background:#28c840"></span>
    </div>
    <img src="${src}"/>
  </div></div></body></html>`;
}

async function mobileHtml(absPath: string): Promise<string> {
  const src = await dataUrl(absPath);
  const m = config.mockups3d.mobile;
  return `<!doctype html><html><head><meta charset="utf-8"><style>
    html,body{margin:0;padding:0;background:transparent;height:100%}
    .scene{width:100%;height:100%;display:flex;align-items:center;justify-content:center}
    .phone{width:${m.phoneW}px;background:#0a0a0c;border-radius:54px;padding:14px;border:1px solid #2a2d34;
      box-shadow:${SHADOW};position:relative;
      transform:perspective(${m.perspective}px) rotateY(${m.rotateY}deg) rotateX(${m.rotateX}deg);
      transform-origin:center}
    .screen{border-radius:40px;overflow:hidden}
    .notch{position:absolute;top:24px;left:50%;transform:translateX(-50%);width:124px;height:26px;background:#000;border-radius:14px;z-index:2}
    img{display:block;width:100%}
  </style></head><body><div class="scene"><div class="phone">
    <div class="notch"></div>
    <div class="screen"><img src="${src}"/></div>
  </div></div></body></html>`;
}
