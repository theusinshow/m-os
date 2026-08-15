import path from "node:path";
import { promises as fs } from "node:fs";
import type { Browser, Page } from "playwright";
import { AtlasError } from "../errors";
import { config } from "../config";
import {
  screenshotDir,
  sectionDir,
  videoDir,
  videoFilePath,
  publicPath as makePublicPath,
} from "../storage/paths";
import type {
  CatalogSection,
  DeviceCapture,
  ProjectInput,
  ProgressEvent,
  ViewportConfig,
} from "../types";
import { scrollToBottom } from "./scroll-to-bottom";
import { smoothScrollTo } from "./record-scroll";
import { detectPageSections } from "./detect-sections";
import type { SectionCandidate } from "./detect-sections";
import { dismissOverlays } from "./dismiss-overlays";
import { waitForPageStability } from "./wait-for-stability";
import { inspectSite } from "./inspect-site";
import type { SiteInspection } from "../types";

// ─── Viewport-only result (Fase 3 — mantido para test-phase3) ────────────────
export interface ViewportCaptureResult {
  absPath: string;
  publicPath: string;
}

// ─── Full device result (v0.3) ────────────────────────────────────────────────
export interface DeviceCaptureResult extends DeviceCapture {
  screenshotAbsPath: string;
  fullpageAbsPath: string;
  sections: CatalogSection[];
  videoPublicPath?: string;
  videoAbsPath?: string;
  inspection?: SiteInspection; // apenas no desktop
}

// ─── Error mapping ───────────────────────────────────────────────────────────
function mapPlaywrightError(err: unknown): AtlasError {
  const msg = String(err);
  if (
    msg.includes("ERR_NAME_NOT_RESOLVED") ||
    msg.includes("ERR_CONNECTION_REFUSED") ||
    msg.includes("ERR_ADDRESS_UNREACHABLE")
  ) {
    return new AtlasError(
      "UNREACHABLE",
      "Não foi possível acessar este site. Confira se ele está no ar.",
      msg
    );
  }
  if (msg.includes("TimeoutError") || (msg.includes("Timeout") && msg.includes("exceeded"))) {
    return new AtlasError(
      "NAV_TIMEOUT",
      "O site demorou demais para responder. Tente aumentar o tempo de espera.",
      msg
    );
  }
  return new AtlasError(
    "CAPTURE_FAILED",
    "Falha ao capturar a página. Tente novamente.",
    msg
  );
}

// ─── Scroll-and-shoot (seções) — v0.3: detecção semântica com fallback ────────
async function capturePageSections(
  page: Page,
  input: ProjectInput,
  viewport: ViewportConfig
): Promise<CatalogSection[]> {
  const device = viewport.label as "desktop" | "mobile";
  const sectDir = sectionDir(input.slug, device);
  await fs.mkdir(sectDir, { recursive: true });

  // ── 1. Tenta detecção semântica do DOM ──────────────────────────────────────
  let candidates: SectionCandidate[] = await detectPageSections(
    page,
    config.sectionMinHeight
  ).catch(() => []);

  // ── 2. Fallback: scroll-step fixo se DOM não tem estrutura suficiente ────────
  if (candidates.length < 2) {
    const totalHeight: number = await page.evaluate(() => document.body.scrollHeight);
    const scrollStep = Math.floor(viewport.height * config.sectionScrollRatio);
    let y = 0;
    candidates = [];
    while (y < totalHeight && candidates.length < config.maxSections) {
      candidates.push({ tag: "div", y, elementHeight: viewport.height });
      y += scrollStep;
    }
  }

  // ── 3. Cap de segurança ──────────────────────────────────────────────────────
  candidates = candidates.slice(0, config.maxSections);

  // ── 4. Captura cada seção ───────────────────────────────────────────────────
  const sections: CatalogSection[] = [];

  for (let idx = 0; idx < candidates.length; idx++) {
    const det = candidates[idx];

    await smoothScrollTo(page, det.y);
    await page.waitForTimeout(config.sectionDelayMs);

    const num = String(idx + 1).padStart(3, "0");
    const filename = `section-${num}.png`;
    const absPath = path.join(sectDir, filename);
    await page.screenshot({ path: absPath, type: "png" });

    sections.push({
      device,
      name: `section-${num}`,
      y: det.y,
      height: viewport.height,
      screenshot: makePublicPath(
        input.slug,
        "screenshots",
        `sections-${device}`,
        filename
      ),
      heading: det.heading,
      suggestedName: det.suggestedName,
      semanticTag: det.tag,
      sectionId: det.id,
    });
  }

  return sections;
}

