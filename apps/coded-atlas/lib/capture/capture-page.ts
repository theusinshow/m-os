import path from "node:path";
import { promises as fs } from "node:fs";
import type { Browser } from "playwright";
import { config } from "../config";
import { pageScreenshotDir, publicPath as makePublicPath } from "../storage/paths";
import { slugify } from "../validation/slugify";
import type { DeviceCapture, PageCapture, ProjectInput, ViewportConfig } from "../types";
import { dismissOverlays } from "./dismiss-overlays";
import { waitForPageStability } from "./wait-for-stability";
import { scrollToBottom } from "./scroll-to-bottom";

/** Resolve uma entrada (path "/sobre" ou URL completa) para URL absoluta. */
export function resolvePageUrl(entry: string, baseUrl: string): string {
  return new URL(entry.trim(), baseUrl).href;
}

/** Slug de pasta para uma página, derivado do seu path. */
function pageSlug(entry: string, baseUrl: string, index: number): string {
  try {
    const u = new URL(entry.trim(), baseUrl);
    const p = u.pathname.replace(/^\/+|\/+$/g, "");
    const s = slugify(p);
    return s || `pagina-${index + 1}`;
  } catch {
    return slugify(entry) || `pagina-${index + 1}`;
  }
}

async function captureOneDevice(
  browser: Browser,
  slug: string,
  pageSlugVal: string,
  fullUrl: string,
  dir: string,
  viewport: ViewportConfig
): Promise<DeviceCapture> {
  const context = await browser.newContext({
    viewport: { width: viewport.width, height: viewport.height },
    deviceScaleFactor: viewport.deviceScaleFactor,
    userAgent: config.userAgent,
  });
  const page = await context.newPage();
  try {
    await page.goto(fullUrl, { waitUntil: "networkidle", timeout: config.navTimeoutMs });
    await dismissOverlays(page);
    await waitForPageStability(page);
    await page.evaluate(() => window.scrollTo(0, 0));

    const vpFile = `${viewport.label}-viewport.png`;
    const fpFile = `${viewport.label}-fullpage.png`;

    await page.screenshot({ path: path.join(dir, vpFile), type: "png" });
    await scrollToBottom(page);
    await page.screenshot({ path: path.join(dir, fpFile), fullPage: true, type: "png" });

    return {
      viewport: `${viewport.width}x${viewport.height}`,
      screenshot: makePublicPath(slug, "screenshots", "pages", pageSlugVal, vpFile),
      fullpage: makePublicPath(slug, "screenshots", "pages", pageSlugVal, fpFile),
    };
  } finally {
    await context.close();
  }
}

/**
 * Captura uma página extra do mesmo site (v1.5): viewport + full page em
 * desktop e mobile. Captura leve — sem seções, vídeo ou inspeção (isso fica
 * só na página principal). Erros propagam; quem chama trata por página.
 */
export async function captureExtraPage(
  browser: Browser,
  input: ProjectInput,
  entry: string,
  index: number
): Promise<PageCapture> {
  const pageSlugVal = pageSlug(entry, input.url, index);
  const fullUrl = resolvePageUrl(entry, input.url);
  const dir = pageScreenshotDir(input.slug, pageSlugVal);
  await fs.mkdir(dir, { recursive: true });

  const desktop = await captureOneDevice(browser, input.slug, pageSlugVal, fullUrl, dir, config.viewports.desktop);
  const mobile = await captureOneDevice(browser, input.slug, pageSlugVal, fullUrl, dir, config.viewports.mobile);

  return { path: entry.trim(), url: fullUrl, desktop, mobile };
}
