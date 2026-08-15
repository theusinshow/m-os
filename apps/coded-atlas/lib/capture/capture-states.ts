import path from "node:path";
import { promises as fs } from "node:fs";
import type { Browser } from "playwright";
import { config } from "../config";
import { stateScreenshotDir, publicPath as makePublicPath } from "../storage/paths";
import { slugify } from "../validation/slugify";
import type { ProjectInput, StateCapture, StateInput } from "../types";
import { dismissOverlays } from "./dismiss-overlays";
import { waitForPageStability } from "./wait-for-stability";

/**
 * Captura um estado de interação (v1.5): abre a página principal em desktop,
 * clica o seletor informado e fotografa o viewport. Captura leve — só desktop.
 * Erros propagam; quem chama trata por estado.
 */
export async function captureState(
  browser: Browser,
  input: ProjectInput,
  state: StateInput,
  index: number
): Promise<StateCapture> {
  const dir = stateScreenshotDir(input.slug);
  await fs.mkdir(dir, { recursive: true });

  const vp = config.viewports.desktop;
  const context = await browser.newContext({
    viewport: { width: vp.width, height: vp.height },
    deviceScaleFactor: vp.deviceScaleFactor,
    userAgent: config.userAgent,
  });
  const page = await context.newPage();

  try {
    await page.goto(input.url, { waitUntil: "networkidle", timeout: config.navTimeoutMs });
    await dismissOverlays(page);
    await waitForPageStability(page);
    await page.evaluate(() => window.scrollTo(0, 0));

    // Ação: clicar o seletor (falha se não existir → o estado é descartado).
    await page.click(state.selector, { timeout: 5000 });
    await page.waitForTimeout(config.stateSettleMs);

    const filename = `${slugify(state.name) || "estado"}-${index + 1}.png`;
    await page.screenshot({ path: path.join(dir, filename), type: "png" });

    return {
      name: state.name,
      selector: state.selector,
      screenshot: makePublicPath(input.slug, "screenshots", "states", filename),
    };
  } finally {
    await context.close();
  }
}