// ─── captureViewport (Fase 3 — context isolado, mantido para test-phase3) ─────
export async function captureViewport(
  browser: Browser,
  input: ProjectInput,
  viewport: ViewportConfig
): Promise<ViewportCaptureResult> {
  const context = await browser.newContext({
    viewport: { width: viewport.width, height: viewport.height },
    deviceScaleFactor: viewport.deviceScaleFactor,
    userAgent: config.userAgent,
  });
  const page = await context.newPage();

  try {
    let response: Awaited<ReturnType<typeof page.goto>>;
    try {
      response = await page.goto(input.url, {
        waitUntil: "networkidle",
        timeout: config.navTimeoutMs,
      });
    } catch (err) {
      throw mapPlaywrightError(err);
    }

    if (response && response.status() === 403) {
      throw new AtlasError(
        "BLOCKED",
        "Este site bloqueou a captura automática. Tente em modo local.",
        `HTTP 403 from ${input.url}`
      );
    }

    await page.waitForTimeout(config.captureDelayMs);

    const filename = `${viewport.label}-${viewport.width}x${viewport.height}.png`;
    const absPath = path.join(screenshotDir(input.slug), filename);

    try {
      await page.screenshot({ path: absPath, type: "png" });
    } catch (err) {
      throw new AtlasError(
        "CAPTURE_FAILED",
        "Falha ao capturar a página. Tente novamente.",
        String(err)
      );
    }

    return { absPath, publicPath: makePublicPath(input.slug, "screenshots", filename) };
  } finally {
    await context.close();
  }
}

