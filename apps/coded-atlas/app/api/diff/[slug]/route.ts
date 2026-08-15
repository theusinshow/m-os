export const runtime = "nodejs";
export const maxDuration = 120;

import { NextRequest } from "next/server";
import { promises as fs } from "node:fs";
import path from "node:path";
import { chromium } from "playwright";
import type { Browser } from "playwright";
import { config } from "@/lib/config";
import { catalogPath, screenshotDir, diffDir, publicPath as makePublicPath } from "@/lib/storage/paths";
import { dismissOverlays } from "@/lib/capture/dismiss-overlays";
import { waitForPageStability } from "@/lib/capture/wait-for-stability";
import { computeVisualDiff } from "@/lib/diff/visual-diff";
import type { Catalog } from "@/lib/types";

interface Params {
  params: Promise<{ slug: string }>;
}

/**
 * Diff visual (v1.7): recaptura a página principal do projeto e compara com o
 * screenshot desktop guardado no catálogo, destacando o que mudou. Útil para
 * monitorar sites entregues. Compara o viewport desktop (dimensões fixas).
 */
export async function POST(_req: NextRequest, { params }: Params): Promise<Response> {
  const { slug } = await params;

  let catalog: Catalog;
  try {
    catalog = JSON.parse(await fs.readFile(catalogPath(slug), "utf-8")) as Catalog;
  } catch {
    return Response.json({ error: "Projeto não encontrado." }, { status: 404 });
  }

  const vp = config.viewports.desktop;
  const beforeAbs = path.join(screenshotDir(slug), `${vp.label}-${vp.width}x${vp.height}.png`);
  try {
    await fs.access(beforeAbs);
  } catch {
    return Response.json({ error: "Captura base não encontrada. Gere o catálogo de novo." }, { status: 400 });
  }

  const dir = diffDir(slug);
  await fs.mkdir(dir, { recursive: true });
  const afterAbs = path.join(dir, "after.png");
  const diffAbs = path.join(dir, "diff.png");

  let browser: Browser | undefined;
  try {
    browser = await chromium.launch({
      headless: config.headless,
      args: ["--no-sandbox", "--disable-dev-shm-usage"],
    });
    const context = await browser.newContext({
      viewport: { width: vp.width, height: vp.height },
      deviceScaleFactor: vp.deviceScaleFactor,
      userAgent: config.userAgent,
    });
    const page = await context.newPage();
    await page.goto(catalog.project.url, { waitUntil: "networkidle", timeout: config.navTimeoutMs });
    await dismissOverlays(page);
    await waitForPageStability(page);
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({ path: afterAbs, type: "png" });
  } catch (err) {
    return Response.json(
      { error: "Não foi possível recapturar o site. Confira se ele está no ar." , detail: String(err) },
      { status: 400 }
    );
  } finally {
    await browser?.close();
  }

  let diff;
  try {
    diff = await computeVisualDiff(beforeAbs, afterAbs, diffAbs);
  } catch (err) {
    return Response.json({ error: "Falha ao comparar as imagens.", detail: String(err) }, { status: 500 });
  }

  const result = {
    percent: Number(diff.percent.toFixed(2)),
    changedPixels: diff.changedPixels,
    totalPixels: diff.totalPixels,
    url: catalog.project.url,
    before: catalog.captures.desktop.screenshot,
    after: makePublicPath(slug, "diffs", "after.png"),
    diff: makePublicPath(slug, "diffs", "diff.png"),
    capturedAt: new Date().toISOString(),
  };

  await fs.writeFile(path.join(dir, "result.json"), JSON.stringify(result, null, 2), "utf-8");

  return Response.json(result, { headers: { "Cache-Control": "no-store" } });
}