// ─── captureDevice (v0.2 — viewport + sections + fullpage + vídeo) ─────────────
export async function captureDevice(
  browser: Browser,
  input: ProjectInput,
  viewport: ViewportConfig,
  onProgress: (event: ProgressEvent) => void
): Promise<DeviceCaptureResult> {
  const isDesktop = viewport.label === "desktop";
  // Opções por geração sobrescrevem os defaults do config (ausência = default).
  const doSections = input.options?.sections ?? config.captureSections;
  const doVideo = input.options?.video ?? config.captureVideo;

  // Prepara dir temporário para o vídeo (Playwright grava com nome aleatório)
  const vidTempDir = doVideo
    ? path.join(videoDir(input.slug), ".tmp")
    : undefined;

  if (vidTempDir) await fs.mkdir(vidTempDir, { recursive: true });

  const context = await browser.newContext({
    viewport: { width: viewport.width, height: viewport.height },
    deviceScaleFactor: viewport.deviceScaleFactor,
    userAgent: config.userAgent,
    ...(vidTempDir
      ? {
          recordVideo: {
            dir: vidTempDir,
            size: { width: viewport.width, height: viewport.height },
          },
        }
      : {}),
  });

  const page = await context.newPage();

  // Resultado parcial construído no try — só definido se não houver erro
  type Partial = Omit<DeviceCaptureResult, "sections" | "videoPublicPath" | "videoAbsPath" | "inspection">;
  let partial: Partial | undefined;
  let sections: CatalogSection[] = [];
  let inspection: SiteInspection | undefined;

  try {
    // ── Navegação ──────────────────────────────────────────────────────────────
    let response: Awaited<ReturnType<typeof page.goto>>;
    try {
      response = await page.goto(input.url, {
        waitUntil: "networkidle",
        timeout: config.navTimeoutMs,
      });
    } catch (err) {
      throw mapPlaywrightError(err);
    }

    if (response && response.status() === 403) {
      throw new AtlasError(
        "BLOCKED",
        "Este site bloqueou a captura automática. Tente em modo local.",
        `HTTP 403 from ${input.url}`
      );
    }

    // ── Overlays: cookie banners, chat widgets, popups ────────────────────────
    await dismissOverlays(page);

    // ── Wait inteligente: fontes + imagens acima do fold + animações ──────────
    await waitForPageStability(page);
    await page.evaluate(() => window.scrollTo(0, 0));

    // ── Inspeção do site (só no desktop — dados são por site, não por device) ──
    if (isDesktop) {
      inspection = await inspectSite(page);
    }

    const shotDir = screenshotDir(input.slug);

    // ── Viewport screenshot ────────────────────────────────────────────────────
    onProgress({
      step: isDesktop ? "capturing-desktop" : "capturing-mobile",
      message: `Capturando ${viewport.label} (${viewport.width}×${viewport.height})...`,
      progress: isDesktop ? 15 : 48,
    });

    const shotFilename = `${viewport.label}-${viewport.width}x${viewport.height}.png`;
    const screenshotAbsPath = path.join(shotDir, shotFilename);

    try {
      await page.screenshot({ path: screenshotAbsPath, type: "png" });
    } catch (err) {
      throw new AtlasError("CAPTURE_FAILED", "Falha ao capturar a página. Tente novamente.", String(err));
    }

    // ── Seções (scroll-and-shoot) ou scrollToBottom para lazy-load ─────────────
    if (doSections) {
      onProgress({
        step: isDesktop ? "capturing-sections-desktop" : "capturing-sections-mobile",
        message: `Fotografando seções ${viewport.label}...`,
        progress: isDesktop ? 22 : 55,
      });
      sections = await capturePageSections(page, input, viewport);
    } else {
      // Sem seções: scroll-to-bottom garante carregamento lazy para o fullpage
      await scrollToBottom(page);
    }

    // ── Fullpage screenshot ────────────────────────────────────────────────────
    onProgress({
      step: isDesktop ? "capturing-fullpage-desktop" : "capturing-fullpage-mobile",
      message: `Gerando full page ${viewport.label}...`,
      progress: isDesktop ? 30 : 62,
    });

    const fpFilename = `${viewport.label}-fullpage.png`;
    const fullpageAbsPath = path.join(shotDir, fpFilename);

    try {
      await page.screenshot({ path: fullpageAbsPath, fullPage: true, type: "png" });
    } catch (err) {
      throw new AtlasError("CAPTURE_FAILED", "Falha ao capturar a página. Tente novamente.", String(err));
    }

    // ── Sinaliza gravação de vídeo antes de fechar o context ──────────────────
    if (doVideo) {
      onProgress({
        step: isDesktop ? "recording-video-desktop" : "recording-video-mobile",
        message: `Salvando vídeo ${viewport.label}...`,
        progress: isDesktop ? 38 : 68,
      });
    }

    partial = {
      viewport: `${viewport.width}x${viewport.height}`,
      screenshot: makePublicPath(input.slug, "screenshots", shotFilename),
      fullpage: makePublicPath(input.slug, "screenshots", fpFilename),
      screenshotAbsPath,
      fullpageAbsPath,
    };
  } finally {
    await context.close(); // vídeo é finalizado aqui pelo Playwright
  }

  // ── Move o arquivo de vídeo após o context fechar ──────────────────────────
  // (este bloco só executa se não houve exceção — erros relançam antes de chegar aqui)
  let videoPublicPath: string | undefined;
  let videoAbsPath: string | undefined;

  if (doVideo) {
    try {
      const tmpPath = await page.video()?.path();
      if (tmpPath) {
        const destDir = videoDir(input.slug);
        await fs.mkdir(destDir, { recursive: true });
        const destPath = videoFilePath(input.slug, viewport.label);
        await fs.rename(tmpPath, destPath);
        videoPublicPath = makePublicPath(input.slug, "videos", `${viewport.label}-scroll.webm`);
        videoAbsPath = destPath;
      }
    } catch (err) {
      console.warn(`[atlas:${input.slug}] Não foi possível salvar vídeo: ${err}`);
    }
  }

  return { ...partial!, sections, videoPublicPath, videoAbsPath, inspection };
}
